// MineHouse for Horizon: the client half of MineHouse's server features, ported
// from the Fabric mod minehouse-client (minehouse-landing, `menu-mod/`): the
// anvil's real cost, the spawn zones' borders and the wardrobe.
// This file is both the mod's Cargo library (its tests) and the module the
// bundle adds to the game.

pub const ID: &str = "minehouse";
pub const NAME: &str = "MineHouse";
pub const VERSION: &str = "0.2.1";
pub const DESCRIPTION: &str = "MineHouse's client features: the real anvil cost, the spawn zones' borders and the wardrobe (O).";

/// `pro.minehouse.anvilclient.AnvilCostPayload`: varint container id, varint cost.
pub const ANVIL_COST: &str = "minehouse:anvil_cost";
/// `pro.minehouse.zonesclient.ZoneShapesPayload`.
pub const ZONE_SHAPES: &str = "minehouse:zone_shapes";
/// `pro.minehouse.menu.wardrobe.WardrobeNet`: one JSON string each way.
pub const WARDROBE_STATE: &str = "minehouse-cosmetics:state";
pub const WARDROBE_ACTION: &str = "minehouse-cosmetics:action";
/// Tells the cosmetics mod this client doesn't draw the player's own hat and
/// backpack on them, so it keeps sending them the vanilla way (helmet slot,
/// passenger). Nothing comes on it.
pub const WARDROBE_NO_SELF: &str = "minehouse-cosmetics:no_self";
/// The channels the server mods look for: registering them is how they learn
/// the client draws these itself (and that it has the wardrobe).
pub const CHANNELS: [&str; 5] = [ANVIL_COST, ZONE_SHAPES, WARDROBE_STATE, WARDROBE_ACTION, WARDROBE_NO_SELF];
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

// --- The wardrobe (`pro.minehouse.menu.wardrobe`) ---------------------------------

/// The key that opens it, as `Wardrobe.java` binds it (O).
pub const KEY_WARDROBE: &str = "wardrobe";
pub const DEFAULT_WARDROBE: &str = "key.keyboard.o";

/// One thing in the shop. `color`: what the player dyed it (or the default),
/// None when it can't be dyed.
#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
pub struct Item {
    pub id: String,
    pub slot: String,
    pub name: String,
    pub price: i32,
    pub owned: bool,
    #[serde(default)]
    pub color: Option<i32>,
    #[serde(default, rename = "defaultColor")]
    pub default_color: Option<i32>,
}

impl Item {
    pub fn dyeable(&self) -> bool {
        self.color.is_some()
    }
    /// The item model the server's resource pack gives it.
    pub fn model(&self) -> String {
        format!("minehouse-cosmetics:{}", self.id)
    }
}

/// `WardrobeNet.State`, and whether the server asks to open the screen.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Wardrobe {
    pub coins: i32,
    #[serde(default)]
    pub store: String,
    pub items: Vec<Item>,
    #[serde(default)]
    pub worn: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub open: bool,
}

impl Wardrobe {
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }
    pub fn is_worn(&self, item: &Item) -> bool {
        self.worn.get(&item.slot).is_some_and(|id| *id == item.id)
    }
}

/// `minehouse-cosmetics:state`: `writeUtf` of the JSON.
pub fn decode_wardrobe(data: &[u8]) -> Option<Wardrobe> {
    let json = Reader(data).utf(1 << 20)?;
    serde_json::from_str(&json).ok()
}

/// What the client asks the server for (`minehouse-cosmetics:action`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Open,
    Wear(String),
    Remove(String),
    Buy(String),
    Dye(String, i32),
}

impl Action {
    pub fn json(&self) -> String {
        use serde_json::json;
        match self {
            Action::Open => json!({ "action": "open" }),
            Action::Wear(id) => json!({ "action": "wear", "id": id }),
            Action::Remove(slot) => json!({ "action": "remove", "slot": slot }),
            Action::Buy(id) => json!({ "action": "buy", "id": id }),
            Action::Dye(id, color) => json!({ "action": "dye", "id": id, "color": color }),
        }
        .to_string()
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        write_utf(&mut out, &self.json());
        out
    }
}

