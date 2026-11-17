// World Map: a full-screen map of every chunk the player has seen, kept on
// disk per world and dimension (in the spirit of Xaero's World Map, written
// from scratch). Chunks are scanned as map items see them (`MapItem.update`:
// the first block with a map colour, the fluid over it, water by depth,
// land shaded against its north neighbour) and stored as one byte per
// column: the map colour id times four plus its brightness, 0 for unknown.
//
// This file holds what doesn't need the game: the columns' scan, the
// regions on disk, the image composed for a view, and the screen's state.
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const ID: &str = "worldmap";
pub const NAME: &str = "World Map";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str = "A full-screen map of everything you have explored, saved per server and dimension. M opens it.";
pub const KEY_OPEN: &str = "open";
pub const DEFAULT_OPEN: &str = "key.keyboard.m";
pub const UNBOUND: &str = "key.keyboard.unknown";

/// Where the map's picture goes in the mod image slot: below the first 256
/// rows, which Plasmo Voice's icons use.
pub const IMAGE_Y: u32 = 256;
pub const IMAGE_W: u32 = 1024;
pub const IMAGE_H: u32 = 768;

/// Chunks per region side, and columns per region side.
pub const REGION: i32 = 32;
const SIDE: usize = (REGION * 16) as usize;
/// Regions kept in memory before the farthest go back to disk.
const KEEP: usize = 96;
/// A chunk whose world changed is scanned again at most this often.
pub const RESCAN: Duration = Duration::from_secs(5);
/// Changes go to disk this often.
pub const SAVE_EVERY: Duration = Duration::from_secs(30);

/// GUI pixels per block, from far out to close in.
pub const ZOOMS: [f32; 9] = [0.0625, 0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0];
pub const DEFAULT_ZOOM: usize = 4;

/// Water's and lava's map colours (`MapColor.WATER`, `FIRE`).
const WATER: u8 = 12;
const LAVA: u8 = 4;
const UNKNOWN: [u8; 4] = [14, 14, 18, 255];

/// What the scan needs of each block state: its map colour, and
/// 1 water, 2 lava (`FLUID`), 4 air, 8 sturdy on top.
pub const FLUID: u8 = 3;
pub const AIR: u8 = 4;
pub const STURDY_TOP: u8 = 8;
#[derive(Default)]
pub struct Palette {
    pub color: Vec<u8>,
    pub flags: Vec<u8>,
}
impl Palette {
    fn get(&self, id: u32) -> (u8, u8) {
        let i = id as usize;
        (self.color.get(i).copied().unwrap_or(0), self.flags.get(i).copied().unwrap_or(AIR))
    }
}

/// Which blocks a column shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    /// The top block, as map items show it.
    Surface,
    /// Under a roof (the Nether): the floor below this height.
    Below(i32),
}

/// One column as a map item finds it: colour, height and water depth.
pub fn scan_column(get: &impl Fn(i32) -> u32, palette: &Palette, top: i32, bottom: i32) -> (u8, i16, u8) {
    let mut y = top;
    while y > bottom {
        let (mut color, flags) = palette.get(get(y));
        if color == 0 {
            y -= 1;
            continue;
        }
        let mut depth = 0u8;
        let fluid = flags & FLUID;
        if fluid != 0 {
            let mut below = y - 1;
            while below > bottom && palette.get(get(below)).1 & FLUID != 0 {
                depth = depth.saturating_add(1);
                below -= 1;
            }
            depth = depth.saturating_add(1);
            if flags & STURDY_TOP == 0 {
                color = if fluid == 2 { LAVA } else { WATER };
            }
        }
        return (color, y as i16, depth);
    }
    (0, bottom as i16, 0)
}

/// The map item's shading at scale 1: brighter facing a lower north
/// neighbour, darker facing a higher one, dithered; water by its depth.
pub fn brightness(color: u8, height: i16, north: Option<i16>, depth: u8, parity: i32) -> u8 {
    let parity = f64::from(parity & 1);
    if color == WATER {
        let diff = f64::from(depth) * 0.1 + parity * 0.2;
        return if diff < 0.5 {
            2
        } else if diff > 0.9 {
            0
        } else {
            1
        };
    }
    let north = north.unwrap_or(height);
    let diff = f64::from(height - north) * 4.0 / 5.0 + (parity - 0.5) * 0.4;
    if diff > 0.6 {
        2
    } else if diff < -0.6 {
        0
    } else {
        1
    }
}

