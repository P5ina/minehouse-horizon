//! Plasmo Voice 2.x's wire format, written from its published protocol
//! (packet ids and layouts of `su.plo.voice.proto`, checked against the
//! 2.1.17 jar a MineHouse server runs). Game-connection packets go over the
//! `plasmo:voice/v2` custom payload; voice goes over UDP.

pub const CHANNEL: &str = "plasmo:voice/v2";
pub const CHANNEL_SERVICE: &str = "plasmo:voice/v2/service";
pub const CHANNEL_INSTALLED: &str = "plasmo:voice/v2/installed";
/// The version we say we are: the server only checks the major (2) and its minimum.
pub const MOD_VERSION: &str = "2.1.17";
pub const MC_VERSION: &str = "26.3";
pub const UDP_MAGIC: i32 = 0x4e90_04e9;
pub const MAX_UDP: usize = 4096;

pub type Uuid = u128;

// --- Java DataOutput / DataInput ----------------------------------------------

#[derive(Default)]
pub struct Out(pub Vec<u8>);

impl Out {
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    pub fn bool(&mut self, v: bool) -> &mut Self {
        self.u8(u8::from(v))
    }
    pub fn i16(&mut self, v: i16) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn i32(&mut self, v: i32) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn i64(&mut self, v: i64) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn uuid(&mut self, v: Uuid) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }
    pub fn bytes(&mut self, v: &[u8]) -> &mut Self {
        self.i32(v.len() as i32);
        self.0.extend_from_slice(v);
        self
    }
    /// `writeUTF`: a u16 length, then modified UTF-8 (NUL as C0 80, astral
    /// characters as two 3-byte surrogates).
    pub fn utf(&mut self, s: &str) -> &mut Self {
        let mut enc = Vec::with_capacity(s.len());
        for u in s.encode_utf16() {
            match u {
                1..=0x7F => enc.push(u as u8),
                0 | 0x80..=0x7FF => {
                    enc.push(0xC0 | (u >> 6) as u8);
                    enc.push(0x80 | (u & 0x3F) as u8);
                }
                _ => {
                    enc.push(0xE0 | (u >> 12) as u8);
                    enc.push(0x80 | ((u >> 6) & 0x3F) as u8);
                    enc.push(0x80 | (u & 0x3F) as u8);
                }
            }
        }
        enc.truncate(u16::MAX as usize);
        self.0.extend_from_slice(&(enc.len() as u16).to_be_bytes());
        self.0.extend_from_slice(&enc);
        self
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Short;

pub type R<T> = Result<T, Short>;

pub struct In<'a>(pub &'a [u8]);

