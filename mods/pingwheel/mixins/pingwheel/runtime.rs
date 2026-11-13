// Ping Wheel for Horizon: a port of the Fabric mod Ping Wheel 1.12
// (https://github.com/LukenSkyne/Minecraft-Ping-Wheel, MIT): the same protocol,
// pings, labels, direction indicator, sound, settings and /pingwheel commands,
// so a Horizon player pings with and sees the pings of Java players.
// This file is both the mod's Cargo library (its tests) and the module the
// bundle adds to the game.

pub const ID: &str = "pingwheel";
pub const NAME: &str = "Ping Wheel";
pub const VERSION: &str = "1.12.2";
pub const DESCRIPTION: &str = "Mark locations similar to the ping mechanism found in modern co-op multiplayer games.";

/// The keys, as Ping Wheel's: Ping Location on the sixth mouse button, Open Settings unbound.
pub const KEY_PING: &str = "ping_location";
pub const DEFAULT_PING: &str = "key.mouse.6";
pub const KEY_SETTINGS: &str = "open_settings";
pub const DEFAULT_SETTINGS: &str = "key.keyboard.unknown";
pub const PICK_ITEM: &str = "key.mouse.middle";
pub const PLAYER_LIST: &str = "key.keyboard.tab";

pub const C2S_PING: &str = "ping-wheel-c2s:ping-location";
pub const C2S_CHANNEL: &str = "ping-wheel-c2s:update-channel";
pub const S2C_PING: &str = "ping-wheel-s2c:ping-location";
/// Fabric only sends a play channel to a client that registered it.
pub const REGISTER: &str = "minecraft:register";
pub const MAX_CHANNEL_LENGTH: usize = 128;

pub const TPS: f32 = 20.0;
pub const MAX_PING_DURATION: i32 = 60;
pub const MAX_PING_DISTANCE: i32 = 2048;
pub const MAX_CORRECTION_PERIOD: f32 = 5.0;
pub const WHITE: u32 = 0xFFFF_FFFF;
pub const SHADOW_BLACK: u32 = 0x4000_0000;

// --- Settings (`ClientConfig`) ------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerInfoMode {
    Hold,
    Disabled,
    Always,
    Compact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamColorMode {
    Full,
    Disabled,
    PingOnly,
    LabelsOnly,
}

impl PlayerInfoMode {
    const ALL: [Self; 4] = [Self::Hold, Self::Disabled, Self::Always, Self::Compact];
    pub fn name(self) -> &'static str {
        ["hold", "disabled", "always", "compact"][self as usize]
    }
    pub fn label(self) -> &'static str {
        ["Hold", "Disabled", "Always", "Compact"][self as usize]
    }
}

impl TeamColorMode {
    const ALL: [Self; 4] = [Self::Full, Self::Disabled, Self::PingOnly, Self::LabelsOnly];
    pub fn name(self) -> &'static str {
        ["full", "disabled", "ping_only", "labels_only"][self as usize]
    }
    pub fn label(self) -> &'static str {
        ["Full", "Disabled", "Ping Only", "Labels Only"][self as usize]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub ping_volume: i32,
    pub ping_duration: i32,
    pub ping_distance: i32,
    pub correction_period: f32,
    pub item_icon_visible: bool,
    pub direction_indicator_visible: bool,
    pub player_info_mode: PlayerInfoMode,
    pub team_color_mode: TeamColorMode,
    pub ping_size: i32,
    // Hidden from the settings screen, as in Ping Wheel.
    pub remove_radius: f32,
    pub raycast_distance: f64,
    pub safe_zone: [f32; 4], // left, right, top, bottom
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ping_volume: 100,
            ping_duration: 7,
            ping_distance: 2048,
            correction_period: 1.0,
            item_icon_visible: true,
            direction_indicator_visible: true,
            player_info_mode: PlayerInfoMode::Hold,
            team_color_mode: TeamColorMode::Full,
            ping_size: 100,
            remove_radius: 10.0,
            raycast_distance: 1024.0,
            safe_zone: [5.0, 5.0, 5.0, 60.0],
        }
    }
}