/// A chunk's 256 columns (`z * 16 + x`) as stored bytes, from each
/// column's block state at a height (`get(x, y, z)`, chunk-local x and z),
/// scanning down from `top` to `bottom`. `north` is the heights of the
/// chunk north of it (its z = 15 row) when known. Returns the bytes and this
/// chunk's own z = 15 heights, for the chunk south of it.
pub fn scan_chunk(
    get: impl Fn(i32, i32, i32) -> u32,
    palette: &Palette,
    top: i32,
    bottom: i32,
    layer: Layer,
    north: Option<[i16; 16]>,
    origin: (i32, i32),
) -> ([u8; 256], [i16; 16]) {
    let mut out = [0u8; 256];
    let mut heights = [0i16; 256];
    let mut colors = [0u8; 256];
    let mut depths = [0u8; 256];
    for z in 0..16 {
        for x in 0..16 {
            let i = (z * 16 + x) as usize;
            let column = |y: i32| get(x, y, z);
            let (c, h, d) = match layer {
                Layer::Below(head) => {
                    let (color, flags) = palette.get(get(x, head, z));
                    if flags & AIR == 0 && color != 0 && flags & FLUID == 0 {
                        // A wall the map cuts through: drawn dark.
                        (color, head as i16, 255)
                    } else {
                        scan_column(&column, palette, top.min(head), bottom)
                    }
                }
                Layer::Surface => scan_column(&column, palette, top, bottom),
            };
            colors[i] = c;
            heights[i] = h;
            depths[i] = d;
        }
    }
    for z in 0..16usize {
        for x in 0..16usize {
            let i = z * 16 + x;
            let c = colors[i];
            if c == 0 || usize::from(c) >= 64 {
                continue;
            }
            let b = if depths[i] == 255 {
                0
            } else {
                let n = if z > 0 {
                    (colors[i - 16] != 0).then_some(heights[i - 16])
                } else {
                    north.map(|row| row[x])
                };
                brightness(c, heights[i], n, depths[i], origin.0 * 16 + x as i32 + origin.1 * 16 + z as i32)
            };
            out[i] = c << 2 | b;
        }
    }
    let mut south = [0i16; 16];
    south.copy_from_slice(&heights[240..256]);
    (out, south)
}

/// A stored byte as RGBA (`MapColor.getColorFromPackedId`), `colors` being
/// vanilla's `MapColor` table.
pub fn rgba(byte: u8, colors: &[u32]) -> [u8; 4] {
    let id = usize::from(byte >> 2);
    if id == 0 || id >= colors.len() {
        return UNKNOWN;
    }
    let modifier = [180u32, 220, 255, 135][usize::from(byte & 3)];
    let col = colors[id];
    let ch = |s: u32| ((col >> s & 255) * modifier / 255) as u8;
    [ch(16), ch(8), ch(0), 255]
}

/// 32 x 32 chunks of stored bytes.
struct Region {
    cols: Vec<u8>,
    dirty: bool,
}
impl Region {
    fn new() -> Self {
        Self { cols: vec![0; SIDE * SIDE], dirty: false }
    }
}

/// Run-length bytes: pairs of (count 1..=255, value).
pub fn pack(cols: &[u8]) -> Vec<u8> {
    let mut out = b"HWM1".to_vec();
    let mut i = 0;
    while i < cols.len() {
        let v = cols[i];
        let mut n = 1;
        while n < 255 && i + n < cols.len() && cols[i + n] == v {
            n += 1;
        }
        out.push(n as u8);
        out.push(v);
        i += n;
    }
    out
}
pub fn unpack(data: &[u8], len: usize) -> Option<Vec<u8>> {
    let body = data.strip_prefix(b"HWM1")?;
    let mut out = Vec::with_capacity(len);
    for pair in body.chunks_exact(2) {
        out.extend(std::iter::repeat_n(pair[1], usize::from(pair[0])));
    }
    (out.len() == len).then_some(out)
}

