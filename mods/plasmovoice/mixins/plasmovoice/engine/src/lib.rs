//! Plasmo Voice for Horizon: a client for the Plasmo Voice 2.x protocol (an
//! unofficial one, not affiliated with Plasmo Voice or its authors), so a
//! Horizon player talks with players on the Java mod on a server that runs
//! Plasmo Voice. Built on Horizon Voice Chat by Nirokx (MIT): its audio,
//! mixer, menu and HUD; the protocol and the connection are Plasmo's.
//!
//! The game thread calls [`tick`], [`payload`], [`key`], [`command`] and the
//! HUD and settings functions; the voice runs on threads of its own.
pub mod audio;
pub mod buf;
pub mod labels;
pub mod mix;
pub mod net;
pub mod opus;
pub mod pv;
pub mod pvcrypt;
pub mod ui;
pub mod icons;

use audio::{AudioIo, Devices, MicQueue};
use mix::{Listener, Mixer};
use net::{Conn, Mode, Redirect, Shared, Status, Talk, Target};
use pv::{PlayerInfo, Tcp, Uuid};
use pvcrypt::{Aes, RsaKey};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub const ID: &str = "plasmovoice";
pub const NAME: &str = "Plasmo Voice";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "Talk with players who use the Plasmo Voice mod, on servers that have it (unofficial). \
V opens the menu. Push to talk (Left Alt) or voice activation.";

pub const KEY_PTT: &str = "push_to_talk";
pub const KEY_WHISPER: &str = "whisper";
pub const KEY_MUTE: &str = "mute";
pub const KEY_DISABLE: &str = "disable";
pub const DEFAULT_PTT: &str = "key.keyboard.left.alt";
pub const DEFAULT_WHISPER: &str = "key.keyboard.unknown";
pub const DEFAULT_MUTE: &str = "key.keyboard.m";
pub const DEFAULT_DISABLE: &str = "key.keyboard.unknown";
pub const KEY_MENU: &str = "menu";
pub const DEFAULT_MENU: &str = "key.keyboard.v";

/// The mod's keys: their name, what they do, and their default.
pub const HOTKEYS: [(&str, &str, &str); 4] = [
    (KEY_MENU, "Plasmo Voice settings", DEFAULT_MENU),
    (KEY_PTT, "Proximity: push to talk", DEFAULT_PTT),
    (KEY_MUTE, "Mute microphone", DEFAULT_MUTE),
    (KEY_DISABLE, "Disable voice chat", DEFAULT_DISABLE),
];

/// A player's eyes above their feet (`Player.getEyeHeight`, standing).
pub const EYE_HEIGHT: f64 = 1.62;

/// The game's side of a tick in a server's world.
pub struct World<'a> {
    /// The address the player joined, as typed (`host` or `host:port`).
    pub address: &'a str,
    pub listener: Option<Listener>,
    /// Other players by UUID, at their eyes.
    pub players: Vec<(u128, [f64; 3])>,
    pub redirect: Redirect,
    /// This player's UUID, when the game knows it.
    pub own: Option<u128>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    PushToTalk,
    Whisper,
    Mute,
    Disable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    Auto,
    V26,
    V27,
}

/// The settings page and what is stored of it.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub mode: Mode,
    pub muted: bool,
    pub disabled: bool,
    pub volume: u32,
    pub mic_gain: u32,
    pub threshold: i32,
    pub protocol: Protocol,
    pub hud: bool,
    pub group_labels: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::PushToTalk,
            muted: false,
            disabled: false,
            volume: 100,
            mic_gain: 100,
            threshold: -30,
            protocol: Protocol::Auto,
            hud: true,
            group_labels: true,
        }
    }
}

const VOLUMES: [u32; 9] = [0, 25, 50, 75, 100, 125, 150, 175, 200];
const GAINS: [u32; 8] = [50, 75, 100, 125, 150, 200, 250, 300];
const THRESHOLDS: [i32; 7] = [-70, -60, -50, -45, -40, -30, -20];

fn next<T: Copy + PartialEq>(list: &[T], v: T) -> T {
    let i = list.iter().position(|x| *x == v).map_or(0, |i| (i + 1) % list.len());
    list[i]
}

impl Settings {
    pub fn load(get: &dyn Fn(&str) -> Option<String>) -> Self {
        let d = Self::default();
        let num = |k: &str| get(k).and_then(|v| v.parse::<i64>().ok());
        Self {
            mode: if get("mode").as_deref() == Some("voice") { Mode::Voice } else { Mode::PushToTalk },
            muted: get("muted").as_deref() == Some("true"),
            disabled: get("disabled").as_deref() == Some("true"),
            volume: num("volume").map_or(d.volume, |v| v.clamp(0, 200) as u32),
            mic_gain: num("mic_gain").map_or(d.mic_gain, |v| v.clamp(10, 500) as u32),
            threshold: num("threshold").map_or(d.threshold, |v| v.clamp(-127, 0) as i32),
            protocol: match get("protocol").as_deref() {
                Some("2.6") => Protocol::V26,
                Some("2.7") => Protocol::V27,
                _ => Protocol::Auto,
            },
            hud: get("hud").as_deref() != Some("false"),
            group_labels: get("group_labels").as_deref() != Some("false"),
        }
    }

