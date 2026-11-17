// `.litematic` files (Litematica's own format): gzip NBT with a `Metadata`
// compound and one compound per region under `Regions`, each with its
// `Position` and `Size` (negative when it grows the other way), a
// `BlockStatePalette` and the palette indices packed in `BlockStates`,
// entries crossing the longs' boundaries (`LitematicaBitArray`), indexed
// `y * sx * sz + z * sx + x`.
use crate::nbt::{self, Tag};
use std::io::Read;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct State {
    pub name: String,
    /// Properties, sorted by name.
    pub props: Vec<(String, String)>,
}
impl State {
    pub fn air() -> Self {
        Self { name: "minecraft:air".into(), props: Vec::new() }
    }
    pub fn is_air(&self) -> bool {
        matches!(&*self.name, "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
    }
    pub fn prop(&self, key: &str) -> Option<&str> {
        self.props.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
    fn set(&mut self, key: &str, value: &str) {
        if let Some(p) = self.props.iter_mut().find(|(k, _)| k == key) {
            p.1 = value.to_owned();
        }
    }
    /// `minecraft:oak_stairs[facing=east,half=bottom]`.
    pub fn text(&self) -> String {
        if self.props.is_empty() {
            return self.name.clone();
        }
        let props: Vec<String> = self.props.iter().map(|(k, v)| format!("{k}={v}")).collect();
        format!("{}[{}]", self.name, props.join(","))
    }
}

pub struct Region {
    pub name: String,
    /// The lowest corner, from the schematic's origin.
    pub min: [i32; 3],
    /// Its size along each axis, positive.
    pub size: [i32; 3],
    pub palette: Vec<State>,
    bits: u32,
    data: Vec<i64>,
}
impl Region {
    /// The palette index at (x, y, z) from `min`.
    pub fn index(&self, x: i32, y: i32, z: i32) -> usize {
        let [sx, _, sz] = self.size;
        let i = (y as u64 * sx as u64 * sz as u64 + z as u64 * sx as u64 + x as u64) * u64::from(self.bits);
        let mask = (1u64 << self.bits) - 1;
        let start = (i >> 6) as usize;
        let end = ((i + u64::from(self.bits) - 1) >> 6) as usize;
        let off = (i & 63) as u32;
        let Some(&lo) = self.data.get(start) else {
            return 0;
        };
        let lo = lo as u64;
        let v = if start == end {
            lo >> off & mask
        } else {
            let hi = self.data.get(end).copied().unwrap_or(0) as u64;
            (lo >> off | hi << (64 - off)) & mask
        };
        v as usize
    }
}

pub struct Schematic {
    pub name: String,
    pub author: String,
    /// The game's data version it was saved by (`MinecraftDataVersion`).
    pub data_version: i32,
    pub regions: Vec<Region>,
}

/// Reads a `.litematic` file's bytes (gzip or plain NBT).
pub fn parse(bytes: &[u8]) -> Result<Schematic, String> {
    let raw = if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(bytes)
            .take(512 << 20)
            .read_to_end(&mut out)
            .map_err(|e| format!("gzip: {e}"))?;
        out
    } else {
        bytes.to_vec()
    };
    let root = nbt::read(&raw)?;
    let meta = root.get("Metadata");
    let text = |k: &str| meta.and_then(|m| m.get(k)).and_then(Tag::str).unwrap_or("").to_owned();
    let regions_tag = root.get("Regions").and_then(Tag::compound).ok_or("no Regions: not a .litematic file")?;
    let mut names: Vec<&String> = regions_tag.keys().collect();
    names.sort();
    let mut regions = Vec::new();
    for name in names {
        let r = &regions_tag[name];
        let pos = r.get("Position").and_then(Tag::vec3).ok_or("a region without Position")?;
        let size = r.get("Size").and_then(Tag::vec3).ok_or("a region without Size")?;
        let mut min = [0; 3];
        let mut abs = [0; 3];
        for a in 0..3 {
            abs[a] = size[a].abs();
            min[a] = if size[a] < 0 { pos[a] + size[a] + 1 } else { pos[a] };
        }
        if abs.iter().any(|&s| s == 0) || abs.iter().map(|&s| i64::from(s)).product::<i64>() > 1 << 28 {
            continue;
        }
        let palette: Vec<State> = r
            .get("BlockStatePalette")
            .map(Tag::list)
            .unwrap_or(&[])
            .iter()
            .map(|t| {
                let name = t.get("Name").and_then(Tag::str).unwrap_or("minecraft:air").to_owned();
                let mut props: Vec<(String, String)> = t
                    .get("Properties")
                    .and_then(Tag::compound)
                    .map(|m| m.iter().filter_map(|(k, v)| Some((k.clone(), v.str()?.to_owned()))).collect())
                    .unwrap_or_default();
                props.sort();
                State { name, props }
            })
            .collect();
        if palette.is_empty() {
            continue;
        }
        let bits = (usize::BITS - (palette.len() - 1).leading_zeros()).max(2);
        let data = match r.get("BlockStates") {
            Some(Tag::LongArray(v)) => v.clone(),
            _ => Vec::new(),
        };
        regions.push(Region { name: name.clone(), min, size: abs, palette, bits, data });
    }
    Ok(Schematic {
        name: text("Name"),
        author: text("Author"),
        data_version: root.get("MinecraftDataVersion").and_then(Tag::int).unwrap_or(0),
        regions,
    })
}

/// A mirror before the turn (vanilla's `Mirror`): `LeftRight` flips z,
/// `FrontBack` flips x.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mirror {
    #[default]
    None,
    LeftRight,
    FrontBack,
}