/// The map of one world and dimension: regions in memory, loaded from and
/// saved to `dir`.
#[derive(Default)]
pub struct Store {
    dir: Option<PathBuf>,
    regions: HashMap<(i32, i32), Region>,
    /// Regions with no file, so the disk isn't asked again.
    missing: HashSet<(i32, i32)>,
    /// Something changed since the image was composed.
    pub changed: bool,
}
impl Store {
    /// The folder this map lives in, None while there is no world.
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }
    /// Save what changed and switch to another world or dimension.
    pub fn open(&mut self, dir: Option<PathBuf>) {
        self.save();
        *self = Self { dir, changed: true, ..Self::default() };
    }
    fn file(&self, key: (i32, i32)) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join(format!("{}_{}.hwm", key.0, key.1)))
    }
    fn region(&mut self, key: (i32, i32), create: bool) -> Option<&mut Region> {
        if !self.regions.contains_key(&key) {
            let loaded = if self.missing.contains(&key) {
                None
            } else {
                self.file(key)
                    .and_then(|f| std::fs::read(f).ok())
                    .and_then(|d| unpack(&d, SIDE * SIDE))
            };
            match loaded {
                Some(cols) => {
                    self.regions.insert(key, Region { cols, dirty: false });
                }
                None if create => {
                    self.missing.remove(&key);
                    self.regions.insert(key, Region::new());
                }
                None => {
                    self.missing.insert(key);
                    return None;
                }
            }
        }
        self.regions.get_mut(&key)
    }
    /// Keep a scanned chunk's columns.
    pub fn put_chunk(&mut self, cx: i32, cz: i32, cols: &[u8; 256]) {
        let key = (cx.div_euclid(REGION), cz.div_euclid(REGION));
        let (ox, oz) = (cx.rem_euclid(REGION) as usize * 16, cz.rem_euclid(REGION) as usize * 16);
        let Some(region) = self.region(key, true) else {
            return;
        };
        let mut differs = false;
        for z in 0..16 {
            let row = (oz + z) * SIDE + ox;
            let new = &cols[z * 16..z * 16 + 16];
            // Unknown columns (nothing scanned there) don't wipe what was.
            for (old, &n) in region.cols[row..row + 16].iter_mut().zip(new) {
                if n != 0 && *old != n {
                    *old = n;
                    differs = true;
                }
            }
        }
        if differs {
            region.dirty = true;
            self.changed = true;
        }
    }
    /// The stored byte of column (x, z), 0 when unknown.
    pub fn column(&mut self, x: i32, z: i32) -> u8 {
        let key = (x.div_euclid(SIDE as i32), z.div_euclid(SIDE as i32));
        let (lx, lz) = (x.rem_euclid(SIDE as i32) as usize, z.rem_euclid(SIDE as i32) as usize);
        self.region(key, false).map_or(0, |r| r.cols[lz * SIDE + lx])
    }
    /// Write every changed region.
    pub fn save(&mut self) {
        let Some(dir) = self.dir.clone() else {
            return;
        };
        let mut made = false;
        for (key, region) in &mut self.regions {
            if !region.dirty {
                continue;
            }
            if !made {
                let _ = std::fs::create_dir_all(&dir);
                made = true;
            }
            let file = dir.join(format!("{}_{}.hwm", key.0, key.1));
            if std::fs::write(file, pack(&region.cols)).is_ok() {
                region.dirty = false;
            }
        }
    }
    /// Let the regions farthest from block (x, z) go once too many are kept.
    pub fn trim(&mut self, x: i32, z: i32) {
        if self.regions.len() <= KEEP {
            return;
        }
        let at = (x.div_euclid(SIDE as i32), z.div_euclid(SIDE as i32));
        let mut keys: Vec<_> = self.regions.keys().copied().collect();
        keys.sort_by_key(|k| std::cmp::Reverse((k.0 - at.0).abs().max((k.1 - at.1).abs())));
        self.save();
        for k in keys.into_iter().take(self.regions.len() - KEEP) {
            self.regions.remove(&k);
        }
    }
}

