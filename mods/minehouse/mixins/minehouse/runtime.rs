// MineHouse for Horizon: the client half of MineHouse's server features, ported
// from the Fabric mod minehouse-client (minehouse-landing, `menu-mod/`): the
// anvil's real cost and the spawn zones' borders.
// This file is both the mod's Cargo library (its tests) and the module the
// bundle adds to the game.

pub const ID: &str = "minehouse";
pub const NAME: &str = "MineHouse";
pub const VERSION: &str = "0.1.0";
pub const DESCRIPTION: &str = "MineHouse's client features: the real anvil cost and the spawn zones' borders.";

/// `pro.minehouse.anvilclient.AnvilCostPayload`: varint container id, varint cost.
pub const ANVIL_COST: &str = "minehouse:anvil_cost";
/// `pro.minehouse.zonesclient.ZoneShapesPayload`.
pub const ZONE_SHAPES: &str = "minehouse:zone_shapes";
/// The channels the server mods look for: registering them is how they learn
/// the client draws these itself.
pub const CHANNELS: [&str; 2] = [ANVIL_COST, ZONE_SHAPES];
/// Fabric only sends a play channel to a client that registered it.
pub const REGISTER: &str = "minecraft:register";

/// `minecraft:register`'s body: channel names, NUL-separated.
pub fn encode_register(channels: &[&str]) -> Vec<u8> {
    channels.join("\0").into_bytes()
}

// --- Reading payloads (`FriendlyByteBuf`) ------------------------------------------

pub struct Reader<'a>(pub &'a [u8]);

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        if self.0.len() < n {
            return None;
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Some(a)
    }
    pub fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }
    pub fn bool(&mut self) -> Option<bool> {
        self.u8().map(|b| b != 0)
    }
    pub fn i32(&mut self) -> Option<i32> {
        self.take(4).map(|b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    pub fn varint(&mut self) -> Option<i32> {
        let mut value: u32 = 0;
        for i in 0..5 {
            let b = self.u8()?;
            value |= u32::from(b & 0x7F) << (7 * i);
            if b & 0x80 == 0 {
                return Some(value as i32);
            }
        }
        None
    }
    /// `readUtf(max)`: a varint byte length, then UTF-8.
    pub fn utf(&mut self, max_chars: usize) -> Option<String> {
        let len = usize::try_from(self.varint()?).ok()?;
        if len > max_chars * 3 {
            return None;
        }
        let s = std::str::from_utf8(self.take(len)?).ok()?;
        (s.chars().count() <= max_chars).then(|| s.to_string())
    }
}

pub fn write_varint(out: &mut Vec<u8>, v: i32) {
    let mut v = v as u32;
    loop {
        if v & !0x7F == 0 {
            out.push(v as u8);
            return;
        }
        out.push((v & 0x7F | 0x80) as u8);
        v >>= 7;
    }
}

pub fn write_utf(out: &mut Vec<u8>, s: &str) {
    write_varint(out, s.len() as i32);
    out.extend_from_slice(s.as_bytes());
}

// --- The anvil (`pro.minehouse.anvilclient`) --------------------------------------

/// Vanilla says "Too Expensive!" from 40; the server mod never puts more than 39
/// in the data slot, the real number comes in `minehouse:anvil_cost`.
pub const CLIENT_CAP: i32 = 40;

pub fn decode_anvil_cost(data: &[u8]) -> Option<(i32, i32)> {
    let mut r = Reader(data);
    Some((r.varint()?, r.varint()?))
}

/// What the server mod puts in the data slot for a real cost.
pub fn clamp(cost: i32) -> i32 {
    cost.min(CLIENT_CAP - 1)
}

/// `AnvilCost`: the last real cost the server sent for an anvil window. It is
/// trusted only while the window's data slot still shows what the server clamped
/// it to: right after an input changes, the client recomputes the cost by
/// vanilla rules, and until the server's answer arrives that is what shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnvilCost {
    pub container: Option<i32>,
    pub cost: i32,
}

impl AnvilCost {
    /// The real cost for `container` whose slot shows `slot_cost`, or None.
    pub fn real(&self, container: Option<i32>, slot_cost: i32) -> Option<i32> {
        (self.container.is_some() && self.container == container && slot_cost > 0 && clamp(self.cost) == slot_cost)
            .then_some(self.cost)
    }
}