/// Where a schematic is placed and how it is turned: mirrored first, then
/// turned `rotation` quarter turns clockwise (seen from above) about its
/// origin, which is put at `origin`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Transform {
    pub origin: [i32; 3],
    pub rotation: u8,
    pub mirror: Mirror,
}
impl Transform {
    /// Where a block at `rel` from the schematic's origin goes in the world.
    pub fn pos(&self, rel: [i32; 3]) -> [i32; 3] {
        let [mut x, y, mut z] = rel;
        match self.mirror {
            Mirror::None => {}
            Mirror::LeftRight => z = -z,
            Mirror::FrontBack => x = -x,
        }
        let (x, z) = match self.rotation & 3 {
            0 => (x, z),
            1 => (-z, x),
            2 => (-x, -z),
            _ => (z, -x),
        };
        [self.origin[0] + x, self.origin[1] + y, self.origin[2] + z]
    }
    /// A horizontal direction after the mirror and the turn.
    pub fn dir<'a>(&self, d: &'a str) -> &'a str {
        const RING: [&str; 4] = ["north", "east", "south", "west"];
        let Some(mut i) = RING.iter().position(|&r| r == d) else {
            return d;
        };
        match self.mirror {
            Mirror::LeftRight if i % 2 == 0 => i = 2 - i,
            Mirror::FrontBack if i % 2 == 1 => i = 4 - i,
            _ => {}
        }
        RING[(i + usize::from(self.rotation & 3)) % 4]
    }
    /// A block state after the mirror and the turn (`BlockState.mirror`,
    /// then `rotate`), for the properties blocks turn by.
    pub fn state(&self, s: &State) -> State {
        if self.rotation & 3 == 0 && self.mirror == Mirror::None {
            return s.clone();
        }
        let mut out = s.clone();
        let mirrored = self.mirror != Mirror::None;
        for (k, v) in &s.props {
            match k.as_str() {
                "facing" | "horizontal_facing" => out.set(k, self.dir(v)),
                "axis" if self.rotation & 1 == 1 => out.set(k, match v.as_str() {
                    "x" => "z",
                    "z" => "x",
                    o => o,
                }),
                "rotation" => {
                    if let Ok(r) = v.parse::<i32>() {
                        let r = match self.mirror {
                            Mirror::None => r,
                            Mirror::LeftRight => (8 - r).rem_euclid(16),
                            Mirror::FrontBack => (16 - r).rem_euclid(16),
                        };
                        out.set(k, &((r + 4 * i32::from(self.rotation & 3)).rem_euclid(16)).to_string());
                    }
                }
                "north" | "east" | "south" | "west" => {
                    let to = self.dir(k);
                    out.set(to, v);
                }
                "shape" => {
                    let swapped = if mirrored {
                        match v.as_str() {
                            "inner_left" => "inner_right".to_owned(),
                            "inner_right" => "inner_left".to_owned(),
                            "outer_left" => "outer_right".to_owned(),
                            "outer_right" => "outer_left".to_owned(),
                            o => o.to_owned(),
                        }
                    } else {
                        v.clone()
                    };
                    let swapped = if swapped.contains('_') && !swapped.starts_with("inner") && !swapped.starts_with("outer") {
                        self.rail(&swapped)
                    } else {
                        swapped
                    };
                    out.set(k, &swapped);
                }
                "hinge" if mirrored => out.set(k, if v == "left" { "right" } else { "left" }),
                "type" if mirrored && (v == "left" || v == "right") => {
                    out.set(k, if v == "left" { "right" } else { "left" })
                }
                _ => {}
            }
        }
        out
    }
    /// A rail's shape: `north_south`, `ascending_east`, `south_west`...
    fn rail(&self, shape: &str) -> String {
        if let Some(d) = shape.strip_prefix("ascending_") {
            return format!("ascending_{}", self.dir(d));
        }
        let parts: Vec<&str> = shape.split('_').map(|p| self.dir(p)).collect();
        if parts.len() != 2 {
            return shape.to_owned();
        }
        // Vanilla names: north_south, east_west, and corners with
        // north/south first.
        let (a, b) = (parts[0], parts[1]);
        let ns = |d: &str| d == "north" || d == "south";
        match (ns(a), ns(b)) {
            (true, true) => "north_south".into(),
            (false, false) => "east_west".into(),
            (true, false) => format!("{a}_{b}"),
            (false, true) => format!("{b}_{a}"),
        }
    }
}

