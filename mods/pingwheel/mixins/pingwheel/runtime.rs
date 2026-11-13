// Ping Wheel for Horizon: speaks the Fabric mod Ping Wheel 1.12's protocol
// (https://github.com/LukenSkyne/Minecraft-Ping-Wheel, MIT) so a Horizon
// player pings and sees the pings of Java players on the same server.
// This file is both the mod's Cargo library (its tests) and the module the
// bundle adds to the game.

pub const ID: &str = "pingwheel";
pub const NAME: &str = "Ping Wheel";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str =
    "Mark a spot for the players near you with the middle mouse button; compatible with the Ping Wheel mod.";

/// The key a player can rebind (`key.pingwheel.ping` in horizon-mods.txt).
pub const KEY: &str = "ping";
pub const DEFAULT_KEY: &str = "key.mouse.middle";

pub const C2S_PING: &str = "ping-wheel-c2s:ping-location";
pub const C2S_CHANNEL: &str = "ping-wheel-c2s:update-channel";
pub const S2C_PING: &str = "ping-wheel-s2c:ping-location";
/// Fabric only sends a play channel to a client that registered it.
pub const REGISTER: &str = "minecraft:register";
const MAX_CHANNEL_LENGTH: usize = 128;

/// How far a ping reaches and how long it stays, as Ping Wheel's defaults.
pub const RAYCAST_DISTANCE: f64 = 1024.0;
pub const PING_DURATION_TICKS: u32 = 7 * 20;

#[derive(Clone, Debug, PartialEq)]
pub struct Ping {
    pub channel: String,
    pub pos: [f64; 3],
    pub entity: Option<u128>,
    pub sequence: i32,
    pub dimension: i32,
    pub author: Option<u128>,
}

// --- Java's encodings -------------------------------------------------------

fn write_varint(out: &mut Vec<u8>, mut v: i32) {
    loop {
        let mut b = (v & 0x7F) as u8;
        v = ((v as u32) >> 7) as i32;
        if v != 0 {
            b |= 0x80;
        }
        out.push(b);
        if v == 0 {
            break;
        }
    }
}

fn write_utf(out: &mut Vec<u8>, s: &str) {
    let s = if s.chars().count() > MAX_CHANNEL_LENGTH {
        s.chars().take(MAX_CHANNEL_LENGTH).collect::<String>()
    } else {
        s.to_owned()
    };
    write_varint(out, s.len() as i32);
    out.extend_from_slice(s.as_bytes());
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        if self.0.len() < n {
            return None;
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Some(a)
    }
    fn varint(&mut self) -> Option<i32> {
        let mut v: u32 = 0;
        for i in 0..5 {
            let b = self.take(1)?[0];
            v |= u32::from(b & 0x7F) << (7 * i);
            if b & 0x80 == 0 {
                return Some(v as i32);
            }
        }
        None
    }
    fn utf(&mut self) -> Option<String> {
        let n = usize::try_from(self.varint()?).ok()?;
        String::from_utf8(self.take(n)?.to_vec()).ok()
    }
    fn f64(&mut self) -> Option<f64> {
        Some(f64::from_be_bytes(self.take(8)?.try_into().ok()?))
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_be_bytes(self.take(4)?.try_into().ok()?))
    }
    fn bool(&mut self) -> Option<bool> {
        Some(self.take(1)?[0] != 0)
    }
    fn uuid(&mut self) -> Option<u128> {
        Some(u128::from_be_bytes(self.take(16)?.try_into().ok()?))
    }
}

/// `String.hashCode`, over UTF-16 units.
pub fn java_string_hash(s: &str) -> i32 {
    s.encode_utf16()
        .fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(i32::from(c)))
}

/// `Identifier.hashCode`: `31 * namespace.hashCode() + path.hashCode()`.
/// Ping Wheel tells dimensions apart by it.
pub fn dimension_hash(id: &str) -> i32 {
    let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
    java_string_hash(ns)
        .wrapping_mul(31)
        .wrapping_add(java_string_hash(path))
}

/// `PingLocationC2SPacket.write`.
pub fn encode_ping(p: &Ping) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    write_utf(&mut out, &p.channel);
    for c in p.pos {
        out.extend_from_slice(&c.to_be_bytes());
    }
    out.push(u8::from(p.entity.is_some()));
    if let Some(e) = p.entity {
        out.extend_from_slice(&e.to_be_bytes());
    }
    out.extend_from_slice(&p.sequence.to_be_bytes());
    out.extend_from_slice(&p.dimension.to_be_bytes());
    out
}

/// `UpdateChannelC2SPacket.write`.
pub fn encode_channel(channel: &str) -> Vec<u8> {
    let mut out = Vec::new();
    write_utf(&mut out, channel);
    out
}

/// `minecraft:register`'s body: channel names, NUL-separated.
pub fn encode_register(channels: &[&str]) -> Vec<u8> {
    channels.join("\0").into_bytes()
}

/// `PingLocationS2CPacket`'s body; None when it doesn't parse.
pub fn decode_ping(data: &[u8]) -> Option<Ping> {
    let mut r = Reader(data);
    let channel = r.utf()?;
    let pos = [r.f64()?, r.f64()?, r.f64()?];
    let entity = if r.bool()? { Some(r.uuid()?) } else { None };
    let sequence = r.i32()?;
    let dimension = r.i32()?;
    let author = Some(r.uuid()?);
    Some(Ping {
        channel,
        pos,
        entity,
        sequence,
        dimension,
        author,
    })
}