// --- Spawn zones (`pro.minehouse.zonesclient`) ------------------------------------

/// One polygon. `draft`: the admin's unsaved markup (vertices numbered, open
/// while it has fewer than 3). `rgb` is 0xRRGGBB.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shape {
    pub rgb: u32,
    pub draft: bool,
    pub y_min: i32,
    pub y_max: i32,
    pub label: String,
    pub xs: Vec<i32>,
    pub zs: Vec<i32>,
}

/// `ZoneShapesPayload.read`.
pub fn decode_zone_shapes(data: &[u8]) -> Option<Vec<Shape>> {
    let mut r = Reader(data);
    let count = usize::try_from(r.varint()?).ok()?.min(256);
    let mut shapes = Vec::with_capacity(count);
    for _ in 0..count {
        let rgb = r.i32()? as u32 & 0xFF_FFFF;
        let draft = r.bool()?;
        let y_min = r.varint()?;
        let y_max = r.varint()?;
        let label = r.utf(128)?;
        let n = usize::try_from(r.varint()?).ok()?.min(4096);
        let (mut xs, mut zs) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for _ in 0..n {
            xs.push(r.varint()?);
            zs.push(r.varint()?);
        }
        shapes.push(Shape { rgb, draft, y_min, y_max, label, xs, zs });
    }
    Some(shapes)
}

/// Edges farther than this from the player are not drawn.
pub const RANGE: f64 = 160.0;
const WALL_ALPHA: u32 = 0x38;
const GRID_ALPHA: u32 = 0x70;

/// What `ZoneRenderer` draws, as world primitives.
#[derive(Clone, Debug, PartialEq)]
pub enum Draw {
    Line { from: [f64; 3], to: [f64; 3], color: u32, width: f32 },
    /// A translucent wall from `from` to `to` (bottom corners), `height` blocks up.
    Wall { from: [f64; 3], to: [f64; 3], height: f64, color: u32 },
    Point { at: [f64; 3], color: u32, size: f32 },
    Text { at: [f64; 3], text: String, color: u32, scale: f32 },
}

pub fn ceil_to_ten(y: f64) -> i32 {
    ((y / 10.0).ceil() * 10.0) as i32
}

/// Middle of the vertices (good enough for a label inside a convex-ish zone).
pub fn centre(xs: &[i32], zs: &[i32]) -> [f64; 2] {
    let n = xs.len().max(1) as f64;
    let x: f64 = xs.iter().map(|&x| f64::from(x) + 0.5).sum();
    let z: f64 = zs.iter().map(|&z| f64::from(z) + 0.5).sum();
    [x / n, z / n]
}

pub fn distance_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let v = [b[0] - a[0], b[1] - a[1]];
    let len2 = v[0] * v[0] + v[1] * v[1];
    let t = if len2 == 0.0 { 0.0 } else { (((p[0] - a[0]) * v[0] + (p[1] - a[1]) * v[1]) / len2).clamp(0.0, 1.0) };
    let d = [p[0] - (a[0] + t * v[0]), p[1] - (a[1] + t * v[1])];
    (d[0] * d[0] + d[1] * d[1]).sqrt()
}