/// The tabs: title and server slot; one without a slot is "coming later".
pub const SLOTS: [(&str, Option<&str>); 3] = [("Голова", Some("hat")), ("Спина", Some("backpack")), ("Плащ", None)];

/// `DyeColor.getTextureDiffuseColor`, in `DyeColor` order.
pub const DYES: [i32; 16] = [
    0xF9FFFE, 0xF9801D, 0xC74EBD, 0x3AB3DA, 0xFED83D, 0x80C71F, 0xF38BAA, 0x474F52, 0x9D9D97, 0x169C9C, 0x8932B8,
    0x3C44AA, 0x835432, 0x5E7C16, 0xB02E26, 0x1D1D21,
];
pub const DYE_NAMES: [&str; 16] = [
    "Белый",
    "Оранжевый",
    "Сиреневый",
    "Голубой",
    "Жёлтый",
    "Лаймовый",
    "Розовый",
    "Серый",
    "Светло-серый",
    "Бирюзовый",
    "Фиолетовый",
    "Синий",
    "Коричневый",
    "Зелёный",
    "Красный",
    "Чёрный",
];
/// The default colour first, then the 16 dyes.
pub const SWATCHES: usize = DYES.len() + 1;

pub fn swatch_color(item: &Item, i: usize) -> i32 {
    if i == 0 { item.default_color.unwrap_or(0xFFFFFF) } else { DYES[i - 1] }
}

pub fn swatch_name(i: usize) -> &'static str {
    if i == 0 { "Как было" } else { DYE_NAMES[i - 1] }
}

// The site's palette (zinc, dark), as `WardrobeScreen` has it.
pub const CARD: u32 = 0xDB09_090B;
pub const BORDER: u32 = 0xFF27_272A;
pub const MUTED_BG: u32 = 0xFF18_181B;
pub const HOVER_BG: u32 = 0xFF27_272A;
pub const FG: u32 = 0xFAFAFA;
pub const MUTED: u32 = 0xA1A1AA;
pub const FAINT: u32 = 0x71717A;
pub const DISABLED: u32 = 0x52525B;
pub const PRIMARY: u32 = 0xFFFA_FAFA;
pub const PRIMARY_HOVER: u32 = 0xFFE4_E4E7;
pub const ON_PRIMARY: u32 = 0x18181B;
pub const ACCENT: u32 = 0xA78BFA;
pub const GREEN: u32 = 0x4ADE80;
pub const RED: u32 = 0xF87171;

pub const PAD: f32 = 8.0;
pub const LEFT_W: f32 = 144.0;
pub const RIGHT_W: f32 = 134.0;
pub const PANEL_GAP: f32 = 6.0;
pub const PANEL_H: f32 = 210.0;
pub const TILE_W: f32 = 40.0;
pub const TILE_H: f32 = 46.0;
pub const GAP: f32 = 4.0;
pub const COLS: usize = 3;
pub const TABS_H: f32 = 18.0;
pub const SWATCH: f32 = 12.0;
pub const SWATCH_GAP: f32 = 4.0;
pub const SWATCH_COLS: usize = 7;
pub const BUTTON_H: f32 = 18.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn over(&self, p: [f32; 2]) -> bool {
        p[0] >= self.x && p[0] < self.x + self.w && p[1] >= self.y && p[1] < self.y + self.h
    }
    pub fn grow(&self, by: f32) -> Rect {
        Rect { x: self.x - by, y: self.y - by, w: self.w + by * 2.0, h: self.h + by * 2.0 }
    }
}

/// Where the two cards and their parts are, for a screen `size` GUI pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layout {
    pub left: Rect,
    pub right: Rect,
    pub tabs: [Rect; 3],
    /// The tile grid's visible area.
    pub grid: Rect,
}