    pub fn stored(&self) -> Vec<(&'static str, String)> {
        vec![
            ("mode", if self.mode == Mode::Voice { "voice" } else { "ptt" }.into()),
            ("muted", self.muted.to_string()),
            ("disabled", self.disabled.to_string()),
            ("volume", self.volume.to_string()),
            ("mic_gain", self.mic_gain.to_string()),
            ("threshold", self.threshold.to_string()),
            ("protocol", match self.protocol {
                Protocol::Auto => "auto",
                Protocol::V26 => "2.6",
                Protocol::V27 => "2.7",
            }
            .into()),
            ("hud", self.hud.to_string()),
            ("group_labels", self.group_labels.to_string()),
        ]
    }

    /// The settings page's buttons, in order.
    pub fn buttons(&self) -> Vec<String> {
        let on_off = |b: bool| if b { "On" } else { "Off" };
        vec![
            format!("Activation: {}", if self.mode == Mode::Voice { "Voice" } else { "Push to talk" }),
            format!("Microphone: {}", if self.muted { "Muted" } else { "On" }),
            format!("Voice chat: {}", on_off(!self.disabled)),
            format!("Voice volume: {}%", self.volume),
            format!("Microphone gain: {}%", self.mic_gain),
            format!("Voice activation level: {} dB", self.threshold),
            format!("Protocol: {}", match self.protocol {
                Protocol::Auto => "Automatic",
                Protocol::V26 => "2.6.x",
                Protocol::V27 => "2.7",
            }),
            format!("Status on screen: {}", on_off(self.hud)),
            format!("Group names above players: {}", on_off(self.group_labels)),
        ]
    }

    pub fn click(&mut self, button: usize) {
        match button {
            0 => self.mode = if self.mode == Mode::Voice { Mode::PushToTalk } else { Mode::Voice },
            1 => self.muted = !self.muted,
            2 => self.disabled = !self.disabled,
            3 => self.volume = next(&VOLUMES, self.volume),
            4 => self.mic_gain = next(&GAINS, self.mic_gain),
            5 => self.threshold = next(&THRESHOLDS, self.threshold),
            6 => {
                self.protocol = match self.protocol {
                    Protocol::Auto => Protocol::V26,
                    Protocol::V26 => Protocol::V27,
                    Protocol::V27 => Protocol::Auto,
                }
            }
            7 => self.hud = !self.hud,
            8 => self.group_labels = !self.group_labels,
            _ => {}
        }
    }
}

/// One server visit.
struct Session {
    address: String,
    joined: Instant,
    /// When we last told the server who we are (`PlayerInfoPacket`), and how often.
    info_sent: Option<Instant>,
    info_count: u32,
    /// The connection packet: there is Plasmo Voice here.
    connection: Option<(Uuid, String, u16)>,
    config: Option<pv::Config>,
    conn: Option<Conn>,
    audio: Option<AudioIo>,
    players: HashMap<Uuid, PlayerInfo>,
    /// This player's UUID (`SelfSourceInfo`'s player, else the one the game gives).
    own: Option<Uuid>,
}

impl Session {
    fn new(address: &str) -> Self {
        Self {
            address: address.to_owned(),
            joined: Instant::now(),
            info_sent: None,
            info_count: 0,
            connection: None,
            config: None,
            conn: None,
            audio: None,
            players: HashMap::new(),
            own: None,
        }
    }

    fn name(&self, id: &Uuid) -> Option<String> {
        self.players.get(id).map(|p| p.nick.clone())
    }
}

/// The RSA key the server seals its voice key with: one per game, made off
/// the game thread (a 2048-bit key takes a moment).
fn rsa_key() -> Option<Arc<RsaKey>> {
    static KEY: LazyLock<Mutex<Option<Arc<RsaKey>>>> = LazyLock::new(|| Mutex::new(None));
    static STARTED: AtomicBool = AtomicBool::new(false);
    if !STARTED.swap(true, Ordering::Relaxed) {
        let _ = std::thread::Builder::new().name("plasmovoice-rsa".into()).spawn(|| {
            let key = Arc::new(RsaKey::generate(2048));
            *KEY.lock().unwrap() = Some(key);
        });
    }
    KEY.lock().unwrap().clone()
}

struct Voice {
    settings: Settings,
    loaded: bool,
    session: Option<Session>,
    /// Custom payloads for the server, sent at the tick.
    outbox: Vec<(String, Vec<u8>)>,
    /// Settings to write back, and chat lines for the player.
    store: Vec<(String, String)>,
    chat: Vec<String>,
    shared: Arc<Shared>,
    mixer: Arc<Mutex<Mixer>>,
    mic: Arc<MicQueue>,
    devices: Arc<Mutex<Devices>>,
    redirect: Option<Redirect>,
    menu: ui::Menu,
}

fn voice() -> MutexGuard<'static, Voice> {
    static VOICE: LazyLock<Mutex<Voice>> = LazyLock::new(|| {
        Mutex::new(Voice {
            settings: Settings::default(),
            loaded: false,
            session: None,
            outbox: Vec::new(),
            store: Vec::new(),
            chat: Vec::new(),
            shared: Arc::default(),
            mixer: Arc::default(),
            mic: Arc::default(),
            devices: Arc::new(Mutex::new(Devices::default())),
            redirect: None,
            menu: ui::Menu::default(),
        })
    });
    VOICE.lock().unwrap_or_else(|e| e.into_inner())
}

static SOUND_CARD: AtomicBool = AtomicBool::new(true);