/// What the map screen shows: its middle in blocks, and its zoom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub x: f64,
    pub z: f64,
    pub zoom: usize,
}
impl View {
    pub fn scale(&self) -> f64 {
        f64::from(ZOOMS[self.zoom.min(ZOOMS.len() - 1)])
    }
    /// The block under GUI point `at` on a screen `size` GUI pixels.
    pub fn block_at(&self, at: [f32; 2], size: [f32; 2]) -> (f64, f64) {
        let s = self.scale();
        (
            self.x + (f64::from(at[0]) - f64::from(size[0]) / 2.0) / s,
            self.z + (f64::from(at[1]) - f64::from(size[1]) / 2.0) / s,
        )
    }
    /// Where block (x, z) is on the screen, in GUI pixels.
    pub fn point_of(&self, x: f64, z: f64, size: [f32; 2]) -> [f32; 2] {
        let s = self.scale();
        [
            ((x - self.x) * s + f64::from(size[0]) / 2.0) as f32,
            ((z - self.z) * s + f64::from(size[1]) / 2.0) as f32,
        ]
    }
    /// Zoom in (`steps` > 0) or out about GUI point `at`, which keeps
    /// showing the same block.
    pub fn zoom_about(&mut self, steps: i32, at: [f32; 2], size: [f32; 2]) {
        let before = self.block_at(at, size);
        self.zoom = (self.zoom as i32 + steps).clamp(0, ZOOMS.len() as i32 - 1) as usize;
        let after = self.block_at(at, size);
        self.x += before.0 - after.0;
        self.z += before.1 - after.1;
    }
}

/// How the picture covers a screen of `size` GUI pixels: texels across and
/// down, and GUI pixels per texel (more than one when the screen is larger
/// than the image slot).
pub fn image_size(size: [f32; 2]) -> (u32, u32, f32) {
    let (w, h) = (size[0].max(1.0), size[1].max(1.0));
    let per = (w / IMAGE_W as f32).max(h / IMAGE_H as f32).max(1.0).ceil();
    let tw = ((w / per).ceil() as u32).clamp(1, IMAGE_W);
    let th = ((h / per).ceil() as u32).clamp(1, IMAGE_H);
    (tw, th, per)
}

/// The picture of `view` on a screen of `size` GUI pixels: RGBA rows of
/// `image_size(size)` texels.
pub fn compose(store: &mut Store, view: View, size: [f32; 2], colors: &[u32]) -> Vec<u8> {
    let (tw, th, per) = image_size(size);
    let mut out = vec![0u8; (tw * th * 4) as usize];
    let s = view.scale();
    let per = f64::from(per);
    let (hw, hh) = (f64::from(size[0]) / 2.0, f64::from(size[1]) / 2.0);
    for j in 0..th {
        let z = (view.z + ((f64::from(j) + 0.5) * per - hh) / s).floor() as i32;
        for i in 0..tw {
            let x = (view.x + ((f64::from(i) + 0.5) * per - hw) / s).floor() as i32;
            let o = ((j * tw + i) * 4) as usize;
            out[o..o + 4].copy_from_slice(&rgba(store.column(x, z), colors));
        }
    }
    out
}

/// A folder name for a world or server: letters, digits, `.`, `-` and `_`
/// kept, the rest `_`.
pub fn folder_name(s: &str) -> String {
    let name: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' })
        .collect();
    if name.is_empty() || name.chars().all(|c| c == '.') { "_".into() } else { name }
}

/// Everything the mod keeps between hooks.
pub struct State {
    pub store: Store,
    pub palette: Palette,
    /// Per loaded chunk: the world revision it was scanned at, and when.
    pub scanned: HashMap<(i32, i32), (u64, Instant)>,
    /// Each scanned chunk's z = 15 heights, for its south neighbour's shading.
    pub south_rows: HashMap<(i32, i32), [i16; 16]>,
    pub saved: Option<Instant>,
    /// The screen's view, None until it first opens in this world.
    pub view: Option<View>,
    /// The view and size the image in the slot shows.
    pub shown: Option<(View, [f32; 2])>,
    /// Asked from a screen hook (which can't close it): close the screen.
    pub close: bool,
    /// Asked from a screen hook: put the view back on the player.
    pub recenter: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            store: Store::default(),
            palette: Palette::default(),
            scanned: HashMap::new(),
            south_rows: HashMap::new(),
            saved: None,
            view: None,
            shown: None,
            close: false,
            recenter: false,
        }
    }
}
thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}
pub fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// The screen's texts.
pub fn tr(russian: bool, key: &str) -> &'static str {
    match (russian, key) {
        (true, "hint") => "ЛКМ — двигать, колесо — масштаб, Пробел — к себе",
        (false, "hint") => "Drag to move, wheel to zoom, Space to centre",
        (true, "title") => "Карта мира",
        (false, "title") => "World Map",
        _ => "",
    }
}