pub fn layout(size: [f32; 2]) -> Layout {
    let h = PANEL_H.min(size[1] - 16.0).max(120.0);
    let x = ((size[0] - LEFT_W - PANEL_GAP - RIGHT_W) / 2.0).floor();
    let y = ((size[1] - h) / 2.0).floor();
    let left = Rect { x, y, w: LEFT_W, h };
    let right = Rect { x: x + LEFT_W + PANEL_GAP, y, w: RIGHT_W, h };
    let tab_w = (LEFT_W - PAD * 2.0) / 3.0;
    let tabs = [0.0, 1.0, 2.0].map(|i| Rect { x: x + PAD + tab_w * i, y: y + PAD, w: tab_w, h: TABS_H });
    let gy = y + PAD + TABS_H + 6.0;
    let grid = Rect { x: x + PAD, y: gy, w: LEFT_W - PAD * 2.0, h: y + h - PAD - 12.0 - gy };
    Layout { left, right, tabs, grid }
}

/// Rows of tiles that fit in the grid.
pub fn rows_shown(lay: &Layout) -> usize {
    (((lay.grid.h + GAP) / (TILE_H + GAP)).floor() as usize).max(1)
}

/// Tile `i` of the shown items, `scroll` rows down; None when it is out of view.
pub fn tile(lay: &Layout, i: usize, scroll: usize) -> Option<Rect> {
    let row = (i / COLS).checked_sub(scroll)?;
    if row >= rows_shown(lay) {
        return None;
    }
    let col = (i % COLS) as f32;
    Some(Rect {
        x: lay.grid.x + col * (TILE_W + GAP),
        y: lay.grid.y + row as f32 * (TILE_H + GAP),
        w: TILE_W,
        h: TILE_H,
    })
}

pub fn max_scroll(lay: &Layout, shown: usize) -> usize {
    shown.div_ceil(COLS).saturating_sub(rows_shown(lay))
}

/// Where the right card's parts are this frame (its height depends on the
/// selected item's name and whether it can be dyed): the screen draws it and
/// clicks read it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RightLayout {
    pub swatch_y: Option<f32>,
    pub buttons_y: f32,
}

impl RightLayout {
    pub fn swatch(&self, lay: &Layout, i: usize) -> Option<Rect> {
        let y = self.swatch_y?;
        Some(Rect {
            x: lay.right.x + PAD + (i % SWATCH_COLS) as f32 * (SWATCH + SWATCH_GAP),
            y: y + (i / SWATCH_COLS) as f32 * (SWATCH + SWATCH_GAP),
            w: SWATCH,
            h: SWATCH,
        })
    }
    pub fn action(&self, lay: &Layout) -> Rect {
        let w = ((RIGHT_W - PAD * 2.0 - 6.0) / 2.0).floor();
        Rect { x: lay.right.x + PAD, y: self.buttons_y, w, h: BUTTON_H }
    }
    pub fn take_off(&self, lay: &Layout) -> Rect {
        let w = ((RIGHT_W - PAD * 2.0 - 6.0) / 2.0).floor();
        Rect { x: lay.right.x + RIGHT_W - PAD - w, y: self.buttons_y, w, h: BUTTON_H }
    }
    /// The balance line under the buttons; "Пополнить" sits at its right end.
    pub fn balance_y(&self) -> f32 {
        self.buttons_y + 29.0
    }
    pub fn top_up(&self, lay: &Layout, text_w: f32) -> Rect {
        let y = self.balance_y();
        Rect { x: lay.right.x + RIGHT_W - PAD - text_w - 2.0, y: y - 2.0, w: text_w + 4.0, h: 14.0 }
    }
}

pub const SWATCH_ROWS: usize = SWATCHES.div_ceil(SWATCH_COLS);

/// The right card's layout with the selected item's name in `name_lines` lines.
pub fn right_layout(lay: &Layout, item: Option<&Item>, name_lines: usize) -> RightLayout {
    let mut y = lay.right.y + PAD + 16.0;
    let mut swatch_y = None;
    match item {
        None => y += 40.0,
        Some(item) => {
            y += name_lines as f32 * 11.0 + 4.0;
            if item.dyeable() {
                y += 12.0;
                swatch_y = Some(y);
                y += SWATCH_ROWS as f32 * (SWATCH + SWATCH_GAP);
            }
        }
    }
    RightLayout { swatch_y, buttons_y: y + 6.0 }
}