/// For tools and tests: leave the sound card alone, and feed [`mic_queue`]
/// and read [`mixer`] instead.
pub fn use_sound_card(on: bool) {
    SOUND_CARD.store(on, Ordering::Relaxed);
}

pub fn mic_queue() -> Arc<MicQueue> {
    voice().mic.clone()
}

pub fn mixer() -> Arc<Mutex<Mixer>> {
    voice().mixer.clone()
}

/// `host[:port]` → host and port (25565 by default).
fn split_host_port(s: &str) -> (String, u16) {
    if let Some(rest) = s.strip_prefix('[')
        && let Some((host, after)) = rest.split_once(']')
    {
        let port = after.strip_prefix(':').and_then(|p| p.parse().ok()).unwrap_or(25565);
        return (host.to_owned(), port);
    }
    match s.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') => (h.to_owned(), p.parse().unwrap_or(25565)),
        _ => (s.to_owned(), 25565),
    }
}

/// The opus application Plasmo's encoder settings name (`VOIP`, `AUDIO`, `RESTRICTED_LOWDELAY`).
fn opus_application(mode: Option<&str>) -> i32 {
    match mode {
        Some("AUDIO") => opus::APPLICATION_AUDIO,
        Some("RESTRICTED_LOWDELAY") => opus::APPLICATION_RESTRICTED_LOWDELAY,
        _ => opus::APPLICATION_VOIP,
    }
}

/// How we talk: the proximity activation (or the lightest one), at its default distance.
fn talk_from(config: &pv::Config) -> Option<Talk> {
    let proximity = pv::activation_id("proximity");
    let a = config
        .activations
        .iter()
        .find(|a| a.id == proximity)
        .or_else(|| config.activations.iter().filter(|a| a.proximity).min_by_key(|a| a.weight))?;
    let codec = a.encoder.as_ref().or(config.encoder.as_ref());
    Some(Talk {
        activation: a.id,
        icon: a.icon.clone(),
        distance: a.default_distance.clamp(1, i32::from(i16::MAX)) as i16,
        stereo: false,
        opus_application: opus_application(codec.and_then(|c| c.param("mode"))),
        bitrate: codec.and_then(|c| c.param("bitrate")).and_then(|b| b.parse().ok()).unwrap_or(-1000),
        mtu: usize::try_from(config.mtu).unwrap_or(1024).clamp(64, 2048),
    })
}

impl Voice {
    fn apply_controls(&self) {
        let s = &self.settings;
        let mut c = self.shared.controls.lock().unwrap();
        c.mode = s.mode;
        c.muted = s.muted;
        c.disabled = s.disabled;
        c.volume = s.volume as f32 / 100.0;
        c.mic_gain = s.mic_gain as f32 / 100.0;
        c.threshold_db = s.threshold as f32;
    }

    fn save(&mut self) {
        self.apply_controls();
        let stored = self.settings.stored();
        self.store.extend(stored.into_iter().map(|(k, v)| (k.to_owned(), v)));
    }

    fn send_pv(&mut self, data: Vec<u8>) {
        self.outbox.push((pv::CHANNEL.to_owned(), data));
    }

    /// Tells the server whether the player muted or turned voice chat off.
    fn send_state(&mut self) {
        if self.session.as_ref().is_none_or(|s| s.connection.is_none()) {
            return;
        }
        let (muted, disabled) = (self.settings.muted, self.settings.disabled);
        self.send_pv(pv::player_state(disabled, muted));
    }

    fn send_info(&mut self) {
        let Some(key) = rsa_key() else { return };
        let (muted, disabled) = (self.settings.muted, self.settings.disabled);
        self.send_pv(pv::player_info(&key.public_der(), disabled, muted));
        if let Some(s) = self.session.as_mut() {
            s.info_sent = Some(Instant::now());
            s.info_count += 1;
        }
    }

    fn leave(&mut self) {
        // The connection's threads stop as it drops.
        self.session = None;
        self.shared.transmitting.store(false, Ordering::Relaxed);
        self.shared.talking.lock().unwrap().clear();
        self.shared.sources.lock().unwrap().clear();
        self.shared.outbox.lock().unwrap().clear();
        *self.shared.aes.lock().unwrap() = None;
        *self.shared.talk.lock().unwrap() = None;
        *self.shared.status.lock().unwrap() = Status::Idle;
        *self.shared.mic_error.lock().unwrap() = None;
        self.mixer.lock().unwrap().clear();
    }

    fn on_connection(&mut self, secret: Uuid, ip: String, port: i32) {
        let Some(redirect) = self.redirect else { return };
        let Some(session) = self.session.as_mut() else { return };
        // A new connection packet replaces the connection.
        session.conn = None;
        let port = u16::try_from(port).unwrap_or(0);
        session.connection = Some((secret, ip.clone(), port));
        let target = Target { secret, ip, port, server: split_host_port(&session.address) };
        session.conn = Some(Conn::start(target, redirect, self.shared.clone(), self.mixer.clone(), self.mic.clone()));
        if session.audio.is_none() && SOUND_CARD.load(Ordering::Relaxed) {
            session.audio = Some(AudioIo::start(self.mixer.clone(), self.mic.clone(), self.devices.clone()));
        }
    }