/// `ZoneRenderer.draw` for one shape, `player` the player's position.
pub fn draw_shape(s: &Shape, player: [f64; 3], out: &mut Vec<Draw>) {
    let n = s.xs.len().min(s.zs.len());
    if n == 0 {
        return;
    }
    let line = 0xFF00_0000 | s.rgb;
    let grid = GRID_ALPHA << 24 | s.rgb;
    let wall = WALL_ALPHA << 24 | s.rgb;
    let (lo, hi) = (f64::from(s.y_min), f64::from(s.y_max) + 1.0);
    let width = if s.draft { 4.0 } else { 3.0 };
    let closed = n >= 3;
    let edges = if closed { n } else { n - 1 };
    let vertex = |i: usize| [f64::from(s.xs[i % n]) + 0.5, f64::from(s.zs[i % n]) + 0.5];
    for i in 0..edges {
        let (a, b) = (vertex(i), vertex(i + 1));
        if distance_to_segment([player[0], player[2]], a, b) > RANGE {
            continue;
        }
        let (a0, b0) = ([a[0], lo, a[1]], [b[0], lo, b[1]]);
        let (a1, b1) = ([a[0], hi, a[1]], [b[0], hi, b[1]]);
        out.push(Draw::Wall { from: a0, to: b0, height: hi - lo, color: wall });
        out.push(Draw::Line { from: a0, to: b0, color: line, width });
        out.push(Draw::Line { from: a1, to: b1, color: line, width });
        let mut y = ceil_to_ten(lo + 1.0);
        while f64::from(y) < hi {
            let y64 = f64::from(y);
            out.push(Draw::Line { from: [a[0], y64, a[1]], to: [b[0], y64, b[1]], color: grid, width: 1.5 });
            y += 10;
        }
        // The edge at the player's feet, so it is easy to see where the border runs on the ground.
        let feet = player[1].floor() + 0.05;
        if feet > lo && feet < hi {
            out.push(Draw::Line { from: [a[0], feet, a[1]], to: [b[0], feet, b[1]], color: line, width });
        }
    }
    for i in 0..n {
        let [x, z] = vertex(i);
        let (dx, dz) = (x - player[0], z - player[2]);
        if dx * dx + dz * dz > RANGE * RANGE {
            continue;
        }
        out.push(Draw::Line { from: [x, lo, z], to: [x, hi, z], color: line, width: if s.draft { 5.0 } else { width } });
        if s.draft {
            let y = (player[1] + 1.6).clamp(lo, hi);
            out.push(Draw::Point { at: [x, y, z], color: 0xFFFF_FFFF, size: 12.0 });
            out.push(Draw::Text { at: [x, y + 0.8, z], text: (i + 1).to_string(), color: 0xFFFF_E066, scale: 1.6 });
        }
    }
    if !s.label.is_empty() {
        let c = centre(&s.xs[..n], &s.zs[..n]);
        let (dx, dz) = (c[0] - player[0], c[1] - player[2]);
        if dx * dx + dz * dz <= RANGE * RANGE {
            let y = (player[1] + 4.0).clamp(lo, hi);
            out.push(Draw::Text { at: [c[0], y, c[1]], text: s.label.clone(), color: line, scale: 3.0 });
        }
    }
}

pub fn draw_shapes(shapes: &[Shape], player: [f64; 3]) -> Vec<Draw> {
    let mut out = Vec::new();
    for s in shapes {
        draw_shape(s, player, &mut out);
    }
    out
}

/// A wall as boxes a few hundredths of a block thin: one along an axis, else
/// one box per block of the edge's length (the primitives have no free quad).
pub fn wall_boxes(from: [f64; 3], to: [f64; 3], height: f64) -> Vec<([f64; 3], [f64; 3])> {
    const T: f64 = 0.02;
    let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
    let top = from[1] + height;
    if dx.abs() < 1e-9 || dz.abs() < 1e-9 {
        let min = [from[0].min(to[0]) - T, from[1], from[2].min(to[2]) - T];
        let max = [from[0].max(to[0]) + T, top, from[2].max(to[2]) + T];
        return vec![(min, max)];
    }
    let len = (dx * dx + dz * dz).sqrt();
    let steps = (len.ceil() as usize).clamp(1, 512);
    (0..steps)
        .map(|i| {
            let (t0, t1) = (i as f64 / steps as f64, (i + 1) as f64 / steps as f64);
            let (x0, z0) = (from[0] + dx * t0, from[2] + dz * t0);
            let (x1, z1) = (from[0] + dx * t1, from[2] + dz * t1);
            ([x0.min(x1), from[1], z0.min(z1)], [x0.max(x1), top, z0.max(z1)])
        })
        .collect()
}

// --- State ---------------------------------------------------------------------

#[derive(Default)]
pub struct State {
    /// The connection we registered our channels on: reset when it goes, or
    /// when a proxy moves us to another server (a new `Login`).
    pub registered: bool,
    /// The anvil window the server last opened (`OpenScreen`), the real cost it
    /// sent, and whether the screen being drawn now shows that real cost.
    pub open_container: Option<i32>,
    pub anvil: AnvilCost,
    pub drawing_real: bool,
    pub shapes: Vec<Shape>,
}

impl State {
    pub fn left_server(&mut self) {
        self.registered = false;
        self.open_container = None;
        self.anvil = AnvilCost::default();
        self.drawing_real = false;
        self.shapes.clear();
    }
}

thread_local! {
    pub static STATE: std::cell::RefCell<State> = std::cell::RefCell::new(State::default());
}