/// Chunk offsets at Chebyshev distance `ring`.
pub fn ring_offsets(ring: i32) -> impl Iterator<Item = (i32, i32)> {
    (-ring..=ring).flat_map(move |dz| {
        (-ring..=ring)
            .filter(move |&dx| dx.abs() == ring || dz.abs() == ring)
            .map(move |dx| (dx, dz))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> Palette {
        // 0 air, 1 grass (colour 1), 2 water (colour 12), 3 stone (colour 11, sturdy).
        Palette { color: vec![0, 1, 12, 11], flags: vec![AIR, STURDY_TOP, 1, STURDY_TOP] }
    }

    #[test]
    fn a_column_shows_its_top_block_and_water_by_depth() {
        let p = palette();
        // Stone up to 60, water 61..=63, air above.
        let get = |y: i32| if y <= 60 { 3 } else if y <= 63 { 2 } else { 0 };
        assert_eq!(scan_column(&get, &p, 100, -64), (WATER, 63, 3));
        let get = |y: i32| if y <= 70 { 1 } else { 0 };
        assert_eq!(scan_column(&get, &p, 100, -64), (1, 70, 0));
    }

    #[test]
    fn land_is_shaded_against_its_north_neighbour() {
        let p = palette();
        // Grass rising one block per row southwards: each column higher than
        // its north one is drawn bright (2) or normal, never dark.
        let (cols, south) = scan_chunk(|_, y, z| if y <= 64 + z { 1 } else { 0 }, &p, 100, -64, Layer::Surface, None, (0, 0));
        for z in 1..16 {
            for x in 0..16 {
                let b = cols[z * 16 + x] & 3;
                assert!(b >= 1, "column {x},{z} is dark");
                assert_eq!(cols[z * 16 + x] >> 2, 1);
            }
        }
        assert_eq!(south[0], 64 + 15);
        // Flat with no north neighbour: never bright or dark.
        let (cols, _) = scan_chunk(|_, y, _| if y <= 64 { 1 } else { 0 }, &p, 100, -64, Layer::Surface, None, (0, 0));
        assert!(cols.iter().all(|c| c & 3 == 1));
    }

    #[test]
    fn regions_round_trip_through_disk_and_keep_known_columns() {
        let dir = std::env::temp_dir().join(format!("worldmap-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = Store::default();
        store.open(Some(dir.clone()));
        let mut cols = [0u8; 256];
        cols[0] = 1 << 2 | 1;
        cols[255] = 12 << 2 | 2;
        store.put_chunk(-1, 33, &cols);
        assert!(store.changed);
        // A later scan with an unknown column keeps what was there.
        let mut again = cols;
        again[0] = 0;
        store.put_chunk(-1, 33, &again);
        store.open(Some(dir.clone()));
        assert_eq!(store.column(-16, 33 * 16), 1 << 2 | 1);
        assert_eq!(store.column(-1, 33 * 16 + 15), 12 << 2 | 2);
        assert_eq!(store.column(0, 0), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zooming_keeps_the_block_under_the_pointer() {
        let size = [400.0, 300.0];
        let mut v = View { x: 100.0, z: -50.0, zoom: DEFAULT_ZOOM };
        let at = [300.0, 50.0];
        let before = v.block_at(at, size);
        v.zoom_about(2, at, size);
        let after = v.block_at(at, size);
        assert!((before.0 - after.0).abs() < 1e-9 && (before.1 - after.1).abs() < 1e-9);
        let p = v.point_of(before.0, before.1, size);
        assert!((p[0] - at[0]).abs() < 1e-3 && (p[1] - at[1]).abs() < 1e-3);
    }

    #[test]
    fn the_picture_fits_the_image_slot() {
        assert_eq!(image_size([480.0, 270.0]), (480, 270, 1.0));
        let (w, h, per) = image_size([1920.0, 1080.0]);
        assert!(w <= IMAGE_W && h <= IMAGE_H && per == 2.0);
    }
}