    fn on_config(&mut self, config: pv::Config) {
        let aes = match &config.encryption {
            Some((algorithm, sealed)) => {
                let opened = rsa_key().and_then(|k| k.decrypt(sealed)).and_then(Aes::new);
                match opened {
                    Some(aes) if algorithm.starts_with("AES") => Some(Arc::new(aes)),
                    _ => {
                        self.chat.push(format!("Voice chat: can't open the server's {algorithm} key"));
                        *self.shared.status.lock().unwrap() = Status::Failed("bad encryption key".into());
                        return;
                    }
                }
            }
            // Plasmo always encrypts; a server without a key can't be heard here.
            None => None,
        };
        *self.shared.aes.lock().unwrap() = aes;
        *self.shared.talk.lock().unwrap() = talk_from(&config);
        if let Some(s) = self.session.as_mut() {
            s.config = Some(config);
        }
        self.send_state();
    }

    fn on_payload(&mut self, data: &[u8]) {
        let Ok(packet) = pv::decode_tcp(data) else { return };
        match packet {
            Tcp::Connection { secret, ip, port } => self.on_connection(secret, ip, port),
            Tcp::PlayerInfoRequest => self.send_info(),
            Tcp::Config(c) => self.on_config(c),
            Tcp::PlayerList(list) => {
                if let Some(s) = self.session.as_mut() {
                    s.players = list.into_iter().map(|p| (p.id, p)).collect();
                }
            }
            Tcp::PlayerInfoUpdate(p) => {
                if let Some(s) = self.session.as_mut() {
                    s.players.insert(p.id, p);
                }
            }
            Tcp::PlayerDisconnect(id) => {
                if let Some(s) = self.session.as_mut() {
                    s.players.remove(&id);
                }
            }
            Tcp::SourceInfo(source) => {
                self.shared.sources.lock().unwrap().insert(source.id, source);
            }
            Tcp::SelfSourceInfo { player, .. } => {
                if let Some(s) = self.session.as_mut() {
                    s.own = Some(player);
                }
            }
            Tcp::SourceAudioEnd { source, .. } => net::end_source(source, &self.shared, &self.mixer),
            Tcp::ActivationRegister(a) if a.id == pv::activation_id("proximity") => {
                if let Some(t) = self.shared.talk.lock().unwrap().as_mut() {
                    t.activation = a.id;
                    t.distance = a.default_distance.clamp(1, i32::from(i16::MAX)) as i16;
                }
            }
            _ => {}
        }
    }

    fn tick(&mut self, world: World, get: &dyn Fn(&str) -> Option<String>) {
        if !self.loaded {
            self.loaded = true;
            self.settings = Settings::load(get);
            self.apply_controls();
        }
        self.redirect = Some(world.redirect);
        if self.session.as_ref().is_some_and(|s| s.address != world.address) {
            self.leave();
        }
        if self.session.is_none() {
            self.session = Some(Session::new(world.address));
            self.outbox.push(("minecraft:register".into(), pv::register_payload()));
        }
        if let Some(l) = world.listener {
            *self.shared.listener.lock().unwrap() = l;
        }
        if world.own.is_some()
            && let Some(s) = self.session.as_mut()
        {
            s.own = world.own;
        }
        *self.shared.players.lock().unwrap() = world.players.into_iter().collect();

        // The server asks who we are when we join, which may be before we
        // registered the channel: tell it anyway, a few times, until it connects us.
        let now = Instant::now();
        let session = self.session.as_ref().expect("made above");
        let due = session.connection.is_none()
            && session.info_count < 4
            && now - session.joined > Duration::from_secs(1)
            && session.info_sent.is_none_or(|t| now - t > Duration::from_secs(5));
        if due {
            self.send_info();
        }
        let queued: Vec<Vec<u8>> = std::mem::take(&mut *self.shared.outbox.lock().unwrap());
        for p in queued {
            self.send_pv(p);
        }
        // The UDP connection failed: ask again, as Plasmo reconnects.
        let failed = matches!(*self.shared.status.lock().unwrap(), Status::Failed(_));
        let session = self.session.as_mut().expect("made above");
        if failed && session.connection.is_some() && session.info_sent.is_none_or(|t| now - t > Duration::from_secs(10)) {
            session.connection = None;
            session.conn = None;
            session.info_count = 0;
            session.info_sent = Some(now);
        }
    }
}

/// A tick in a server's world (`world`), or out of one (None). Returns the
/// custom payloads to send the server and the settings to store.
pub fn tick(world: Option<World>, get: &dyn Fn(&str) -> Option<String>) -> Tick {
    let mut v = voice();
    match world {
        Some(w) => v.tick(w, get),
        None => {
            if v.session.is_some() {
                v.leave();
            }
        }
    }
    Tick {
        payloads: std::mem::take(&mut v.outbox),
        store: std::mem::take(&mut v.store),
        chat: std::mem::take(&mut v.chat),
    }
}

#[derive(Default)]
pub struct Tick {
    pub payloads: Vec<(String, Vec<u8>)>,
    pub store: Vec<(String, String)>,
    pub chat: Vec<String>,
}

/// A custom payload from the server: true when it was Plasmo Voice's.
pub fn payload(channel: &str, data: &[u8]) -> bool {
    if channel != pv::CHANNEL {
        return false;
    }
    voice().on_payload(data);
    true
}