impl Config {
    pub fn load(get: &dyn Fn(&str) -> Option<String>) -> Self {
        let d = Self::default();
        let int = |k: &str, lo: i32, hi: i32, dv: i32| get(k).and_then(|v| v.trim().parse().ok()).map_or(dv, |v: i32| v.clamp(lo, hi));
        Self {
            ping_volume: int("ping_volume", 0, 100, d.ping_volume),
            ping_duration: int("ping_duration", 1, MAX_PING_DURATION, d.ping_duration),
            ping_distance: int("ping_distance", 0, MAX_PING_DISTANCE, d.ping_distance),
            correction_period: get("correction_period")
                .and_then(|v| v.trim().parse::<f32>().ok())
                .map_or(d.correction_period, |v| v.clamp(0.1, MAX_CORRECTION_PERIOD)),
            item_icon_visible: get("item_icon_visible").is_none_or(|v| v.trim() != "false"),
            direction_indicator_visible: get("direction_indicator_visible").is_none_or(|v| v.trim() != "false"),
            player_info_mode: get("player_info_mode")
                .and_then(|v| PlayerInfoMode::ALL.into_iter().find(|m| m.name() == v.trim()))
                .unwrap_or(d.player_info_mode),
            team_color_mode: get("team_color_mode")
                .and_then(|v| TeamColorMode::ALL.into_iter().find(|m| m.name() == v.trim()))
                .unwrap_or(d.team_color_mode),
            ping_size: int("ping_size", 40, 300, d.ping_size),
            ..d
        }
    }

    pub fn stored(&self) -> Vec<(&'static str, String)> {
        vec![
            ("ping_volume", self.ping_volume.to_string()),
            ("ping_duration", self.ping_duration.to_string()),
            ("ping_distance", self.ping_distance.to_string()),
            ("correction_period", format!("{:.1}", self.correction_period)),
            ("item_icon_visible", self.item_icon_visible.to_string()),
            ("direction_indicator_visible", self.direction_indicator_visible.to_string()),
            ("player_info_mode", self.player_info_mode.name().into()),
            ("team_color_mode", self.team_color_mode.name().into()),
            ("ping_size", self.ping_size.to_string()),
        ]
    }

    /// The settings screen's rows, as Ping Wheel's `SettingsScreen` labels them.
    pub fn buttons(&self, channel: &str) -> Vec<String> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!(
                "Ping Volume: {}",
                if self.ping_volume == 0 { "OFF".into() } else { format!("{}%", self.ping_volume) }
            ),
            format!(
                "Ping Duration: {}",
                if self.ping_duration >= MAX_PING_DURATION { "∞".into() } else { format!("{}s", self.ping_duration) }
            ),
            format!(
                "Ping Distance: {}",
                match self.ping_distance {
                    0 => "Hidden".into(),
                    d if d >= MAX_PING_DISTANCE => "∞".into(),
                    d => format!("{d}m"),
                }
            ),
            format!(
                "Correction Period: {}",
                if self.correction_period >= MAX_CORRECTION_PERIOD {
                    "∞".into()
                } else {
                    format!("{:.1}s", self.correction_period)
                }
            ),
            format!("Item Icons: {}", on(self.item_icon_visible)),
            format!("Direction Indicator: {}", on(self.direction_indicator_visible)),
            format!("Player Info: {}", self.player_info_mode.label()),
            format!("Team Color: {}", self.team_color_mode.label()),
            format!("Ping Size: {}%", self.ping_size),
            format!(
                "Ping Channel: {}",
                if channel.is_empty() { "Global".to_owned() } else { format!("\"{channel}\"") }
            ),
        ]
    }

    /// A click on row `i` steps its value (shift: back), as the sliders' arrow keys do.
    pub fn click(&mut self, i: usize, back: bool) {
        fn step<T: Copy + PartialEq>(list: &[T], v: T, back: bool) -> T {
            let n = list.len();
            let at = list.iter().position(|x| *x == v).unwrap_or(0);
            list[if back { (at + n - 1) % n } else { (at + 1) % n }]
        }
        match i {
            0 => self.ping_volume = step(&[0, 10, 25, 50, 75, 100], self.ping_volume, back),
            1 => self.ping_duration = step(&[1, 3, 5, 7, 10, 15, 20, 30, 45, 60], self.ping_duration, back),
            2 => self.ping_distance = step(&[0, 64, 128, 256, 512, 1024, 2048], self.ping_distance, back),
            3 => {
                let v = (self.correction_period * 10.0).round() as i32;
                self.correction_period = step(&[1, 5, 10, 20, 30, 50], v, back) as f32 / 10.0;
            }
            4 => self.item_icon_visible = !self.item_icon_visible,
            5 => self.direction_indicator_visible = !self.direction_indicator_visible,
            6 => self.player_info_mode = step(&PlayerInfoMode::ALL, self.player_info_mode, back),
            7 => self.team_color_mode = step(&TeamColorMode::ALL, self.team_color_mode, back),
            8 => self.ping_size = step(&[40, 60, 80, 100, 120, 150, 200, 250, 300], self.ping_size, back),
            _ => {}
        }
    }
}