pub fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_round_trip() {
        for v in [0, 1, 127, 128, 300, 2_097_151, -1, i32::MAX, i32::MIN] {
            let mut out = Vec::new();
            write_varint(&mut out, v);
            assert_eq!(Reader(&out).varint(), Some(v));
        }
    }

    #[test]
    fn anvil_cost_only_while_the_slot_shows_the_clamp() {
        let mut out = Vec::new();
        write_varint(&mut out, 7);
        write_varint(&mut out, 55);
        let (container, cost) = decode_anvil_cost(&out).unwrap();
        let a = AnvilCost { container: Some(container), cost };
        assert_eq!(a.real(Some(7), 39), Some(55));
        // The client recomputed after an input changed: vanilla's number shows.
        assert_eq!(a.real(Some(7), 12), None);
        assert_eq!(a.real(Some(8), 39), None);
        assert_eq!(a.real(None, 39), None);
        let small = AnvilCost { container: Some(7), cost: 5 };
        assert_eq!(small.real(Some(7), 5), Some(5));
        assert_eq!(small.real(Some(7), 0), None);
    }

    fn encode_shapes(shapes: &[Shape]) -> Vec<u8> {
        let mut out = Vec::new();
        write_varint(&mut out, shapes.len() as i32);
        for s in shapes {
            out.extend_from_slice(&(s.rgb as i32).to_be_bytes());
            out.push(u8::from(s.draft));
            write_varint(&mut out, s.y_min);
            write_varint(&mut out, s.y_max);
            write_utf(&mut out, &s.label);
            write_varint(&mut out, s.xs.len() as i32);
            for (x, z) in s.xs.iter().zip(&s.zs) {
                write_varint(&mut out, *x);
                write_varint(&mut out, *z);
            }
        }
        out
    }

    fn square() -> Shape {
        Shape {
            rgb: 0x55AAFF,
            draft: false,
            y_min: 60,
            y_max: 80,
            label: "Зона 1".into(),
            xs: vec![0, 10, 10, 0],
            zs: vec![0, 0, 10, 10],
        }
    }

    #[test]
    fn zone_shapes_decode() {
        let draft = Shape { draft: true, label: String::new(), xs: vec![-5, 3], zs: vec![-100, 7], ..square() };
        let shapes = vec![square(), draft];
        assert_eq!(decode_zone_shapes(&encode_shapes(&shapes)), Some(shapes));
        assert_eq!(decode_zone_shapes(&[0]), Some(vec![]));
        assert_eq!(decode_zone_shapes(&[1, 0]), None);
    }

    #[test]
    fn zone_drawing() {
        let d = draw_shapes(&[square()], [5.0, 70.0, 5.0]);
        let walls = d.iter().filter(|d| matches!(d, Draw::Wall { .. })).count();
        assert_eq!(walls, 4);
        // A line every 10 blocks between 61 and 81: at 70 and 80, per edge.
        let grid = d.iter().filter(|d| matches!(d, Draw::Line { width, .. } if *width == 1.5)).count();
        assert_eq!(grid, 8);
        assert!(d.iter().any(|d| matches!(d, Draw::Text { text, scale, .. } if text == "Зона 1" && *scale == 3.0)));
        // Too far: nothing.
        assert!(draw_shapes(&[square()], [1000.0, 70.0, 1000.0]).is_empty());
        // An open draft of two corners: one edge, numbered corners.
        let draft = Shape { draft: true, label: String::new(), xs: vec![0, 10], zs: vec![0, 0], ..square() };
        let d = draw_shapes(&[draft], [5.0, 70.0, 5.0]);
        assert_eq!(d.iter().filter(|d| matches!(d, Draw::Wall { .. })).count(), 1);
        assert!(d.iter().any(|d| matches!(d, Draw::Text { text, .. } if text == "2")));
    }

    #[test]
    fn walls() {
        assert_eq!(wall_boxes([0.5, 60.0, 0.5], [10.5, 60.0, 0.5], 21.0).len(), 1);
        let diagonal = wall_boxes([0.5, 60.0, 0.5], [10.5, 60.0, 10.5], 21.0);
        assert_eq!(diagonal.len(), 15);
        assert!(diagonal.iter().all(|(min, max)| max[1] - min[1] == 21.0));
    }

    #[test]
    fn register_body() {
        assert_eq!(encode_register(&CHANNELS), b"minehouse:anvil_cost\0minehouse:zone_shapes".to_vec());
    }
}