impl<'a> In<'a> {
    fn take(&mut self, n: usize) -> R<&'a [u8]> {
        if self.0.len() < n {
            return Err(Short);
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    pub fn u8(&mut self) -> R<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn bool(&mut self) -> R<bool> {
        Ok(self.u8()? != 0)
    }
    pub fn i16(&mut self) -> R<i16> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn u16(&mut self) -> R<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn i32(&mut self) -> R<i32> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn i64(&mut self) -> R<i64> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn f64(&mut self) -> R<f64> {
        Ok(f64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn uuid(&mut self) -> R<Uuid> {
        Ok(u128::from_be_bytes(self.take(16)?.try_into().unwrap()))
    }
    /// A length in `0..=max` (`PacketUtil.readSafeInt`).
    pub fn len(&mut self, max: usize) -> R<usize> {
        let n = self.i32()?;
        usize::try_from(n).ok().filter(|&n| n <= max).ok_or(Short)
    }
    pub fn bytes(&mut self, max: usize) -> R<Vec<u8>> {
        let n = self.len(max)?;
        Ok(self.take(n)?.to_vec())
    }
    /// `readUTF`.
    pub fn utf(&mut self) -> R<String> {
        let n = self.u16()? as usize;
        let raw = self.take(n)?;
        let mut units = Vec::with_capacity(n);
        let mut i = 0;
        while i < raw.len() {
            let b = raw[i];
            if b < 0x80 {
                units.push(u16::from(b));
                i += 1;
            } else if b & 0xE0 == 0xC0 && i + 1 < raw.len() {
                units.push((u16::from(b & 0x1F) << 6) | u16::from(raw[i + 1] & 0x3F));
                i += 2;
            } else if b & 0xF0 == 0xE0 && i + 2 < raw.len() {
                units.push(
                    (u16::from(b & 0x0F) << 12)
                        | (u16::from(raw[i + 1] & 0x3F) << 6)
                        | u16::from(raw[i + 2] & 0x3F),
                );
                i += 3;
            } else {
                return Err(Short);
            }
        }
        Ok(String::from_utf16_lossy(&units))
    }
    pub fn nullable_utf(&mut self) -> R<Option<String>> {
        if self.bool()? { Ok(Some(self.utf()?)) } else { Ok(None) }
    }
    pub fn pos(&mut self) -> R<[f64; 3]> {
        Ok([self.f64()?, self.f64()?, self.f64()?])
    }
}

// --- Data types -----------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Codec {
    pub name: String,
    pub params: Vec<(String, String)>,
}

impl Codec {
    fn read(r: &mut In) -> R<Self> {
        let name = r.utf()?;
        let n = r.len(128)?;
        let mut params = Vec::with_capacity(n);
        for _ in 0..n {
            params.push((r.utf()?, r.utf()?));
        }
        Ok(Self { name, params })
    }
    fn read_opt(r: &mut In) -> R<Option<Self>> {
        if r.bool()? { Ok(Some(Self::read(r)?)) } else { Ok(None) }
    }
    pub fn param(&self, key: &str) -> Option<&str> {
        self.params.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub id: Uuid,
    pub name: String,
}

fn read_profile(r: &mut In) -> R<Profile> {
    let id = r.uuid()?;
    let name = r.utf()?;
    let n = r.len(100)?;
    for _ in 0..n {
        r.utf()?;
        r.utf()?;
        r.utf()?;
    }
    Ok(Profile { id, name })
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerInfo {
    pub id: Uuid,
    pub nick: String,
    pub muted: bool,
    pub voice_disabled: bool,
    pub mic_muted: bool,
}

fn read_player_info(r: &mut In) -> R<PlayerInfo> {
    Ok(PlayerInfo {
        id: r.uuid()?,
        nick: r.utf()?,
        muted: r.bool()?,
        voice_disabled: r.bool()?,
        mic_muted: r.bool()?,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Activation {
    pub id: Uuid,
    pub name: String,
    pub translation: String,
    pub icon: String,
    pub distances: Vec<i32>,
    pub default_distance: i32,
    pub proximity: bool,
    pub transitive: bool,
    pub stereo: bool,
    pub encoder: Option<Codec>,
    pub weight: i32,
}

/// `UUID.nameUUIDFromBytes`: an MD5 of the name, version 3.
pub fn name_uuid(name: &str) -> Uuid {
    let mut h = md5(name.as_bytes());
    h[6] = (h[6] & 0x0F) | 0x30;
    h[8] = (h[8] & 0x3F) | 0x80;
    u128::from_be_bytes(h)
}

pub fn activation_id(name: &str) -> Uuid {
    name_uuid(&format!("{name}_activation"))
}

pub fn line_id(name: &str) -> Uuid {
    name_uuid(&format!("{name}_line"))
}

fn read_activation(r: &mut In) -> R<Activation> {
    let name = r.utf()?;
    let translation = r.utf()?;
    let icon = r.utf()?;
    let n = r.len(64)?;
    let mut distances = Vec::with_capacity(n);
    for _ in 0..n {
        distances.push(r.i32()?);
    }
    let default_distance = r.i32()?;
    Ok(Activation {
        id: activation_id(&name),
        default_distance: valid_default_distance(&distances, default_distance),
        name,
        translation,
        icon,
        distances,
        proximity: r.bool()?,
        transitive: r.bool()?,
        stereo: r.bool()?,
        encoder: Codec::read_opt(r)?,
        weight: r.i32()?,
    })
}

/// `VoiceActivation.validateDefaultDistance`.
pub fn valid_default_distance(distances: &[i32], default: i32) -> i32 {
    if distances.is_empty() {
        return 0;
    }
    if distances.len() == 2 && distances[0] == -1 {
        return if (1..=distances[1]).contains(&default) { default } else { distances[1] / 2 };
    }
    if distances.contains(&default) { default } else { distances[distances.len() / 2] }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub id: Uuid,
    pub name: String,
    pub translation: String,
    pub icon: String,
    pub default_volume: f64,
    pub weight: i32,
}

fn read_line(r: &mut In) -> R<Line> {
    let name = r.utf()?;
    let translation = r.utf()?;
    let icon = r.utf()?;
    let default_volume = r.f64()?;
    let weight = r.i32()?;
    if r.bool()? {
        let n = r.len(i16::MAX as usize)?;
        for _ in 0..n {
            read_profile(r)?;
        }
    }
    Ok(Line { id: line_id(&name), name, translation, icon, default_volume, weight })
}

#[derive(Clone, Debug, PartialEq)]
pub enum SourceKind {
    Player(PlayerInfo),
    Entity(i32),
    Static([f64; 3]),
    Direct { sender: Option<Profile>, relative: Option<[f64; 3]> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub addon: String,
    pub id: Uuid,
    pub name: Option<String>,
    pub state: u8,
    pub decoder: Option<Codec>,
    pub stereo: bool,
    pub line: Uuid,
    pub icon_visible: bool,
    pub angle: i32,
    pub kind: SourceKind,
}

fn read_source(r: &mut In) -> R<Source> {
    let ty = r.utf()?;
    let addon = r.utf()?;
    let id = r.uuid()?;
    let name = r.nullable_utf()?;
    let state = r.u8()?;
    let decoder = Codec::read_opt(r)?;
    let stereo = r.bool()?;
    let line = r.uuid()?;
    let icon_visible = r.bool()?;
    let angle = r.i32()?;
    let kind = match ty.as_str() {
        "PLAYER" => SourceKind::Player(read_player_info(r)?),
        "ENTITY" => SourceKind::Entity(r.i32()?),
        "STATIC" => {
            let at = r.pos()?;
            r.pos()?;
            SourceKind::Static(at)
        }
        "DIRECT" => {
            let sender = if r.bool()? { Some(read_profile(r)?) } else { None };
            let relative = if r.bool()? { Some(r.pos()?) } else { None };
            let _camera_relative = r.bool()?;
            SourceKind::Direct { sender, relative }
        }
        _ => return Err(Short),
    };
    Ok(Source { addon, id, name, state, decoder, stereo, line, icon_visible, angle, kind })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub server_id: Uuid,
    pub sample_rate: i32,
    pub mtu: i32,
    pub encoder: Option<Codec>,
    /// `(algorithm, RSA-sealed key)`.
    pub encryption: Option<(String, Vec<u8>)>,
    pub lines: Vec<Line>,
    pub activations: Vec<Activation>,
    pub permissions: Vec<(String, bool)>,
    /// `PlayerIconConfig`: which player icons the server hides, and where icons sit above heads.
    pub icon_visibility: Vec<String>,
    pub icon_offset: [f64; 3],
}

fn read_permissions(r: &mut In) -> R<Vec<(String, bool)>> {
    let n = r.len(127)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push((r.utf()?, r.bool()?));
    }
    Ok(out)
}

fn read_config(r: &mut In) -> R<Config> {
    let server_id = r.uuid()?;
    let sample_rate = r.i32()?;
    let mtu = r.i32()?;
    let encoder = Codec::read_opt(r)?;
    let encryption = if r.bool()? {
        let algorithm = r.utf()?;
        let n = r.i32()?;
        let n = usize::try_from(n).ok().filter(|&n| (1..=2048).contains(&n)).ok_or(Short)?;
        Some((algorithm, r.take(n)?.to_vec()))
    } else {
        None
    };
    let n = r.len(127)?;
    let mut lines = Vec::with_capacity(n);
    for _ in 0..n {
        lines.push(read_line(r)?);
    }
    let n = r.len(127)?;
    let mut activations = Vec::with_capacity(n);
    for _ in 0..n {
        activations.push(read_activation(r)?);
    }
    let permissions = read_permissions(r)?;
    // The player icon config, which older servers leave out.
    let (icon_visibility, icon_offset) = (|| -> R<(Vec<String>, [f64; 3])> {
        let n = r.len(5)?;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(r.utf()?);
        }
        Ok((v, r.pos()?))
    })()
    .unwrap_or_default();
    Ok(Config { server_id, sample_rate, mtu, encoder, encryption, lines, activations, permissions, icon_visibility, icon_offset })
}

// --- Game-connection packets ------------------------------------------------------

/// What the server sends over `plasmo:voice/v2`.
#[derive(Clone, Debug, PartialEq)]
pub enum Tcp {
    Connection { secret: Uuid, ip: String, port: i32 },
    PlayerInfoRequest,
    Config(Config),
    Permissions(Vec<(String, bool)>),
    PlayerList(Vec<PlayerInfo>),
    PlayerInfoUpdate(PlayerInfo),
    PlayerDisconnect(Uuid),
    SourceInfo(Source),
    SelfSourceInfo { source: Source, player: Uuid, activation: Uuid, sequence: i64 },
    SourceAudioEnd { source: Uuid, sequence: i64 },
    ActivationRegister(Activation),
    ActivationUnregister(Uuid),
    LineRegister(Line),
    LineUnregister(Uuid),
    ActionBar(String),
    Other(u8),
}

pub mod id {
    pub const CONNECTION: u8 = 1;
    pub const PLAYER_INFO_REQUEST: u8 = 2;
    pub const CONFIG: u8 = 3;
    pub const CONFIG_PLAYER_INFO: u8 = 4;
    pub const LANGUAGE_REQUEST: u8 = 5;
    pub const LANGUAGE: u8 = 6;
    pub const PLAYER_LIST: u8 = 7;
    pub const PLAYER_INFO_UPDATE: u8 = 8;
    pub const PLAYER_DISCONNECT: u8 = 9;
    pub const PLAYER_INFO: u8 = 10;
    pub const PLAYER_STATE: u8 = 11;
    pub const PLAYER_AUDIO_END: u8 = 12;
    pub const PLAYER_ACTIVATION_DISTANCES: u8 = 13;
    pub const DISTANCE_VISUALIZE: u8 = 14;
    pub const SOURCE_INFO_REQUEST: u8 = 15;
    pub const SOURCE_INFO: u8 = 16;
    pub const SELF_SOURCE_INFO: u8 = 17;
    pub const SOURCE_AUDIO_END: u8 = 18;
    pub const ACTIVATION_REGISTER: u8 = 19;
    pub const ACTIVATION_UNREGISTER: u8 = 20;
    pub const LINE_REGISTER: u8 = 21;
    pub const LINE_UNREGISTER: u8 = 22;
    pub const LINE_PLAYER_ADD: u8 = 23;
    pub const LINE_PLAYER_REMOVE: u8 = 24;
    pub const LINE_PLAYERS_LIST: u8 = 25;
    pub const ANIMATED_ACTION_BAR: u8 = 26;
}

pub fn decode_tcp(data: &[u8]) -> R<Tcp> {
    let mut r = In(data);
    let kind = r.u8()?;
    Ok(match kind {
        id::CONNECTION => Tcp::Connection { secret: r.uuid()?, ip: r.utf()?, port: r.i32()? },
        id::PLAYER_INFO_REQUEST => Tcp::PlayerInfoRequest,
        id::CONFIG => Tcp::Config(read_config(&mut r)?),
        id::CONFIG_PLAYER_INFO => Tcp::Permissions(read_permissions(&mut r)?),
        id::PLAYER_LIST => {
            let n = r.len(i16::MAX as usize)?;
            let mut list = Vec::with_capacity(n);
            for _ in 0..n {
                list.push(read_player_info(&mut r)?);
            }
            Tcp::PlayerList(list)
        }
        id::PLAYER_INFO_UPDATE => Tcp::PlayerInfoUpdate(read_player_info(&mut r)?),
        id::PLAYER_DISCONNECT => Tcp::PlayerDisconnect(r.uuid()?),
        id::SOURCE_INFO => Tcp::SourceInfo(read_source(&mut r)?),
        id::SELF_SOURCE_INFO => Tcp::SelfSourceInfo {
            source: read_source(&mut r)?,
            player: r.uuid()?,
            activation: r.uuid()?,
            sequence: r.i64()?,
        },
        id::SOURCE_AUDIO_END => Tcp::SourceAudioEnd { source: r.uuid()?, sequence: r.i64()? },
        id::ACTIVATION_REGISTER => Tcp::ActivationRegister(read_activation(&mut r)?),
        id::ACTIVATION_UNREGISTER => Tcp::ActivationUnregister(r.uuid()?),
        id::LINE_REGISTER => Tcp::LineRegister(read_line(&mut r)?),
        id::LINE_UNREGISTER => Tcp::LineUnregister(r.uuid()?),
        id::ANIMATED_ACTION_BAR => Tcp::ActionBar(r.utf()?),
        other => Tcp::Other(other),
    })
}

/// `PlayerInfoPacket`: who we are, our RSA public key (X.509), and our state.
pub fn player_info(public_key_der: &[u8], voice_disabled: bool, mic_muted: bool) -> Vec<u8> {
    let mut o = Out::default();
    o.u8(id::PLAYER_INFO)
        .bool(voice_disabled)
        .bool(mic_muted)
        .utf(MC_VERSION)
        .utf(MOD_VERSION)
        .bytes(public_key_der);
    o.0
}

pub fn player_state(voice_disabled: bool, mic_muted: bool) -> Vec<u8> {
    let mut o = Out::default();
    o.u8(id::PLAYER_STATE).bool(voice_disabled).bool(mic_muted);
    o.0
}

pub fn player_audio_end(sequence: i64, activation: Uuid, distance: i16) -> Vec<u8> {
    let mut o = Out::default();
    o.u8(id::PLAYER_AUDIO_END).i64(sequence).uuid(activation).i16(distance);
    o.0
}

pub fn source_info_request(source: Uuid) -> Vec<u8> {
    let mut o = Out::default();
    o.u8(id::SOURCE_INFO_REQUEST).uuid(source);
    o.0
}

pub fn activation_distances(distances: &[(Uuid, i32)]) -> Vec<u8> {
    let mut o = Out::default();
    o.u8(id::PLAYER_ACTIVATION_DISTANCES).i32(distances.len() as i32);
    for &(a, d) in distances {
        o.uuid(a).i32(d);
    }
    o.0
}

/// `minecraft:register`'s body: Fabric sends a channel only to a client that registered it.
pub fn register_payload() -> Vec<u8> {
    [CHANNEL, CHANNEL_SERVICE].join("\0").into_bytes()
}

// --- UDP ----------------------------------------------------------------------------

pub mod udp {
    pub const PING: u8 = 1;
    pub const PLAYER_AUDIO: u8 = 2;
    pub const SOURCE_AUDIO: u8 = 3;
    pub const SELF_AUDIO_INFO: u8 = 4;
}

fn udp_header(o: &mut Out, kind: u8, secret: Uuid, now_ms: i64) {
    o.i32(UDP_MAGIC).u8(kind).uuid(secret).i64(now_ms);
}

/// A ping: the first ones carry where we connected (the server checks it), the
/// answers to the server's keep-alives carry nothing.
pub fn udp_ping(secret: Uuid, now_ms: i64, server: Option<(&str, u16)>) -> Vec<u8> {
    let mut o = Out::default();
    udp_header(&mut o, udp::PING, secret, now_ms);
    o.i64(now_ms);
    if let Some((ip, port)) = server {
        o.utf(ip).i16(port as i16);
    }
    o.0
}

/// `PlayerAudioPacket`: `data` is the encrypted Opus frame.
pub fn udp_player_audio(
    secret: Uuid,
    now_ms: i64,
    sequence: i64,
    data: &[u8],
    activation: Uuid,
    distance: i16,
    stereo: bool,
) -> Vec<u8> {
    let mut o = Out::default();
    udp_header(&mut o, udp::PLAYER_AUDIO, secret, now_ms);
    o.i64(sequence).bytes(data).uuid(activation).i16(distance).bool(stereo);
    o.0
}

#[derive(Clone, Debug, PartialEq)]
pub enum Udp {
    Ping,
    SourceAudio { sequence: i64, data: Vec<u8>, source: Uuid, state: u8, distance: i16 },
    SelfAudioInfo { source: Uuid, sequence: i64, data: Option<Vec<u8>>, distance: i16 },
    Other(u8),
}

pub fn decode_udp(data: &[u8]) -> R<Udp> {
    let mut r = In(data);
    if r.i32()? != UDP_MAGIC {
        return Err(Short);
    }
    let kind = r.u8()?;
    let _secret = r.uuid()?;
    let _timestamp = r.i64()?;
    Ok(match kind {
        udp::PING => Udp::Ping,
        udp::SOURCE_AUDIO => {
            let sequence = r.i64()?;
            let n = r.i32()?;
            let n = usize::try_from(n).ok().filter(|&n| (1..=2048).contains(&n)).ok_or(Short)?;
            let data = r.take(n)?.to_vec();
            Udp::SourceAudio { sequence, data, source: r.uuid()?, state: r.u8()?, distance: r.i16()? }
        }
        udp::SELF_AUDIO_INFO => {
            let source = r.uuid()?;
            let sequence = r.i64()?;
            let data = if r.bool()? { Some(r.bytes(2048)?) } else { None };
            Udp::SelfAudioInfo { source, sequence, data, distance: r.i16()? }
        }
        other => Udp::Other(other),
    })
}

// --- MD5, for nameUUIDFromBytes ------------------------------------------------------

fn md5(input: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
        14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
        21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32).collect();
    let (mut a0, mut b0, mut c0, mut d0) = (0x6745_2301u32, 0xefcd_ab89u32, 0x98ba_dcfeu32, 0x1032_5476u32);
    let mut msg = input.to_vec();
    let bits = (input.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_le_bytes());
    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk.chunks(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    for (i, v) in [a0, b0, c0, d0].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_matches_the_rfc_vectors() {
        let hex = |b: [u8; 16]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        assert_eq!(hex(md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex(md5(b"abc")), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn proximity_ids_match_java() {
        // jshell: UUID.nameUUIDFromBytes("proximity_activation".getBytes(UTF_8))
        assert_eq!(
            format!("{:032x}", activation_id("proximity")),
            JAVA_PROXIMITY_ACTIVATION.replace('-', "")
        );
    }
    // Filled from jshell (see the test above).
    const JAVA_PROXIMITY_ACTIVATION: &str = include_str!("../testdata/proximity_activation.txt");

    #[test]
    fn modified_utf8_round_trips_nul_cyrillic_and_emoji() {
        let s = "a\0Привет🙂";
        let mut o = Out::default();
        o.utf(s);
        // NUL is two bytes, the emoji two 3-byte surrogates.
        assert_eq!(o.0[3..5], [0xC0, 0x80]);
        assert_eq!(In(&o.0).utf().unwrap(), s);
    }

    #[test]
    fn a_connection_packet_decodes() {
        let mut o = Out::default();
        o.u8(id::CONNECTION).uuid(42).utf("0.0.0.0").i32(24454);
        assert_eq!(
            decode_tcp(&o.0).unwrap(),
            Tcp::Connection { secret: 42, ip: "0.0.0.0".into(), port: 24454 }
        );
    }

    #[test]
    fn source_audio_decodes_and_garbage_does_not() {
        let mut o = Out::default();
        o.i32(UDP_MAGIC).u8(udp::SOURCE_AUDIO).uuid(1).i64(5);
        o.i64(9).bytes(&[1, 2, 3]).uuid(7).u8(2).i16(48);
        assert_eq!(
            decode_udp(&o.0).unwrap(),
            Udp::SourceAudio { sequence: 9, data: vec![1, 2, 3], source: 7, state: 2, distance: 48 }
        );
        assert!(decode_udp(&o.0[..20]).is_err());
        assert!(decode_udp(&[0; 40]).is_err());
    }

    #[test]
    fn default_distance_follows_plasmo() {
        assert_eq!(valid_default_distance(&[8, 16, 32], 16), 16);
        assert_eq!(valid_default_distance(&[8, 16, 32], 5), 16);
        assert_eq!(valid_default_distance(&[-1, 64], 100), 32);
        assert_eq!(valid_default_distance(&[], 7), 0);
    }
}