// --- Java's encodings -------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct Ping {
    pub channel: String,
    pub pos: [f64; 3],
    pub entity: Option<u128>,
    pub sequence: i32,
    pub dimension: i32,
    pub author: Option<u128>,
}

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
    let s: String = s.chars().take(MAX_CHANNEL_LENGTH).collect();
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
    s.encode_utf16().fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(i32::from(c)))
}

/// `Identifier.hashCode`: `31 * namespace.hashCode() + path.hashCode()`.
/// Ping Wheel tells dimensions apart by it.
pub fn dimension_hash(id: &str) -> i32 {
    let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
    java_string_hash(ns).wrapping_mul(31).wrapping_add(java_string_hash(path))
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
    Some(Ping { channel, pos, entity, sequence, dimension, author })
}

// --- State (`PingManager`, `PingController`) ------------------------------------

#[derive(Clone, Debug)]
pub struct Shown {
    pub ping: Ping,
    pub name: String,
    pub spawn_tick: Option<i64>,
    pub age: i64,
    /// This frame: where on screen (GUI pixels), and whether behind the camera.
    pub screen: Option<([f32; 2], bool)>,
    pub distance: f64,
    /// The entity it follows (an entity ping), and its item when it is one.
    pub entity_id: Option<u32>,
    pub item: Option<String>,
}

impl Shown {
    pub fn expired(&self, c: &Config) -> bool {
        c.ping_duration < MAX_PING_DURATION && self.age as f32 > c.ping_duration as f32 * TPS
    }
    pub fn removable(&self, c: &Config, size: (f32, f32)) -> bool {
        (c.correction_period >= MAX_CORRECTION_PERIOD || self.age as f32 > c.correction_period * TPS)
            && self.distance_to_center(size) < c.remove_radius
    }
    pub fn distance_to_center(&self, (w, h): (f32, f32)) -> f32 {
        match self.screen {
            Some(([x, y], _)) => ((x - w * 0.5).powi(2) + (y - h * 0.5).powi(2)).sqrt(),
            None => 0.0,
        }
    }
    /// `PingView.calculateScale`.
    pub fn scale(&self, c: &Config) -> f32 {
        let s = 2.0 / self.distance.max(1e-6).powf(0.3);
        (s.max(1.0) as f32) * 0.5 * (c.ping_size as f32 / 100.0)
    }
}

#[derive(Default)]
pub struct State {
    pub loaded: bool,
    pub config: Config,
    pub pings: Vec<Shown>,
    pub sequence: i32,
    pub last_ping_tick: i64,
    pub queued: bool,
    /// The connection we registered on: reset when it goes.
    pub registered: bool,
    /// The server this is (`host:port`), for its channel.
    pub server: String,
    pub channel: String,
    pub player_list_held: bool,
    pub open_settings: bool,
    pub store: Vec<(String, String)>,
}

thread_local! {
    pub static STATE: std::cell::RefCell<State> = std::cell::RefCell::new(State::default());
}

