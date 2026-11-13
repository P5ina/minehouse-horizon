//! The Plasmo Voice UDP connection (`NettyUdpClient` and its handler): ping
//! the server with our secret until it answers, answer its keep-alives, play
//! the voices it relays, and send what the player says, encrypted with the
//! AES key the server sealed for us. What has to go over the game connection
//! (audio ends, source info requests) is queued in `Shared::outbox` for the
//! game thread.
use crate::audio::MicQueue;
use crate::mix::{Frame, Listener, Mixer, spatial};
use crate::opus::{Decoder, Encoder, FRAME};
use crate::pv::{self, Source, SourceKind, Udp, Uuid};
use crate::pvcrypt::Aes;
use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Where a server address leads (its SRV record): the game's own resolver.
pub type Redirect = fn(&str, u16) -> (String, u16);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    PushToTalk,
    Voice,
}

/// What the player chose, read by the threads.
#[derive(Clone, Debug)]
pub struct Controls {
    pub mode: Mode,
    pub ptt: bool,
    /// Unused by Plasmo (it has no whisper); kept for the shared menu.
    pub whisper: bool,
    pub muted: bool,
    pub disabled: bool,
    pub threshold_db: f32,
    pub mic_gain: f32,
    pub volume: f32,
    pub player_volumes: HashMap<u128, f32>,
    /// Source lines' volumes by line id (`SourceLineVolumes`).
    pub line_volumes: HashMap<Uuid, f32>,
    /// Pan voices by where they come from (Advanced > Panning).
    pub panning: bool,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            mode: Mode::PushToTalk,
            ptt: false,
            whisper: false,
            muted: false,
            disabled: false,
            threshold_db: -50.0,
            mic_gain: 1.0,
            volume: 1.0,
            player_volumes: HashMap::new(),
            line_volumes: HashMap::new(),
            panning: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Status {
    #[default]
    Idle,
    Connecting,
    Connected,
    Failed(String),
}

/// The proximity activation the server gave us, and how far we talk.
#[derive(Clone, Debug, PartialEq)]
pub struct Talk {
    pub activation: Uuid,
    /// The activation's HUD icon (`plasmovoice:textures/icons/microphone.png`).
    pub icon: String,
    pub distance: i16,
    pub stereo: bool,
    pub opus_application: i32,
    pub bitrate: i32,
    pub mtu: usize,
}

/// What the game thread and the voice threads share.
#[derive(Default)]
pub struct Shared {
    pub listener: Mutex<Listener>,
    /// Players' eye positions by UUID, this tick.
    pub players: Mutex<HashMap<u128, [f64; 3]>>,
    /// Entities' positions by network id, this tick (entity sources).
    pub entities: Mutex<HashMap<i32, [f64; 3]>>,
    /// Who was heard last and when.
    pub talking: Mutex<HashMap<u128, (Instant, bool)>>,
    /// The source line each speaker was last heard on, for its icon.
    pub talking_line: Mutex<HashMap<u128, Uuid>>,
    /// The UDP connection missed keep-alives for a while (Plasmo's soft timeout).
    pub timed_out: AtomicBool,
    pub controls: Mutex<Controls>,
    pub status: Mutex<Status>,
    pub mic_error: Mutex<Option<String>>,
    /// The microphone's last peak, dB as f32 bits.
    pub mic_level: AtomicU32,
    pub transmitting: AtomicBool,
    /// Where the voice goes, for the status line.
    pub voice_target: Mutex<String>,
    pub tries: AtomicU32,
    pub received: AtomicU32,
    pub unreadable: AtomicU32,
    /// Set by the config packet: the voice key and how to talk.
    pub aes: Mutex<Option<Arc<Aes>>>,
    pub talk: Mutex<Option<Talk>>,
    /// Sources the server told us about (`SourceInfoPacket`).
    pub sources: Mutex<HashMap<Uuid, Source>>,
    /// `plasmo:voice/v2` payloads for the game thread to send.
    pub outbox: Mutex<Vec<Vec<u8>>>,
    pub udp_connected: AtomicBool,
}

impl Shared {
    pub fn level(&self) -> f32 {
        f32::from_bits(self.mic_level.load(Ordering::Relaxed))
    }
    pub fn set_status(&self, s: Status) {
        *self.status.lock().unwrap() = s;
    }
    pub fn send_tcp(&self, payload: Vec<u8>) {
        let mut o = self.outbox.lock().unwrap();
        if o.len() < 256 {
            o.push(payload);
        }
    }
}

pub struct Conn {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

pub struct Target {
    pub secret: Uuid,
    /// From the connection packet; "0.0.0.0" means the game server's host.
    pub ip: String,
    pub port: u16,
    /// The game server the player joined, as typed.
    pub server: (String, u16),
}

impl Conn {
    pub fn start(
        target: Target,
        redirect: Redirect,
        shared: Arc<Shared>,
        mixer: Arc<Mutex<Mixer>>,
        mic: Arc<MicQueue>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        shared.set_status(Status::Connecting);
        let thread = std::thread::Builder::new()
            .name("plasmovoice-net".into())
            .spawn(move || {
                let result = run(target, redirect, &shared, &mixer, mic, &flag);
                mixer.lock().unwrap().clear();
                shared.transmitting.store(false, Ordering::Relaxed);
                shared.udp_connected.store(false, Ordering::Relaxed);
                shared.set_status(match result {
                    Ok(()) => Status::Idle,
                    Err(e) => Status::Failed(e),
                });
            })
            .ok();
        Self { stop, thread }
    }
}

impl Drop for Conn {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn resolve(host: &str, port: u16) -> Result<SocketAddr, String> {
    let addrs: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|e| format!("unknown voice host {host}: {e}"))?
        .collect();
    addrs
        .iter()
        .find(|a| a.is_ipv4())
        .or(addrs.first())
        .copied()
        .ok_or_else(|| format!("unknown voice host {host}"))
}

/// Plasmo's client times out after 15 seconds without a keep-alive.
const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(15);

fn run(
    target: Target,
    redirect: Redirect,
    shared: &Arc<Shared>,
    mixer: &Arc<Mutex<Mixer>>,
    mic: Arc<MicQueue>,
    stop: &Arc<AtomicBool>,
) -> Result<(), String> {
    let host = if target.ip.is_empty() || target.ip == "0.0.0.0" {
        // The game connected to the SRV target, if any: so does the voice.
        redirect(&target.server.0, target.server.1).0
    } else {
        target.ip.clone()
    };
    let addr = resolve(&host, target.port)?;
    *shared.voice_target.lock().unwrap() = addr.to_string();
    for count in [&shared.tries, &shared.received, &shared.unreadable] {
        count.store(0, Ordering::Relaxed);
    }
    let socket = UdpSocket::bind(if addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" })
        .map_err(|e| format!("no UDP socket: {e}"))?;
    socket.connect(addr).map_err(|e| format!("voice server {addr}: {e}"))?;
    socket.set_read_timeout(Some(Duration::from_millis(50))).map_err(|e| e.to_string())?;
    let secret = target.secret;

    let quit = Arc::new(AtomicBool::new(false));
    let mic_thread = {
        let (socket, shared, quit) = (socket.try_clone().map_err(|e| e.to_string())?, shared.clone(), quit.clone());
        std::thread::Builder::new()
            .name("plasmovoice-mic".into())
            .spawn(move || mic_loop(&socket, secret, &shared, &mic, &quit))
            .ok()
    };

    let mut last_try: Option<Instant> = None;
    let mut keep_alive = Instant::now();
    let mut channels: HashMap<Uuid, Channel> = HashMap::new();
    let mut requested: HashMap<Uuid, Instant> = HashMap::new();
    let mut buf = vec![0u8; pv::MAX_UDP];
    let result = loop {
        if stop.load(Ordering::Relaxed) {
            break Ok(());
        }
        let now = Instant::now();
        let connected = shared.udp_connected.load(Ordering::Relaxed);
        if !connected && last_try.is_none_or(|t| now - t >= Duration::from_secs(1)) {
            last_try = Some(now);
            shared.tries.fetch_add(1, Ordering::Relaxed);
            let _ = socket.send(&pv::udp_ping(secret, now_ms(), Some((&host, target.port))));
        }
        shared.timed_out.store(connected && now - keep_alive > Duration::from_secs(7), Ordering::Relaxed);
        if now - keep_alive > KEEP_ALIVE_TIMEOUT {
            break Err(if connected {
                "the voice server stopped answering".into()
            } else {
                format!("no answer from the voice server at {addr}")
            });
        }
        let n = match socket.recv(&mut buf) {
            Ok(n) => n,
            Err(_) => continue,
        };
        shared.received.fetch_add(1, Ordering::Relaxed);
        let Ok(packet) = pv::decode_udp(&buf[..n]) else {
            shared.unreadable.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        match packet {
            Udp::Ping => {
                keep_alive = now;
                if !shared.udp_connected.swap(true, Ordering::Relaxed) {
                    shared.set_status(Status::Connected);
                }
                let _ = socket.send(&pv::udp_ping(secret, now_ms(), None));
            }
            Udp::SourceAudio { sequence, data, source, state, distance } => {
                if shared.controls.lock().unwrap().disabled {
                    continue;
                }
                let info = shared.sources.lock().unwrap().get(&source).cloned();
                match info {
                    Some(info) if info.state == state => {
                        play(&info, sequence, &data, distance, shared, mixer, &mut channels);
                    }
                    _ => {
                        // Unknown or changed: ask, at most once a second.
                        if requested.get(&source).is_none_or(|t| now - *t > Duration::from_secs(1)) {
                            requested.insert(source, now);
                            shared.send_tcp(pv::source_info_request(source));
                        }
                    }
                }
            }
            Udp::SelfAudioInfo { .. } | Udp::Other(_) => {}
        }
        channels.retain(|_, c| now - c.used < Duration::from_secs(30));
    };
    shared.udp_connected.store(false, Ordering::Relaxed);
    quit.store(true, Ordering::Relaxed);
    if let Some(t) = mic_thread {
        let _ = t.join();
    }
    result
}

struct Channel {
    decoder: Decoder,
    last: i64,
    used: Instant,
}

/// The source ended (`SourceAudioEndPacket`): drop what's queued for it.
pub fn end_source(source: Uuid, shared: &Shared, mixer: &Mutex<Mixer>) {
    mixer.lock().unwrap().end(source);
    if let Some(s) = shared.sources.lock().unwrap().get(&source)
        && let SourceKind::Player(p) = &s.kind
    {
        shared.talking.lock().unwrap().remove(&p.id);
    }
}

/// Decode in order, conceal what was lost, and place the voice where its
/// source is (`ClientAudioSource.process`).
fn play(
    info: &Source,
    sequence: i64,
    sealed: &[u8],
    distance: i16,
    shared: &Shared,
    mixer: &Mutex<Mixer>,
    channels: &mut HashMap<Uuid, Channel>,
) {
    let now = Instant::now();
    let Some(aes) = shared.aes.lock().unwrap().clone() else { return };
    let Some(data) = aes.decrypt(sealed) else {
        shared.unreadable.fetch_add(1, Ordering::Relaxed);
        return;
    };
    let ch = match channels.entry(info.id) {
        std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
        std::collections::hash_map::Entry::Vacant(e) => match Decoder::new() {
            Ok(decoder) => e.insert(Channel { decoder, last: -1, used: now }),
            Err(_) => return,
        },
    };
    ch.used = now;
    if ch.last >= 0 && sequence <= ch.last {
        if ch.last - sequence > 100 {
            // A new phrase restarted the numbering.
            ch.last = -1;
            ch.decoder.reset();
        } else {
            return;
        }
    }
    let mut frames = Vec::new();
    let mut lost = if ch.last >= 0 { sequence - ch.last - 1 } else { 0 };
    if lost > 5 {
        ch.decoder.reset();
        lost = 0;
    }
    for i in 0..lost {
        frames.push(if i == lost - 1 { ch.decoder.decode(Some(&data), true) } else { ch.decoder.decode(None, false) });
    }
    frames.push(ch.decoder.decode(Some(&data), false));
    ch.last = sequence;

    let speaker = match &info.kind {
        SourceKind::Player(p) => Some(p.id),
        SourceKind::Direct { sender, .. } => sender.as_ref().map(|s| s.id),
        _ => None,
    };
    let (volume, listener, panning) = {
        let c = shared.controls.lock().unwrap();
        let v = c.volume
            * speaker.and_then(|s| c.player_volumes.get(&s).copied()).unwrap_or(1.0)
            * c.line_volumes.get(&info.line).copied().unwrap_or(1.0);
        (v, *shared.listener.lock().unwrap(), c.panning)
    };
    let max = f32::from(distance.max(1));
    let gain = match &info.kind {
        SourceKind::Player(p) => shared.players.lock().unwrap().get(&p.id).copied().and_then(|at| spatial(&listener, at, max)),
        SourceKind::Entity(id) => shared.entities.lock().unwrap().get(id).copied().and_then(|at| spatial(&listener, at, max)),
        SourceKind::Static(at) => spatial(&listener, *at, max),
        SourceKind::Direct { .. } => Some([1.0, 1.0]),
    };
    let Some(gain) = gain else { return };
    // Without panning both ears hear the louder side.
    let gain = if panning { gain } else { [gain[0].max(gain[1]); 2] };
    let gain = gain.map(|g| g * volume);
    let mut m = mixer.lock().unwrap();
    for f in frames {
        m.push(info.id, Frame { pcm: f.iter().map(|&v| f32::from(v) / 32768.0).collect(), gain });
    }
    drop(m);
    if let Some(s) = speaker
        && gain.iter().any(|g| *g > 0.0)
    {
        shared.talking.lock().unwrap().insert(s, (now, false));
        shared.talking_line.lock().unwrap().insert(s, info.line);
    }
}

/// The mic thread: each 20 ms frame, sent while push to talk is held or the
/// voice is loud enough, and a little after; then the end over the game
/// connection, as Plasmo's `VoiceAudioCapture` does.
fn mic_loop(socket: &UdpSocket, secret: Uuid, shared: &Shared, mic: &MicQueue, stop: &AtomicBool) {
    let mut encoder: Option<(Encoder, Talk)> = None;
    let mut sequence: i64 = 0;
    let mut sending = false;
    let mut hold = 0u32;
    while !stop.load(Ordering::Relaxed) {
        let frame: Vec<f32> = {
            let mut q = mic.samples.lock().unwrap();
            if q.len() < FRAME {
                q = mic.ready.wait_timeout(q, Duration::from_millis(100)).unwrap().0;
            }
            if q.len() < FRAME {
                continue;
            }
            q.drain(..FRAME).collect()
        };
        let c = shared.controls.lock().unwrap().clone();
        let mut pcm = [0i16; FRAME];
        let mut peak = 0f32;
        for (o, s) in pcm.iter_mut().zip(&frame) {
            let v = (s * c.mic_gain).clamp(-1.0, 1.0);
            peak = peak.max(v.abs());
            *o = (v * 32767.0) as i16;
        }
        let db = if peak > 0.0 { (20.0 * peak.log10()).max(-127.0) } else { -127.0 };
        shared.mic_level.store(db.to_bits(), Ordering::Relaxed);

        let talk_cfg = shared.talk.lock().unwrap().clone();
        let aes = shared.aes.lock().unwrap().clone();
        let (Some(cfg), Some(aes)) = (talk_cfg, aes) else {
            shared.transmitting.store(false, Ordering::Relaxed);
            continue;
        };
        if encoder.as_ref().is_none_or(|(_, t)| *t != cfg) {
            match Encoder::with_bitrate(cfg.opus_application, cfg.mtu, cfg.bitrate) {
                Ok(e) => encoder = Some((e, cfg.clone())),
                Err(e) => {
                    *shared.mic_error.lock().unwrap() = Some(e);
                    continue;
                }
            }
        }
        let Some((enc, _)) = encoder.as_mut() else { continue };

        let can = shared.udp_connected.load(Ordering::Relaxed) && !c.muted && !c.disabled;
        let mut talk = can
            && match c.mode {
                Mode::PushToTalk => c.ptt,
                Mode::Voice => db > c.threshold_db,
            };
        if talk {
            hold = if c.mode == Mode::PushToTalk { 5 } else { 25 };
        } else if can && hold > 0 {
            hold -= 1;
            talk = true;
        }
        if talk {
            if let Ok(data) = enc.encode(&pcm) {
                let sealed = aes.encrypt(&data);
                let packet = pv::udp_player_audio(secret, now_ms(), sequence, &sealed, cfg.activation, cfg.distance, cfg.stereo);
                let _ = socket.send(&packet);
                sequence += 1;
                sending = true;
            }
        } else if sending {
            shared.send_tcp(pv::player_audio_end(sequence, cfg.activation, cfg.distance));
            sequence += 1;
            enc.reset();
            sending = false;
            hold = 0;
        }
        shared.transmitting.store(sending, Ordering::Relaxed);
    }
}