/// A key went down or up. Returns a line for the chat, for toggles.
pub fn key(which: Key, down: bool) -> Option<String> {
    let mut v = voice();
    match which {
        Key::PushToTalk => v.shared.controls.lock().unwrap().ptt = down,
        Key::Whisper => v.shared.controls.lock().unwrap().whisper = down,
        Key::Mute if down => {
            v.settings.muted = !v.settings.muted;
            v.save();
            v.send_state();
            return Some(if v.settings.muted { "Microphone muted" } else { "Microphone on" }.into());
        }
        Key::Disable if down => {
            v.settings.disabled = !v.settings.disabled;
            v.save();
            v.send_state();
            if v.settings.disabled {
                v.mixer.lock().unwrap().clear();
            }
            return Some(if v.settings.disabled { "Voice chat off" } else { "Voice chat on" }.into());
        }
        _ => {}
    }
    None
}

pub fn settings_buttons() -> Vec<String> {
    voice().settings.buttons()
}

pub fn settings_click(button: usize) {
    voice().click(button);
}

impl Voice {
    fn click(&mut self, button: usize) {
        let before = (self.settings.muted, self.settings.disabled);
        self.settings.click(button);
        self.save();
        if before != (self.settings.muted, self.settings.disabled) {
            self.send_state();
        }
    }

    /// Where the voice is going and what has come back, to tell a blocked
    /// port from packets this client can't read.
    fn connecting_line(&self) -> String {
        let shared = &self.shared;
        let target = shared.voice_target.lock().unwrap();
        let count = |c: &std::sync::atomic::AtomicU32| c.load(std::sync::atomic::Ordering::Relaxed);
        format!(
            "Connecting to {target}: sent {}, got {}, unreadable {}",
            count(&shared.tries),
            count(&shared.received),
            count(&shared.unreadable),
        )
    }

    fn status_line(&self) -> (String, u32) {
        let Some(s) = self.session.as_ref() else {
            return ("Not on a server".into(), GREY);
        };
        if s.connection.is_none() {
            return if s.info_count >= 4 {
                ("No Plasmo Voice on this server".into(), GREY)
            } else {
                ("Asking the server...".into(), YELLOW)
            };
        }
        match &*self.shared.status.lock().unwrap() {
            Status::Connected if self.shared.aes.lock().unwrap().is_some() => ("Connected (Plasmo Voice)".into(), GREEN),
            Status::Connected => ("Connected, waiting for the server's settings".into(), YELLOW),
            Status::Failed(e) => (e.clone(), RED),
            _ => (self.connecting_line(), YELLOW),
        }
    }

    fn view(&self, hotkeys: Vec<(String, String)>) -> ui::View {
        let session = self.session.as_ref();
        let connected = session.is_some_and(|s| s.connection.is_some())
            && *self.shared.status.lock().unwrap() == Status::Connected;
        // The right side lists who has voice chat (Plasmo has no groups).
        let mut players: Vec<ui::GroupRow> = session
            .map(|s| {
                s.players
                    .values()
                    .map(|p| ui::GroupRow {
                        id: p.id,
                        name: p.nick.clone(),
                        has_password: false,
                        members: vec![if p.voice_disabled {
                            "voice off".into()
                        } else if p.mic_muted || p.muted {
                            "muted".into()
                        } else {
                            "voice on".into()
                        }],
                        ours: false,
                    })
                    .collect()
            })
            .unwrap_or_default();
        players.sort_by_key(|g| g.name.to_lowercase());
        ui::View {
            status: self.status_line(),
            settings: self.settings.clone(),
            level_db: self.shared.level(),
            connected,
            groups_enabled: false,
            groups: players,
            hotkeys,
        }
    }

    /// What a click or Enter in the menu asks of the voice chat.
    fn menu_action(&mut self, action: ui::Action) {
        use ui::Action as A;
        let Some(action) = self.menu.act(action) else { return };
        match action {
            A::Mode => self.click(0),
            A::Mute => self.click(1),
            A::Disable => self.click(2),
            A::Labels => self.click(8),
            A::Volume(d) => {
                self.settings.volume = (self.settings.volume as i32 + d).clamp(0, 200) as u32;
                self.save();
            }
            A::Gain(d) => {
                self.settings.mic_gain = (self.settings.mic_gain as i32 + d).clamp(25, 500) as u32;
                self.save();
            }
            A::Threshold(d) => {
                self.settings.threshold = (self.settings.threshold + d).clamp(-80, 0);
                self.save();
            }
            _ => {}
        }
    }
}

/// Opens the menu, opened by a key typing `opened_by`.
pub fn menu_open(opened_by: Option<char>) {
    voice().menu.open(opened_by);
}

pub fn menu_is_open() -> bool {
    voice().menu.open
}

/// The game's screen is (`ours`) or isn't the menu.
pub fn menu_sync(ours: bool) {
    voice().menu.sync_open(ours);
}

/// Whether the game should close the menu now (once).
pub fn menu_take_close() -> bool {
    voice().menu.take_close()
}

/// Draws the menu; `hotkeys` are what each key does and its key's name.
pub fn menu_draw(
    size: [f32; 2],
    mouse: [f32; 2],
    measure: &dyn Fn(&str) -> f32,
    hotkeys: Vec<(String, String)>,
) -> Vec<ui::Draw> {
    let mut v = voice();
    let view = v.view(hotkeys);
    v.menu.draw(&view, size, mouse, measure)
}

pub fn menu_click(at: [f32; 2]) {
    let mut v = voice();
    if let Some(action) = v.menu.hit(at) {
        v.menu_action(action);
    }
}