/// `text` broken into lines no wider than `max`, by words (a word wider than
/// the line gets a line of its own).
pub fn wrap(text: &str, max: f32, width: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if !line.is_empty() && width(&candidate) > max {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// What the screen remembers between frames.
#[derive(Clone, Debug, Default)]
pub struct WardrobeUi {
    pub slot: usize,
    pub selected: Option<String>,
    pub scroll: usize,
    /// The colour clicked for an item the player doesn't own yet (shown on the swatches).
    pub tint: std::collections::BTreeMap<String, i32>,
    /// The tile clicked last and when, for a double click.
    pub last_click: Option<(String, std::time::Instant)>,
    /// The screen and the right card as the last frame drew them.
    pub layout: Layout,
    pub right: RightLayout,
}

pub const DOUBLE_CLICK: std::time::Duration = std::time::Duration::from_millis(400);

/// What a click asks of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Clicked {
    Send(Action),
    OpenStore(String),
}

impl WardrobeUi {
    pub fn slot_id(&self) -> Option<&'static str> {
        SLOTS[self.slot.min(SLOTS.len() - 1)].1
    }
    pub fn shown<'a>(&self, w: &'a Wardrobe) -> Vec<&'a Item> {
        let Some(slot) = self.slot_id() else { return Vec::new() };
        w.items.iter().filter(|i| i.slot == slot).collect()
    }
    pub fn selected_item<'a>(&self, w: &'a Wardrobe) -> Option<&'a Item> {
        w.item(self.selected.as_deref()?)
    }
    /// The colour shown as chosen for `item`.
    pub fn color_of(&self, item: &Item) -> i32 {
        self.tint.get(&item.id).copied().or(item.color).or(item.default_color).unwrap_or(0xFFFFFF)
    }
    /// `selectSlot`: the tab, and what is worn there selected.
    pub fn select_slot(&mut self, w: &Wardrobe, slot: usize) {
        if self.slot != slot {
            self.slot = slot;
            self.scroll = 0;
        }
        let worn = SLOTS[slot].1.and_then(|s| w.worn.get(s)).and_then(|id| w.item(id));
        if let Some(item) = worn {
            self.selected = Some(item.id.clone());
        } else if self.selected_item(w).is_some_and(|i| Some(i.slot.as_str()) != SLOTS[slot].1) {
            self.selected = None;
        }
    }
    pub fn action_label(w: &Wardrobe, item: Option<&Item>) -> String {
        match item {
            Some(item) if w.is_worn(item) => "Снять".into(),
            Some(item) if !item.owned => format!("{} MX", item.price),
            _ => "Надеть".into(),
        }
    }
    pub fn action_enabled(&self, w: &Wardrobe) -> bool {
        self.selected_item(w).is_some_and(|i| i.owned || w.coins >= i.price)
    }
    pub fn scroll_by(&mut self, w: &Wardrobe, lay: &Layout, rows: i32) {
        let max = max_scroll(lay, self.shown(w).len());
        self.scroll = (self.scroll as i64 + i64::from(rows)).clamp(0, max as i64) as usize;
    }
    /// A left click at `p`; `top_up_w` is the width of "Пополнить".
    pub fn click(&mut self, w: &Wardrobe, lay: &Layout, p: [f32; 2], top_up_w: f32, now: std::time::Instant) -> Vec<Clicked> {
        let mut out = Vec::new();
        if lay.left.over(p) {
            for (i, tab) in lay.tabs.iter().enumerate() {
                if SLOTS[i].1.is_some() && tab.over(p) {
                    self.select_slot(w, i);
                    return out;
                }
            }
            let shown = self.shown(w);
            for (i, item) in shown.iter().enumerate() {
                if !tile(lay, i, self.scroll).is_some_and(|t| t.over(p)) {
                    continue;
                }
                let double = self
                    .last_click
                    .as_ref()
                    .is_some_and(|(id, at)| *id == item.id && now.duration_since(*at) <= DOUBLE_CLICK);
                if double && item.owned {
                    out.push(Clicked::Send(if w.is_worn(item) {
                        Action::Remove(item.slot.clone())
                    } else {
                        Action::Wear(item.id.clone())
                    }));
                    self.last_click = None;
                } else {
                    self.last_click = Some((item.id.clone(), now));
                }
                self.selected = Some(item.id.clone());
                return out;
            }
            return out;
        }
        if !lay.right.over(p) {
            return out;
        }
        let right = self.right;
        let selected = self.selected_item(w);
        if let Some(item) = selected.filter(|i| i.dyeable()) {
            for i in 0..SWATCHES {
                if right.swatch(lay, i).is_some_and(|s| s.over(p)) {
                    let rgb = swatch_color(item, i);
                    self.tint.insert(item.id.clone(), rgb);
                    if item.owned {
                        out.push(Clicked::Send(Action::Dye(item.id.clone(), rgb)));
                    }
                    return out;
                }
            }
        }
        if right.action(lay).over(p) && self.action_enabled(w) {
            if let Some(item) = selected {
                out.push(Clicked::Send(if w.is_worn(item) {
                    Action::Remove(item.slot.clone())
                } else if item.owned {
                    Action::Wear(item.id.clone())
                } else {
                    Action::Buy(item.id.clone())
                }));
            }
            return out;
        }
        if right.take_off(lay).over(p) && !w.worn.is_empty() {
            out.extend(w.worn.keys().map(|s| Clicked::Send(Action::Remove(s.clone()))));
            self.selected = None;
            return out;
        }
        if right.top_up(lay, top_up_w).over(p) && w.store.starts_with("https://") {
            out.push(Clicked::OpenStore(w.store.clone()));
        }
        out
    }
}