pub fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// The settings key a server's channel is kept under (Ping Wheel's `serverChannels`).
pub fn channel_key(server: &str) -> String {
    let clean: String = server
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
        .collect();
    format!("channel.{clean}")
}

impl State {
    pub fn load(&mut self, get: &dyn Fn(&str) -> Option<String>) {
        if !self.loaded {
            self.loaded = true;
            self.config = Config::load(get);
        }
    }

    pub fn save(&mut self) {
        let mut out: Vec<(String, String)> = self.config.stored().into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
        out.push((channel_key(&self.server), self.channel.clone()));
        self.store.extend(out);
    }

    /// `PingManager.acceptPingPacket`: on our channel, within reach.
    pub fn accept(&mut self, ping: Ping, name: String, player: [f64; 3]) -> bool {
        if ping.channel != self.channel {
            return false;
        }
        if self.config.ping_distance < MAX_PING_DISTANCE && distance(player, ping.pos) > f64::from(self.config.ping_distance) {
            return false;
        }
        let shown = Shown { ping, name, spawn_tick: None, age: 0, screen: None, distance: 0.0, entity_id: None, item: None };
        match self
            .pings
            .iter_mut()
            .find(|s| s.ping.author == shown.ping.author && s.ping.sequence == shown.ping.sequence)
        {
            Some(slot) => *slot = shown,
            None => self.pings.push(shown),
        }
        true
    }

    /// `PingManager.updatePings`: age them, drop the expired, and when a ping is
    /// queued, the removable one nearest the centre (pinging it again removes it).
    /// True when that took the queued ping.
    pub fn update(&mut self, game_tick: i64, size: (f32, f32)) -> bool {
        let c = self.config.clone();
        for p in &mut self.pings {
            let spawn = *p.spawn_tick.get_or_insert(game_tick);
            p.age = game_tick - spawn;
        }
        self.pings.retain(|p| !p.expired(&c));
        let mut took = false;
        if self.queued {
            let target = self
                .pings
                .iter()
                .enumerate()
                .filter(|(_, p)| p.removable(&c, size))
                .min_by(|a, b| a.1.distance_to_center(size).total_cmp(&b.1.distance_to_center(size)))
                .map(|(i, _)| i);
            if let Some(i) = target {
                self.pings.remove(i);
                self.queued = false;
                took = true;
            }
        }
        self.pings.sort_by(|a, b| b.distance.total_cmp(&a.distance));
        took
    }

    /// `PingController.pollPingAction`: the sequence a new ping gets (a new one
    /// after the correction period, so a quick second ping corrects the first).
    pub fn next_sequence(&mut self, game_tick: i64) -> i32 {
        let c = &self.config;
        if c.correction_period < MAX_CORRECTION_PERIOD
            && (game_tick - self.last_ping_tick) as f32 > c.correction_period * TPS
        {
            self.sequence = self.sequence.wrapping_add(1);
        }
        self.last_ping_tick = game_tick;
        self.sequence
    }
}

// --- Geometry ------------------------------------------------------------------------

fn mul(m: &[f32; 16], v: [f32; 4]) -> [f32; 4] {
    let mut o = [0.0; 4];
    for (row, o) in o.iter_mut().enumerate() {
        *o = (0..4).map(|c| m[c * 4 + row] * v[c]).sum();
    }
    o
}

/// `MathUtils.worldToScreen`: GUI pixels, and whether the point is behind the
/// camera (its depth negative; the position is then mirrored, as Ping Wheel's).
pub fn project(
    world: [f64; 3],
    camera: [f64; 3],
    view: &[f32; 16],
    projection: &[f32; 16],
    size: (f32, f32),
) -> Option<([f32; 2], bool)> {
    let rel = [(world[0] - camera[0]) as f32, (world[1] - camera[1]) as f32, (world[2] - camera[2]) as f32, 1.0];
    let clip = mul(projection, mul(view, rel));
    let w = clip[3];
    if w.abs() < 1e-6 {
        return None;
    }
    let (nx, ny) = (clip[0] / w, clip[1] / w);
    Some(([size.0 * (0.5 + nx * 0.5), size.1 * (0.5 - ny * 0.5)], w < 0.0))
}