/// A placed schematic's blocks: the states (turned) and each block's world
/// position with its index into them. Air is left out.
pub struct Layout {
    pub states: Vec<State>,
    pub blocks: Vec<([i32; 3], u32)>,
    pub min: [i32; 3],
    pub max: [i32; 3],
}
pub fn layout(schematic: &Schematic, t: &Transform) -> Layout {
    let mut states = Vec::new();
    let mut blocks = Vec::new();
    let mut min = [i32::MAX; 3];
    let mut max = [i32::MIN; 3];
    for r in &schematic.regions {
        let base = states.len() as u32;
        states.extend(r.palette.iter().map(|s| t.state(s)));
        let air: Vec<bool> = r.palette.iter().map(State::is_air).collect();
        for y in 0..r.size[1] {
            for z in 0..r.size[2] {
                for x in 0..r.size[0] {
                    let i = r.index(x, y, z);
                    if air.get(i).copied().unwrap_or(true) {
                        continue;
                    }
                    let p = t.pos([r.min[0] + x, r.min[1] + y, r.min[2] + z]);
                    for a in 0..3 {
                        min[a] = min[a].min(p[a]);
                        max[a] = max[a].max(p[a]);
                    }
                    blocks.push((p, base + i as u32));
                }
            }
        }
    }
    if blocks.is_empty() {
        min = t.origin;
        max = t.origin;
    }
    Layout { states, blocks, min, max }
}