/// Seconds to wait for the state after asking to open before saying the server has no wardrobe.
pub const NO_WARDROBE_AFTER: std::time::Duration = std::time::Duration::from_millis(1500);

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
    /// The last wardrobe state the server sent (None on a server without one).
    pub wardrobe: Option<Wardrobe>,
    pub ui: WardrobeUi,
    /// Actions the screen's clicks queued, sent on the next tick.
    pub out: Vec<Action>,
    /// When the player pressed the wardrobe key and no state has come yet.
    pub asked: Option<std::time::Instant>,
}

impl State {
    pub fn left_server(&mut self) {
        self.registered = false;
        self.open_container = None;
        self.anvil = AnvilCost::default();
        self.drawing_real = false;
        self.shapes.clear();
        self.wardrobe = None;
        self.ui = WardrobeUi::default();
        self.out.clear();
        self.asked = None;
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
        assert_eq!(
            encode_register(&CHANNELS),
            b"minehouse:anvil_cost\0minehouse:zone_shapes\0minehouse-cosmetics:state\0minehouse-cosmetics:action\0minehouse-cosmetics:no_self".to_vec()
        );
    }
    fn wardrobe() -> Wardrobe {
        let json = r#"{"coins":120,"store":"https://minehouse.pro/store","open":true,
            "items":[{"id":"cap","slot":"hat","name":"Кепка","price":50,"owned":true,"color":16711680,"defaultColor":255},
                     {"id":"crown","slot":"hat","name":"Корона","price":500,"owned":false},
                     {"id":"bag","slot":"backpack","name":"Рюкзак","price":80,"owned":false}],
            "worn":{"hat":"cap"}}"#;
        let mut data = Vec::new();
        write_utf(&mut data, json);
        decode_wardrobe(&data).unwrap()
    }

    #[test]
    fn wardrobe_state_decodes() {
        let w = wardrobe();
        assert_eq!(w.coins, 120);
        assert!(w.open);
        assert_eq!(w.items.len(), 3);
        assert_eq!(w.item("cap").unwrap().default_color, Some(255));
        assert!(w.is_worn(w.item("cap").unwrap()));
        assert!(!w.item("crown").unwrap().dyeable());
        assert_eq!(w.item("bag").unwrap().model(), "minehouse-cosmetics:bag");
        assert_eq!(decode_wardrobe(&[3, b'{', b'}', b'x']), None);
    }

    #[test]
    fn wardrobe_actions() {
        assert_eq!(Action::Open.json(), r#"{"action":"open"}"#);
        let v: serde_json::Value = serde_json::from_str(&Action::Dye("cap".into(), 0x3C44AA).json()).unwrap();
        assert_eq!(v["action"], "dye");
        assert_eq!(v["id"], "cap");
        assert_eq!(v["color"], 0x3C44AA);
        let enc = Action::Remove("hat".into()).encode();
        assert_eq!(Reader(&enc).utf(4096).unwrap(), r#"{"action":"remove","slot":"hat"}"#);
    }

    #[test]
    fn wardrobe_clicks() {
        let w = wardrobe();
        let lay = layout([427.0, 240.0]);
        let mut ui = WardrobeUi::default();
        let now = std::time::Instant::now();
        let centre = |r: Rect| [r.x + r.w / 2.0, r.y + r.h / 2.0];
        // The second hat tile selects the crown; the buy button buys... not with 120 MX.
        let t = tile(&lay, 1, 0).unwrap();
        assert!(ui.click(&w, &lay, centre(t), 40.0, now).is_empty());
        assert_eq!(ui.selected.as_deref(), Some("crown"));
        ui.right = right_layout(&lay, ui.selected_item(&w), 1);
        assert!(!ui.action_enabled(&w));
        assert!(ui.click(&w, &lay, centre(ui.right.action(&lay)), 40.0, now).is_empty());
        // A double click on the worn cap takes it off.
        let t = tile(&lay, 0, 0).unwrap();
        assert!(ui.click(&w, &lay, centre(t), 40.0, now).is_empty());
        let out = ui.click(&w, &lay, centre(t), 40.0, now + std::time::Duration::from_millis(200));
        assert_eq!(out, vec![Clicked::Send(Action::Remove("hat".into()))]);
        // A swatch dyes it.
        ui.right = right_layout(&lay, ui.selected_item(&w), 1);
        let s = ui.right.swatch(&lay, 12).unwrap();
        let out = ui.click(&w, &lay, centre(s), 40.0, now);
        assert_eq!(out, vec![Clicked::Send(Action::Dye("cap".into(), DYES[11]))]);
        assert_eq!(ui.color_of(w.item("cap").unwrap()), DYES[11]);
        // The back tab: nothing worn there, the cap is no longer selected.
        let out = ui.click(&w, &lay, centre(lay.tabs[1]), 40.0, now);
        assert!(out.is_empty());
        assert_eq!(ui.slot, 1);
        assert_eq!(ui.selected, None);
        assert_eq!(ui.shown(&w).len(), 1);
        // The cape tab is not there yet.
        ui.click(&w, &lay, centre(lay.tabs[2]), 40.0, now);
        assert_eq!(ui.slot, 1);
        // "Снять всё" and the store link.
        let out = ui.click(&w, &lay, centre(ui.right.take_off(&lay)), 40.0, now);
        assert_eq!(out, vec![Clicked::Send(Action::Remove("hat".into()))]);
        let out = ui.click(&w, &lay, centre(ui.right.top_up(&lay, 40.0)), 40.0, now);
        assert_eq!(out, vec![Clicked::OpenStore("https://minehouse.pro/store".into())]);
    }

    #[test]
    fn wardrobe_grid_scrolls() {
        let lay = layout([427.0, 240.0]);
        let rows = rows_shown(&lay);
        assert!(rows >= 2);
        assert_eq!(max_scroll(&lay, COLS * rows), 0);
        assert_eq!(max_scroll(&lay, COLS * rows + 1), 1);
        assert!(tile(&lay, COLS * rows, 0).is_none());
        assert_eq!(tile(&lay, COLS * rows, 1).unwrap().y, lay.grid.y + (rows - 1) as f32 * (TILE_H + GAP));
        assert!(tile(&lay, 0, 1).is_none());
        let last = tile(&lay, COLS * rows - 1, 0).unwrap();
        assert!(last.y + last.h <= lay.grid.y + lay.grid.h);
    }

    #[test]
    fn wrapping() {
        let w = |s: &str| s.chars().count() as f32 * 6.0;
        assert_eq!(wrap("Кепка с длинным козырьком", 60.0, w), vec!["Кепка с", "длинным", "козырьком"]);
        assert_eq!(wrap("Кепка", 60.0, w), vec!["Кепка"]);
        assert!(wrap("", 60.0, w).is_empty());
    }
}