/// `MathUtils.calculateAngleRectIntersection`: where a ray from the safe
/// zone's centre at `angle` leaves it.
pub fn angle_rect_intersection(angle: f32, lt: [f32; 2], rb: [f32; 2]) -> [f32; 2] {
    let (w, h) = (rb[0] - lt[0], rb[1] - lt[1]);
    let (dx, dy) = (angle.cos(), angle.sin());
    let (sx, sy) = (dx / w, dy / h);
    let (cx, cy) = (w * 0.5, h * 0.5);
    if sx.abs() < sy.abs() {
        if sy < 0.0 {
            let t = -cy / dy;
            [cx + t * dx + lt[0], lt[1]]
        } else {
            let t = cy / dy;
            [cx + t * dx + lt[0], rb[1]]
        }
    } else if sx < 0.0 {
        let t = -cx / dx;
        [lt[0], cy + t * dy + lt[1]]
    } else {
        let t = cx / dx;
        [rb[0], cy + t * dy + lt[1]]
    }
}

/// `Entity.calculateViewVector`.
pub fn view_vector(yaw: f32, pitch: f32) -> [f64; 3] {
    let (y, p) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()]
}

pub fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Where a ray from `o` along `d` first enters the box, as a distance along it (`AABB.clip`).
pub fn ray_box(o: [f64; 3], d: [f64; 3], min: [f64; 3], max: [f64; 3]) -> Option<f64> {
    let (mut t0, mut t1) = (0.0f64, f64::INFINITY);
    for i in 0..3 {
        if d[i].abs() < 1e-12 {
            if o[i] < min[i] || o[i] > max[i] {
                return None;
            }
        } else {
            let (a, b) = ((min[i] - o[i]) / d[i], (max[i] - o[i]) / d[i]);
            t0 = t0.max(a.min(b));
            t1 = t1.min(a.max(b));
        }
    }
    (t0 <= t1).then_some(t0)
}

/// The distance label: `"%,.1f"` and the metre unit, `1,234.5m`.
pub fn distance_text(m: f64) -> String {
    let s = format!("{m:.1}");
    let (int, frac) = s.split_once('.').unwrap_or((&s, "0"));
    let digits: Vec<char> = int.chars().collect();
    let mut grouped = String::new();
    for (i, c) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(*c);
    }
    format!("{grouped}.{frac}m")
}

/// The ping sound: Ping Wheel's `ping.ogg`, as 48 kHz mono samples.
pub fn ping_sound(pcm_s16le: &[u8], volume: f32) -> Vec<f32> {
    pcm_s16le
        .chunks_exact(2)
        .map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0 * volume)
        .collect()
}

// --- Commands (`ClientCommandBuilder`) ---------------------------------------------

pub enum Command {
    Help,
    Config,
    Channel(Option<String>),
}

pub fn parse_command(line: &str) -> Option<Command> {
    let line = line.trim();
    let rest = line.strip_prefix("pingwheel")?;
    if !rest.is_empty() && !rest.starts_with(' ') {
        return None;
    }
    let rest = rest.trim();
    Some(match rest.split_once(' ').map_or((rest, ""), |(a, b)| (a, b.trim())) {
        ("" | "help", _) => Command::Help,
        ("config", _) => Command::Config,
        ("channel", "") => Command::Channel(None),
        ("channel", name) => Command::Channel(Some(name.trim_matches('"').to_owned())),
        _ => Command::Help,
    })
}