/// A key on the menu: `menu_key` (the open key) closes it unless a field
/// takes the typing.
pub fn menu_key(key: &str, shift: bool, menu_key: &str) {
    let mut v = voice();
    if key == menu_key && !v.menu.typing() {
        v.menu.close();
        return;
    }
    if let Some(action) = v.menu.key(key, shift) {
        v.menu_action(action);
    }
}

pub fn menu_char(c: char) {
    voice().menu.char_typed(c);
}

pub fn menu_scroll(steps: f64) {
    voice().menu.scroll_by(steps.signum() as i32);
}

/// `key.keyboard.v` → `v`: the character a key types, for single letters
/// and digits.
pub fn key_char(key: &str) -> Option<char> {
    let name = key.strip_prefix("key.keyboard.")?;
    let mut chars = name.chars();
    let c = chars.next()?;
    chars.next().is_none().then_some(c)
}

/// Loads the stored settings before the first world, for the settings page.
pub fn load(get: &dyn Fn(&str) -> Option<String>) {
    let mut v = voice();
    if !v.loaded {
        v.loaded = true;
        v.settings = Settings::load(get);
        v.apply_controls();
    }
}

/// Settings changed since the last call, to store.
pub fn take_store() -> Vec<(String, String)> {
    std::mem::take(&mut voice().store)
}

/// A line of the status the HUD shows, and its colour.
pub struct Line {
    pub text: String,
    pub rgb: u32,
}

const WHITE: u32 = 0xFFFFFF;
const GREY: u32 = 0xAAAAAA;
const GREEN: u32 = 0x55FF55;
const AQUA: u32 = 0x55FFFF;
const RED: u32 = 0xFF5555;
const YELLOW: u32 = 0xFFFF55;

/// The status for the HUD, top line first: nothing on a server without
/// voice chat.
pub fn hud(ptt_key: &str) -> Vec<Line> {
    let v = voice();
    let Some(session) = v.session.as_ref().filter(|_| v.settings.hud) else {
        return Vec::new();
    };
    if session.connection.is_none() {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let line = |text: String, rgb| Line { text, rgb };
    let status = v.shared.status.lock().unwrap().clone();
    match status {
        Status::Connected => {}
        Status::Failed(ref e) => lines.push(line(format!("Voice chat: {e}"), RED)),
        _ => lines.push(line("Voice chat: connecting...".into(), YELLOW)),
    }
    if let Some(e) = v.shared.mic_error.lock().unwrap().as_ref() {
        lines.push(line(format!("Voice chat: {e}"), RED));
    }
    if let Err(e) = &v.devices.lock().unwrap().input
        && e != "not open"
    {
        lines.push(line(format!("Microphone: {e}"), RED));
    }
    if let Err(e) = &v.devices.lock().unwrap().output
        && e != "not open"
    {
        lines.push(line(format!("Speakers: {e}"), RED));
    }
    if v.settings.disabled {
        lines.push(line("Voice chat off".into(), RED));
    } else if v.settings.muted {
        lines.push(line("Microphone muted".into(), RED));
    } else if v.shared.transmitting.load(Ordering::Relaxed) {
        let whisper = v.shared.controls.lock().unwrap().whisper;
        lines.push(if whisper { line("Whispering".into(), AQUA) } else { line("Talking".into(), GREEN) });
    } else if status == Status::Connected {
        lines.push(match v.settings.mode {
            Mode::PushToTalk => line(format!("Push to talk: {}", key_label(ptt_key)), GREY),
            Mode::Voice => line(format!("Voice activation ({:.0} dB)", v.shared.level()), GREY),
        });
    }
    let now = Instant::now();
    let mut talking: Vec<(String, bool)> = v
        .shared
        .talking
        .lock()
        .unwrap()
        .iter()
        .filter(|(_, (at, _))| now - *at < Duration::from_millis(300))
        .map(|(id, (_, whisper))| {
            let name = session.name(id).unwrap_or_else(|| "Someone".into());
            (name, *whisper)
        })
        .collect();
    talking.sort();
    for (name, whisper) in talking {
        lines.push(line(format!("> {name}"), if whisper { AQUA } else { WHITE }));
    }
    lines
}

/// The group names to draw above the players the camera sees, on a `size`
/// screen.
pub fn group_labels(_camera: &labels::Camera, _size: [f32; 2]) -> Vec<labels::Label> {
    Vec::new()
}

/// The activation icon for the HUD (`HudIconRenderer`): the connection's or the
/// microphone's state, else the activation while talking; None for nothing.
pub fn hud_icon() -> Option<&'static str> {
    let v = voice();
    let session = v.session.as_ref()?;
    session.connection.as_ref()?;
    if *v.shared.status.lock().unwrap() != Status::Connected {
        return None;
    }
    if v.shared.timed_out.load(Ordering::Relaxed) {
        return Some("microphone_disconnected");
    }
    if v.settings.disabled {
        return Some("speaker_disabled");
    }
    let own = session.players.values().find(|p| Some(p.id) == session.own).map(|p| p.muted).unwrap_or(false);
    if own {
        return Some("microphone_muted");
    }
    if v.settings.muted {
        return Some("microphone_disabled");
    }
    if v.shared.transmitting.load(Ordering::Relaxed) {
        let icon = v.shared.talk.lock().unwrap().as_ref().map(|t| t.icon.clone()).unwrap_or_default();
        return Some(icons::name_of(&icon).unwrap_or("microphone"));
    }
    None
}