/// How many of each block a layout holds, most first (the material list).
pub fn materials(l: &Layout) -> Vec<(String, u32)> {
    let mut counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for &(_, s) in &l.blocks {
        *counts.entry(&l.states[s as usize].name).or_default() += 1;
    }
    let mut out: Vec<(String, u32)> = counts.into_iter().map(|(k, v)| (k.to_owned(), v)).collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

#[cfg(test)]
pub(crate) fn pack_bits(values: &[usize], bits: u32) -> Vec<i64> {
    let total = values.len() as u64 * u64::from(bits);
    let mut out = vec![0u64; total.div_ceil(64) as usize];
    for (n, &v) in values.iter().enumerate() {
        let i = n as u64 * u64::from(bits);
        let (start, off) = ((i >> 6) as usize, (i & 63) as u32);
        out[start] |= (v as u64) << off;
        if off + bits > 64 {
            out[start + 1] |= (v as u64) >> (64 - off);
        }
    }
    out.into_iter().map(|v| v as i64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn compound(items: Vec<(&str, Tag)>) -> Tag {
        Tag::Compound(items.into_iter().map(|(k, v)| (k.to_owned(), v)).collect::<HashMap<_, _>>())
    }
    fn vec3(x: i32, y: i32, z: i32) -> Tag {
        compound(vec![("x", Tag::Int(x)), ("y", Tag::Int(y)), ("z", Tag::Int(z))])
    }
    fn state(name: &str, props: &[(&str, &str)]) -> Tag {
        let mut items = vec![("Name", Tag::String(name.into()))];
        if !props.is_empty() {
            items.push(("Properties", compound(props.iter().map(|(k, v)| (*k, Tag::String((*v).into()))).collect())));
        }
        compound(items)
    }

    /// A 3 x 2 x 2 region growing towards -x, five palette entries (3 bits,
    /// so entries cross the longs), gzipped as Litematica writes it.
    fn file() -> Vec<u8> {
        let palette = vec![
            state("minecraft:air", &[]),
            state("minecraft:stone", &[]),
            state("minecraft:oak_log", &[("axis", "x")]),
            state("minecraft:oak_stairs", &[("facing", "east"), ("half", "bottom"), ("shape", "inner_left"), ("waterlogged", "false")]),
            state("minecraft:glass", &[]),
        ];
        // 3 x 2 x 2 = 12 entries; index y*6 + z*3 + x.
        let values: Vec<usize> = (0..12).map(|i| i % 5).collect();
        let region = compound(vec![
            ("Position", vec3(2, 0, 0)),
            ("Size", vec3(-3, 2, 2)),
            ("BlockStatePalette", Tag::List(palette)),
            ("BlockStates", Tag::LongArray(pack_bits(&values, 3))),
        ]);
        let root = compound(vec![
            ("MinecraftDataVersion", Tag::Int(4000)),
            ("Version", Tag::Int(7)),
            ("Metadata", compound(vec![("Name", Tag::String("Hut".into())), ("Author", Tag::String("Timur".into()))])),
            ("Regions", compound(vec![("Main", region)])),
        ]);
        let raw = nbt::write(&root);
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        std::io::Write::write_all(&mut gz, &raw).unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn a_litematic_file_reads_with_its_regions_and_packed_blocks() {
        let s = parse(&file()).unwrap();
        assert_eq!((s.name.as_str(), s.author.as_str(), s.data_version), ("Hut", "Timur", 4000));
        let r = &s.regions[0];
        assert_eq!(r.min, [0, 0, 0]);
        assert_eq!(r.size, [3, 2, 2]);
        for i in 0..12 {
            let (x, z, y) = (i % 3, i / 3 % 2, i / 6);
            assert_eq!(r.index(x, y, z), i as usize % 5, "entry {i}");
        }
        assert_eq!(r.palette[3].prop("shape"), Some("inner_left"));
        let l = layout(&s, &Transform { origin: [100, 64, -5], ..Transform::default() });
        // Air (index 0) three times in 12 entries.
        assert_eq!(l.blocks.len(), 9);
        assert_eq!(l.min, [100, 64, -5]);
        assert_eq!(l.max, [102, 65, -4]);
        assert_eq!(materials(&l)[0].1, 3);
    }

    #[test]
    fn turning_moves_blocks_and_turns_their_states() {
        let t = Transform { origin: [0, 0, 0], rotation: 1, mirror: Mirror::None };
        // Clockwise from above: east goes south, (1, 0, 0) goes to (0, 0, 1).
        assert_eq!(t.pos([1, 5, 0]), [0, 5, 1]);
        assert_eq!(t.dir("east"), "south");
        let fence = State {
            name: "minecraft:oak_fence".into(),
            props: vec![("east".into(), "true".into()), ("north".into(), "false".into()), ("south".into(), "false".into()), ("west".into(), "false".into())],
        };
        let turned = t.state(&fence);
        assert_eq!(turned.prop("south"), Some("true"));
        assert_eq!(turned.prop("east"), Some("false"));
        let log = State { name: "minecraft:oak_log".into(), props: vec![("axis".into(), "x".into())] };
        assert_eq!(t.state(&log).prop("axis"), Some("z"));
        let sign = State { name: "minecraft:oak_sign".into(), props: vec![("rotation".into(), "2".into())] };
        assert_eq!(t.state(&sign).prop("rotation"), Some("6"));
        let rail = State { name: "minecraft:rail".into(), props: vec![("shape".into(), "north_east".into())] };
        assert_eq!(t.state(&rail).prop("shape"), Some("south_east"));
    }

    #[test]
    fn mirroring_flips_sides_and_handedness() {
        let t = Transform { origin: [0, 0, 0], rotation: 0, mirror: Mirror::LeftRight };
        assert_eq!(t.pos([1, 0, 2]), [1, 0, -2]);
        assert_eq!(t.dir("north"), "south");
        assert_eq!(t.dir("east"), "east");
        let stairs = State {
            name: "minecraft:oak_stairs".into(),
            props: vec![("facing".into(), "north".into()), ("shape".into(), "outer_left".into())],
        };
        let m = t.state(&stairs);
        assert_eq!((m.prop("facing"), m.prop("shape")), (Some("south"), Some("outer_right")));
        let door = State { name: "minecraft:oak_door".into(), props: vec![("hinge".into(), "left".into())] };
        assert_eq!(t.state(&door).prop("hinge"), Some("right"));
        let f = Transform { mirror: Mirror::FrontBack, ..t };
        assert_eq!(f.dir("east"), "west");
        assert_eq!(f.pos([3, 0, 1]), [-3, 0, 1]);
        let sign = State { name: "minecraft:oak_sign".into(), props: vec![("rotation".into(), "4".into())] };
        // Facing west (4) mirrored front to back faces east (12).
        assert_eq!(f.state(&sign).prop("rotation"), Some("12"));
        // Facing south (0) mirrored left to right faces north (8).
        let south = State { name: "minecraft:oak_sign".into(), props: vec![("rotation".into(), "0".into())] };
        assert_eq!(t.state(&south).prop("rotation"), Some("8"));
    }
}