pub const HELP: [&str; 6] = [
    "/pingwheel config",
    "  open configuration interface",
    "/pingwheel channel",
    "  get your current channel",
    "/pingwheel channel <channel_name>",
    "  set your channel, use \"\" for default channel",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_hashes_match_java() {
        // From jshell: "minecraft".hashCode(), "overworld".hashCode(); Identifier
        // hashes as 31 * namespace + path (26.3's bytecode).
        assert_eq!(java_string_hash("minecraft"), 695_073_197);
        assert_eq!(java_string_hash("overworld"), -745_159_874);
        assert_eq!(dimension_hash("minecraft:overworld"), 695_073_197i32.wrapping_mul(31).wrapping_add(-745_159_874));
    }

    #[test]
    fn ping_round_trips_through_the_server_layout() {
        let p = Ping { channel: String::new(), pos: [1.5, 64.0, -3.25], entity: Some(9), sequence: 7, dimension: 3, author: None };
        let mut s2c = encode_ping(&p);
        let author = 0x0123_4567_89ab_cdef_0123_4567_89ab_cdefu128;
        s2c.extend_from_slice(&author.to_be_bytes());
        let back = decode_ping(&s2c).unwrap();
        assert_eq!((back.pos, back.entity, back.sequence, back.author), (p.pos, Some(9), 7, Some(author)));
        assert!(decode_ping(&s2c[..10]).is_none());
    }

    fn ping(seq: i32, author: u128, channel: &str) -> Ping {
        Ping { channel: channel.into(), pos: [0.0; 3], entity: None, sequence: seq, dimension: 0, author: Some(author) }
    }

    #[test]
    fn pings_are_replaced_by_sequence_filtered_by_channel_and_expire() {
        let mut st = State::default();
        assert!(st.accept(ping(1, 1, ""), "a".into(), [0.0; 3]));
        assert!(st.accept(ping(1, 1, ""), "a".into(), [0.0; 3]));
        assert!(st.accept(ping(2, 1, ""), "a".into(), [0.0; 3]));
        assert!(!st.accept(ping(3, 1, "team"), "a".into(), [0.0; 3]));
        assert_eq!(st.pings.len(), 2);
        st.update(100, (400.0, 200.0));
        st.update(100 + 7 * 20 + 1, (400.0, 200.0));
        assert!(st.pings.is_empty());
    }

    #[test]
    fn a_second_ping_inside_the_correction_period_keeps_the_sequence() {
        let mut st = State::default();
        let a = st.next_sequence(1000);
        assert_eq!(st.next_sequence(1010), a);
        assert_eq!(st.next_sequence(1100), a + 1);
    }

    #[test]
    fn pinging_a_ping_near_the_centre_removes_it() {
        let mut st = State::default();
        st.accept(ping(1, 1, ""), "a".into(), [0.0; 3]);
        st.update(0, (400.0, 200.0));
        st.pings[0].screen = Some(([201.0, 101.0], false));
        st.queued = true;
        assert!(st.update(40, (400.0, 200.0)));
        assert!(st.pings.is_empty() && !st.queued);
    }

    #[test]
    fn labels_and_edges_follow_ping_wheel() {
        assert_eq!(distance_text(1234.56), "1,234.6m");
        assert_eq!(distance_text(5.0), "5.0m");
        // Straight right and straight up leave the box at its edges.
        let e = angle_rect_intersection(0.0, [5.0, 5.0], [395.0, 140.0]);
        assert!((e[0] - 395.0).abs() < 1e-3 && (e[1] - 72.5).abs() < 1e-3);
        let e = angle_rect_intersection(-std::f32::consts::FRAC_PI_2, [5.0, 5.0], [395.0, 140.0]);
        assert!((e[1] - 5.0).abs() < 1e-3);
        assert_eq!(ray_box([0.0, 0.5, 0.5], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [3.0, 1.0, 1.0]), Some(2.0));
    }

    #[test]
    fn settings_round_trip_and_commands_parse() {
        let mut c = Config::default();
        c.click(0, false);
        c.click(6, false);
        let stored: std::collections::HashMap<_, _> = c.stored().into_iter().collect();
        let back = Config::load(&|k| stored.get(k).cloned());
        assert_eq!(back, c);
        assert!(matches!(parse_command("pingwheel channel \"red\""), Some(Command::Channel(Some(n))) if n == "red"));
        assert!(matches!(parse_command("pingwheel config"), Some(Command::Config)));
        assert!(parse_command("pingwheelx").is_none());
    }
}