/// The icon above a player's head (`EntityIconStateExtractor.getPlayerIcon`):
/// not installed, muted by us, muted by the server, voice chat off, else the
/// source line they are heard on while they talk.
pub fn player_icon(player: u128) -> Option<&'static str> {
    let v = voice();
    let session = v.session.as_ref()?;
    session.connection.as_ref()?;
    if *v.shared.status.lock().unwrap() != Status::Connected || Some(player) == session.own {
        return None;
    }
    let hidden = |what: &str| session.config.as_ref().is_some_and(|c| c.icon_visibility.iter().any(|h| h == what));
    let Some(info) = session.players.get(&player) else {
        return (!hidden("HIDE_NOT_INSTALLED")).then_some("headset_not_installed");
    };
    let client_muted = v.shared.controls.lock().unwrap().player_volumes.get(&player).is_some_and(|&g| g == 0.0);
    if client_muted {
        return (!hidden("HIDE_CLIENT_MUTED")).then_some("speaker_disabled");
    }
    if info.muted {
        return (!hidden("HIDE_SERVER_MUTED")).then_some("speaker_muted");
    }
    if info.voice_disabled {
        return (!hidden("HIDE_VOICE_CHAT_DISABLED")).then_some("headset_disabled");
    }
    if hidden("HIDE_SOURCE_ICON") {
        return None;
    }
    let talking = v.shared.talking.lock().unwrap().get(&player).is_some_and(|(at, _)| at.elapsed() < Duration::from_millis(300));
    if !talking {
        return None;
    }
    let line = v.shared.talking_line.lock().unwrap().get(&player).copied();
    let icon = line
        .and_then(|l| session.config.as_ref()?.lines.iter().find(|x| x.id == l).map(|x| x.icon.clone()))
        .unwrap_or_default();
    Some(icons::name_of(&icon).unwrap_or("speaker"))
}

/// Where player icons sit above the name tag, from the server's config.
pub fn icon_offset() -> [f64; 3] {
    voice().session.as_ref().and_then(|s| s.config.as_ref()).map_or([0.0; 3], |c| c.icon_offset)
}