/// Diagnostics in chat while `PINGWHEEL_DEBUG` is set.
pub fn debug_on() -> bool {
    std::env::var_os("PINGWHEEL_DEBUG").is_some()
}

// --- State ------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Shown {
    pub ping: Ping,
    pub name: String,
    pub age: u32,
}

#[derive(Default)]
pub struct State {
    pub pings: Vec<Shown>,
    pub sequence: i32,
    /// The connection we registered on (`remote` was set): reset when it goes.
    pub registered: bool,
}

thread_local! {
    pub static STATE: std::cell::RefCell<State> = std::cell::RefCell::new(State::default());
}

pub fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

impl State {
    /// A ping arrived: one per author (a new one replaces it), as Ping Wheel.
    pub fn add(&mut self, ping: Ping, name: String) {
        self.pings.retain(|s| s.ping.author != ping.author || ping.author.is_none());
        self.pings.push(Shown { ping, name, age: 0 });
    }
    pub fn tick(&mut self) {
        for s in &mut self.pings {
            s.age += 1;
        }
        self.pings.retain(|s| s.age < PING_DURATION_TICKS);
    }
    pub fn next_sequence(&mut self) -> i32 {
        self.sequence = self.sequence.wrapping_add(1);
        self.sequence
    }
}

// --- Projection -------------------------------------------------------------

/// Column-major 4x4 times a point.
fn mul(m: &[f32; 16], v: [f32; 4]) -> [f32; 4] {
    let mut o = [0.0; 4];
    for (row, o) in o.iter_mut().enumerate() {
        *o = (0..4).map(|c| m[c * 4 + row] * v[c]).sum();
    }
    o
}

/// Where `world` lands on a `size` (GUI pixels) screen, for a camera at
/// `camera` with camera-relative `view` and `projection`; None behind it.
pub fn project(
    world: [f64; 3],
    camera: [f64; 3],
    view: &[f32; 16],
    projection: &[f32; 16],
    size: (f32, f32),
) -> Option<[f32; 2]> {
    let rel = [
        (world[0] - camera[0]) as f32,
        (world[1] - camera[1]) as f32,
        (world[2] - camera[2]) as f32,
        1.0,
    ];
    let clip = mul(projection, mul(view, rel));
    if clip[3] <= 1e-4 {
        return None;
    }
    let (nx, ny) = (clip[0] / clip[3], clip[1] / clip[3]);
    Some([(nx + 1.0) * 0.5 * size.0, (1.0 - ny) * 0.5 * size.1])
}

/// The view direction for `yaw` and `pitch` in degrees (`Entity.calculateViewVector`).
pub fn view_vector(yaw: f32, pitch: f32) -> [f64; 3] {
    let (y, p) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()]
}

pub fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The label under a marker: who pinged and how far, `P5ina · 23 m`.
pub fn label(name: &str, metres: f64) -> String {
    if name.is_empty() {
        format!("{metres:.0} m")
    } else {
        format!("{name} · {metres:.0} m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_hashes_match_java() {
        // From jshell: "minecraft".hashCode(), "overworld".hashCode(); Identifier
        // hashes as 31 * namespace + path (26.3's bytecode).
        assert_eq!(java_string_hash("minecraft"), 695_073_197);
        assert_eq!(java_string_hash("overworld"), -745_159_874);
        assert_eq!(java_string_hash("Пинг"), "Пинг".encode_utf16().fold(0i32, |h, c| h * 31 + i32::from(c)));
        assert_eq!(
            dimension_hash("minecraft:overworld"),
            695_073_197i32.wrapping_mul(31).wrapping_add(-745_159_874)
        );
    }

    #[test]
    fn ping_round_trips_through_the_server_layout() {
        let p = Ping {
            channel: String::new(),
            pos: [1.5, 64.0, -3.25],
            entity: None,
            sequence: 7,
            dimension: dimension_hash("minecraft:overworld"),
            author: None,
        };
        // The server echoes the client's fields and appends the author.
        let mut s2c = encode_ping(&p);
        let author = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdefu128;
        s2c.extend_from_slice(&author.to_be_bytes());
        let back = decode_ping(&s2c).unwrap();
        assert_eq!(back.pos, p.pos);
        assert_eq!(back.sequence, 7);
        assert_eq!(back.author, Some(author));
        assert!(decode_ping(&s2c[..10]).is_none());
    }

    #[test]
    fn a_new_ping_replaces_the_authors_last_and_old_ones_expire() {
        let mut st = State::default();
        let p = |seq, a| Ping {
            channel: String::new(),
            pos: [0.0; 3],
            entity: None,
            sequence: seq,
            dimension: 0,
            author: Some(a),
        };
        st.add(p(1, 1), "a".into());
        st.add(p(2, 1), "a".into());
        st.add(p(3, 2), "b".into());
        assert_eq!(st.pings.len(), 2);
        for _ in 0..PING_DURATION_TICKS {
            st.tick();
        }
        assert!(st.pings.is_empty());
    }

    #[test]
    fn a_point_ahead_projects_to_the_centre() {
        // Identity view, a simple perspective looking down -Z.
        let id = [1.0, 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.];
        let proj = [1.0, 0., 0., 0., 0., 1., 0., 0., 0., 0., 0., -1., 0., 0., 0.05, 0.];
        let at = project([0.0, 0.0, -10.0], [0.0; 3], &id, &proj, (400.0, 200.0)).unwrap();
        assert!((at[0] - 200.0).abs() < 1e-3 && (at[1] - 100.0).abs() < 1e-3);
        assert!(project([0.0, 0.0, 10.0], [0.0; 3], &id, &proj, (400.0, 200.0)).is_none());
    }
}