/// The overlay (`OverlayRenderer`): who is talking now, by name, with their line's icon.
pub fn overlay() -> Vec<(String, &'static str)> {
    let v = voice();
    let Some(session) = v.session.as_ref().filter(|s| s.connection.is_some()) else {
        return Vec::new();
    };
    let talking = v.shared.talking.lock().unwrap();
    let lines = v.shared.talking_line.lock().unwrap();
    let mut out: Vec<(String, &'static str)> = talking
        .iter()
        .filter(|(_, (at, _))| at.elapsed() < Duration::from_millis(300))
        .map(|(id, _)| {
            let icon = lines
                .get(id)
                .and_then(|l| session.config.as_ref()?.lines.iter().find(|x| x.id == *l).map(|x| x.icon.clone()))
                .unwrap_or_default();
            (session.name(id).unwrap_or_else(|| "?".into()), icons::name_of(&icon).unwrap_or("speaker"))
        })
        .collect();
    if v.shared.transmitting.load(Ordering::Relaxed)
        && let Some(own) = session.own.and_then(|o| session.name(&o))
    {
        out.push((own, "microphone"));
    }
    out.sort();
    out
}

/// `key.keyboard.caps.lock` → `Caps Lock`.
pub fn key_label(key: &str) -> String {
    let name = key.strip_prefix("key.keyboard.").or_else(|| key.strip_prefix("key.mouse.")).unwrap_or(key);
    if name == "unknown" {
        return "not bound".into();
    }
    name.split('.')
        .map(|w| {
            let mut c = w.chars();
            c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub const COMMAND: &str = "vc";

const HELP: &[&str] = &[
    "/vc status - the connection, devices and who has voice chat",
    "/vc mute, /vc unmute - the microphone",
    "/vc off, /vc on - all of voice chat",
    "/vc mode ptt|voice - push to talk or voice activation",
    "/vc volume <0-200> - everyone's voices",
    "/vc volume <player> <0-200> - one player's voice",
    "/vc gain <10-500> - the microphone's gain",
    "/vc threshold <-127-0> - the voice activation level, in dB",
];

/// `/vc …`, without the slash: the chat lines it answers, or None when the
/// command isn't the voice chat's.
pub fn command(line: &str) -> Option<Vec<String>> {
    let mut words = line.split_whitespace();
    if words.next() != Some(COMMAND) {
        return None;
    }
    let args: Vec<&str> = words.collect();
    let mut v = voice();
    let out = |s: &str| vec![s.to_owned()];
    Some(match args.as_slice() {
        [] | ["help"] => HELP.iter().map(|s| s.to_string()).collect(),
        ["status"] => v.status(),
        ["mute"] | ["unmute"] => {
            v.settings.muted = args[0] == "mute";
            v.save();
            v.send_state();
            out(if v.settings.muted { "Microphone muted" } else { "Microphone on" })
        }
        ["off"] | ["on"] => {
            v.settings.disabled = args[0] == "off";
            v.save();
            v.send_state();
            out(if v.settings.disabled { "Voice chat off" } else { "Voice chat on" })
        }
        ["mode", m] => {
            v.settings.mode = match *m {
                "voice" | "va" => Mode::Voice,
                "ptt" | "push" => Mode::PushToTalk,
                _ => return Some(out("Modes: ptt, voice")),
            };
            v.save();
            out(if v.settings.mode == Mode::Voice { "Voice activation" } else { "Push to talk" })
        }
        ["volume", n] => match n.parse::<u32>() {
            Ok(n) if n <= 200 => {
                v.settings.volume = n;
                v.save();
                vec![format!("Voice volume {n}%")]
            }
            _ => out("Volume: 0 to 200"),
        },
        ["volume", name, n] => {
            let Ok(n) = n.parse::<u32>().map(|n| n.min(400)) else {
                return Some(out("Volume: 0 to 400"));
            };
            let id = v.session.as_ref().and_then(|s| {
                s.players.values().find(|p| p.nick.eq_ignore_ascii_case(name)).map(|p| p.id)
            });
            match id {
                Some(id) => {
                    v.shared.controls.lock().unwrap().player_volumes.insert(id, n as f32 / 100.0);
                    vec![format!("{name}'s voice at {n}%")]
                }
                None => vec![format!("No voice chat player named {name}")],
            }
        }
        ["gain", n] => match n.parse::<u32>() {
            Ok(n) if (10..=500).contains(&n) => {
                v.settings.mic_gain = n;
                v.save();
                vec![format!("Microphone gain {n}%")]
            }
            _ => out("Gain: 10 to 500"),
        },
        ["threshold", n] => match n.parse::<i32>() {
            Ok(n) if (-127..=0).contains(&n) => {
                v.settings.threshold = n;
                v.save();
                vec![format!("Voice activation at {n} dB (now {:.0} dB)", v.shared.level())]
            }
            _ => out("Threshold: -127 to 0 dB"),
        },
        _ => HELP.iter().map(|s| s.to_string()).collect(),
    })
}

impl Voice {
    fn status(&self) -> Vec<String> {
        let mut out = Vec::new();
        let Some(s) = self.session.as_ref() else {
            return vec!["Not on a server".into()];
        };
        match &s.connection {
            None => out.push("This server hasn't connected Plasmo Voice (it may not have it)".into()),
            Some((_, ip, port)) => {
                let distance = self.shared.talk.lock().unwrap().as_ref().map_or(0, |t| t.distance);
                out.push(format!(
                    "Plasmo Voice {:?} (voice server {ip}:{port}, talking distance {distance} blocks)",
                    *self.shared.status.lock().unwrap()
                ));
            }
        }
        let d = self.devices.lock().unwrap();
        let dev = |r: &Result<String, String>| r.clone().unwrap_or_else(|e| format!("none ({e})"));
        out.push(format!("Microphone: {}, speakers: {}", dev(&d.input), dev(&d.output)));
        if let Some(e) = self.shared.mic_error.lock().unwrap().as_ref() {
            out.push(format!("Problem: {e}"));
        }
        let mut names: Vec<String> = s
            .players
            .values()
            .map(|p| format!("{}{}", p.nick, if p.voice_disabled { " (off)" } else if p.mic_muted { " (muted)" } else { "" }))
            .collect();
        names.sort();
        if !names.is_empty() {
            out.push(format!("With voice chat: {}", names.join(", ")));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(address: &str) -> World<'_> {
        World { address, listener: None, players: Vec::new(), redirect: |h, p| (h.to_owned(), p), own: None }
    }

    #[test]
    fn registers_the_channel_and_ignores_other_payloads() {
        use_sound_card(false);
        let none = |_: &str| None;
        let t = tick(Some(world("play.example.net")), &none);
        assert_eq!(t.payloads[0].0, "minecraft:register");
        assert_eq!(t.payloads[0].1, b"plasmo:voice/v2\0plasmo:voice/v2/service");
        assert!(!payload("minecraft:brand", b""));
        let t = tick(None, &none);
        assert!(t.payloads.is_empty());
        assert!(voice().session.is_none());
    }

    #[test]
    fn talks_with_the_proximity_activation() {
        let codec = pv::Codec { name: "opus".into(), params: vec![("mode".into(), "VOIP".into()), ("bitrate".into(), "24000".into())] };
        let proximity = pv::Activation {
            id: pv::activation_id("proximity"),
            name: "proximity".into(),
            translation: String::new(),
            icon: "plasmovoice:textures/icons/microphone.png".into(),
            distances: vec![8, 16, 32],
            default_distance: 16,
            proximity: true,
            transitive: true,
            stereo: false,
            encoder: None,
            weight: 0,
        };
        let config = pv::Config {
            server_id: 1,
            sample_rate: 48_000,
            mtu: 1024,
            encoder: Some(codec),
            encryption: None,
            lines: Vec::new(),
            activations: vec![proximity],
            permissions: Vec::new(),
            icon_visibility: Vec::new(),
            icon_offset: [0.0; 3],
        };
        let t = talk_from(&config).unwrap();
        assert_eq!((t.distance, t.bitrate, t.opus_application), (16, 24_000, opus::APPLICATION_VOIP));
        assert_eq!(split_host_port("play.minehouse.pro"), ("play.minehouse.pro".into(), 25565));
        assert_eq!(split_host_port("1.2.3.4:25566"), ("1.2.3.4".into(), 25566));
    }

    #[test]
    fn settings_round_trip_and_cycle() {
        let mut s = Settings::default();
        s.click(0);
        s.click(3);
        let stored: HashMap<_, _> = s.stored().into_iter().collect();
        let back = Settings::load(&|k| stored.get(k).cloned());
        assert_eq!(back, s);
        assert_eq!((back.mode, back.volume), (Mode::Voice, 125));
        assert_eq!(key_label("key.keyboard.caps.lock"), "Caps Lock");
    }
}
