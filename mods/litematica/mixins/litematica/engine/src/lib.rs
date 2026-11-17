//! Litematica for Horizon: schematics (`.litematic`) placed in the world and
//! drawn where their blocks are missing or wrong, a material list, and the
//! printer, which places the missing blocks within reach from the hotbar.
//! Written for Horizon; the file format is Litematica's.
pub mod nbt;
pub mod schematic;

use schematic::{Layout, Mirror, Schematic, State, Transform};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const ID: &str = "litematica";
pub const NAME: &str = "Litematica";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str =
    "Schematics (.litematic) shown in the world, with a material list and a printer. /lm for the commands.";
pub const COMMAND: &str = "lm";
pub const KEY_RENDER: &str = "toggle_render";
pub const KEY_PRINTER: &str = "toggle_printer";
pub const DEFAULT_RENDER: &str = "key.keyboard.unknown";
pub const DEFAULT_PRINTER: &str = "key.keyboard.caps.lock";
pub const UNBOUND: &str = "key.keyboard.unknown";
/// Blocks within this many sections (16 blocks) of the camera are drawn.
pub const DRAW_SECTIONS: i32 = 3;
/// Blocks drawn at most a frame.
pub const DRAW_MAX: usize = 6000;
/// How far the printer reaches, from the eyes.
pub const REACH: f64 = 4.5;
/// Ticks between two blocks the printer places.
pub const PRINT_EVERY: u32 = 2;

/// Which layers of the schematics show (and print).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layers {
    #[default]
    All,
    Only(i32),
    Below(i32),
    Above(i32),
}
impl Layers {
    pub fn shows(self, y: i32) -> bool {
        match self {
            Layers::All => true,
            Layers::Only(l) => y == l,
            Layers::Below(l) => y <= l,
            Layers::Above(l) => y >= l,
        }
    }
}

/// A schematic placed in the world.
pub struct Placement {
    pub file: String,
    pub schematic: Schematic,
    pub t: Transform,
    pub layout: Layout,
    /// The blocks by section (`pos >> 4`), indices into `layout.blocks`.
    pub sections: HashMap<[i32; 3], Vec<u32>>,
    /// Each of `layout.states` as the game's block state id, once resolved.
    pub ids: Vec<Option<u32>>,
    pub resolved: bool,
    pub shown: bool,
}
impl Placement {
    pub fn new(file: String, schematic: Schematic, t: Transform) -> Self {
        let mut p = Self {
            file,
            schematic,
            t,
            layout: Layout { states: Vec::new(), blocks: Vec::new(), min: [0; 3], max: [0; 3] },
            sections: HashMap::new(),
            ids: Vec::new(),
            resolved: false,
            shown: true,
        };
        p.relayout();
        p
    }
    /// Lay the blocks out again after the transform changed.
    pub fn relayout(&mut self) {
        self.layout = schematic::layout(&self.schematic, &self.t);
        self.sections.clear();
        for (i, (p, _)) in self.layout.blocks.iter().enumerate() {
            self.sections.entry([p[0] >> 4, p[1] >> 4, p[2] >> 4]).or_default().push(i as u32);
        }
        self.ids.clear();
        self.resolved = false;
    }
}

/// Everything the mod keeps.
#[derive(Default)]
pub struct Mod {
    pub placements: Vec<Placement>,
    pub selected: usize,
    pub hidden: bool,
    pub layers: Layers,
    pub printer: bool,
    /// Ticks until the printer may place again.
    pub cooldown: u32,
    /// The world the placements belong to; they are saved under it.
    pub world: Option<String>,
    /// Placements to save (`take_save`).
    pub save: bool,
}
thread_local! {
    static MOD: RefCell<Mod> = RefCell::new(Mod::default());
}
pub fn with<R>(f: impl FnOnce(&mut Mod) -> R) -> R {
    MOD.with(|m| f(&mut m.borrow_mut()))
}

/// What a command needs of the game.
pub struct Ctx<'a> {
    pub russian: bool,
    pub dir: &'a Path,
    /// The block the player stands in.
    pub feet: [i32; 3],
    /// The direction the player faces, `north`...
    pub facing: &'a str,
    /// How many of each item the player has (for `/lm materials`).
    pub have: &'a dyn Fn(&str) -> u32,
    /// Whether the block at a position is already what the schematic wants
    /// (for `/lm materials`).
    pub done: &'a dyn Fn([i32; 3], &State) -> bool,
}

fn t(ru: bool, en: &str, ruw: &str) -> String {
    if ru { ruw.to_owned() } else { en.to_owned() }
}

/// The `.litematic` files in `dir`, sorted.
pub fn files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    while let Some((d, prefix)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let path = e.path();
            if path.is_dir() && prefix.matches('/').count() < 4 {
                stack.push((path, format!("{prefix}{name}/")));
            } else if name.to_ascii_lowercase().ends_with(".litematic") {
                out.push(format!("{prefix}{}", &name[..name.len() - ".litematic".len()]));
            }
        }
    }
    out.sort_by_key(|s| s.to_lowercase());
    out
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let direct = dir.join(format!("{name}.litematic"));
    if direct.is_file() {
        return Some(direct);
    }
    let lower = name.to_lowercase();
    let all = files(dir);
    let hit = all
        .iter()
        .find(|f| f.to_lowercase() == lower)
        .or_else(|| all.iter().find(|f| f.to_lowercase().rsplit('/').next() == Some(&lower)))
        .or_else(|| {
            let starts: Vec<_> = all.iter().filter(|f| f.to_lowercase().contains(&lower)).collect();
            (starts.len() == 1).then(|| starts[0])
        })?;
    Some(dir.join(format!("{hit}.litematic")))
}

/// Runs `/lm ...`; None when the command isn't ours. Returns the lines to
/// show in the chat.
pub fn command(line: &str, ctx: &Ctx<'_>) -> Option<Vec<String>> {
    let mut words = line.split_whitespace();
    if words.next()? != COMMAND {
        return None;
    }
    let args: Vec<&str> = words.collect();
    Some(with(|m| m.run(&args, ctx)).unwrap_or_else(|e| vec![format!("§c{e}")]))
}

fn int(s: Option<&&str>, base: i32) -> Result<i32, String> {
    let s = s.ok_or("a number is missing")?;
    if let Some(rel) = s.strip_prefix('~') {
        return Ok(base + if rel.is_empty() { 0 } else { rel.parse::<i32>().map_err(|_| format!("not a number: {s}"))? });
    }
    s.parse().map_err(|_| format!("not a number: {s}"))
}

impl Mod {
    fn current(&mut self) -> Result<&mut Placement, String> {
        let i = self.selected;
        self.placements.get_mut(i).ok_or_else(|| "no schematic is placed: /lm load <name>".to_owned())
    }

    fn run(&mut self, args: &[&str], ctx: &Ctx<'_>) -> Result<Vec<String>, String> {
        let ru = ctx.russian;
        let sub = args.first().copied().unwrap_or("help");
        let mut out = Vec::new();
        match sub {
            "help" | "?" => {
                out.push(t(ru, "§6Litematica§r: /lm ...", "§6Litematica§r: /lm ..."));
                for (cmd, en, rus) in [
                    ("list", "the schematics in schematics/", "схемы из папки schematics/"),
                    ("load <name>", "place one at your feet", "поставить схему у ног"),
                    ("here", "move it to your feet", "перенести к ногам"),
                    ("move <x> <y> <z>", "move its origin (~ works)", "сдвинуть начало (~ можно)"),
                    ("nudge <dir> [n]", "move it n blocks: up, down, north, forward...", "сдвинуть на n блоков: up, down, north, forward..."),
                    ("rotate [cw|ccw]", "turn it a quarter", "повернуть на четверть"),
                    ("mirror <none|x|z>", "mirror it", "отразить"),
                    ("layer <all|y|below y|above y|here>", "which layers show", "какие слои видны"),
                    ("printer [on|off]", "place missing blocks from the hotbar", "ставить недостающие блоки из хотбара"),
                    ("materials", "what is still needed", "сколько чего ещё нужно"),
                    ("show / hide", "show or hide schematics", "показать или скрыть схемы"),
                    ("select <n>", "choose a placement", "выбрать схему"),
                    ("unload [all]", "remove it", "убрать схему"),
                    ("info", "the placements", "список поставленных"),
                ] {
                    out.push(format!("§e/lm {cmd}§r — {}", if ru { rus } else { en }));
                }
            }
            "list" => {
                let all = files(ctx.dir);
                if all.is_empty() {
                    out.push(t(ru, &format!("No .litematic files in {}", ctx.dir.display()), &format!("В {} нет файлов .litematic", ctx.dir.display())));
                } else {
                    out.push(t(ru, "Schematics:", "Схемы:"));
                    for f in all.iter().take(40) {
                        out.push(format!(" §b{f}"));
                    }
                    if all.len() > 40 {
                        out.push(format!(" … +{}", all.len() - 40));
                    }
                }
            }
            "load" => {
                let name = args[1..].join(" ");
                if name.is_empty() {
                    return Err(t(ru, "/lm load <name>; /lm list shows them", "/lm load <имя>; список: /lm list"));
                }
                let path = find_file(ctx.dir, &name)
                    .ok_or_else(|| t(ru, &format!("No schematic {name} in {}", ctx.dir.display()), &format!("Нет схемы {name} в {}", ctx.dir.display())))?;
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let schematic = schematic::parse(&bytes)?;
                let file = path
                    .strip_prefix(ctx.dir)
                    .unwrap_or(&path)
                    .with_extension("")
                    .to_string_lossy()
                    .replace('\\', "/");
                let p = Placement::new(file, schematic, Transform { origin: ctx.feet, ..Transform::default() });
                let count = p.layout.blocks.len();
                let size = [p.layout.max[0] - p.layout.min[0] + 1, p.layout.max[1] - p.layout.min[1] + 1, p.layout.max[2] - p.layout.min[2] + 1];
                let title = if p.schematic.name.is_empty() { p.file.clone() } else { p.schematic.name.clone() };
                self.placements.push(p);
                self.selected = self.placements.len() - 1;
                self.hidden = false;
                self.save = true;
                out.push(t(
                    ru,
                    &format!("Placed §b{title}§r at {} {} {}: {}x{}x{}, {count} blocks", ctx.feet[0], ctx.feet[1], ctx.feet[2], size[0], size[1], size[2]),
                    &format!("Схема §b{title}§r поставлена у {} {} {}: {}x{}x{}, блоков: {count}", ctx.feet[0], ctx.feet[1], ctx.feet[2], size[0], size[1], size[2]),
                ));
            }
            "here" => {
                let p = self.current()?;
                p.t.origin = ctx.feet;
                p.relayout();
                self.save = true;
                out.push(t(ru, "Moved to your feet", "Перенесена к ногам"));
            }
            "move" => {
                let p = self.current()?;
                let feet = ctx.feet;
                let to = [int(args.get(1), feet[0])?, int(args.get(2), feet[1])?, int(args.get(3), feet[2])?];
                p.t.origin = to;
                p.relayout();
                self.save = true;
                out.push(t(ru, &format!("Origin at {} {} {}", to[0], to[1], to[2]), &format!("Начало в {} {} {}", to[0], to[1], to[2])));
            }
            "nudge" => {
                let dir = args.get(1).copied().unwrap_or("up");
                let n = args.get(2).map_or(Ok(1), |s| s.parse::<i32>().map_err(|_| format!("not a number: {s}")))?;
                let facing = ctx.facing;
                let d = match dir {
                    "forward" | "f" => facing,
                    "back" | "b" => opposite(facing),
                    "left" | "l" => ccw(facing),
                    "right" | "r" => cw(facing),
                    d => d,
                };
                let v = match d {
                    "up" | "u" => [0, 1, 0],
                    "down" | "d" => [0, -1, 0],
                    "north" | "n" => [0, 0, -1],
                    "south" | "s" => [0, 0, 1],
                    "east" | "e" => [1, 0, 0],
                    "west" | "w" => [-1, 0, 0],
                    _ => return Err(format!("{dir}? up, down, north, south, east, west, forward, back, left, right")),
                };
                let p = self.current()?;
                for a in 0..3 {
                    p.t.origin[a] += v[a] * n;
                }
                p.relayout();
                let o = p.t.origin;
                self.save = true;
                out.push(t(ru, &format!("Origin at {} {} {}", o[0], o[1], o[2]), &format!("Начало в {} {} {}", o[0], o[1], o[2])));
            }
            "rotate" => {
                let back = matches!(args.get(1).copied(), Some("ccw" | "left" | "-90" | "270"));
                let p = self.current()?;
                p.t.rotation = (p.t.rotation + if back { 3 } else { 1 }) & 3;
                p.relayout();
                let deg = u32::from(p.t.rotation) * 90;
                self.save = true;
                out.push(t(ru, &format!("Turned {deg}°"), &format!("Повёрнута на {deg}°")));
            }
            "mirror" => {
                let m = match args.get(1).copied() {
                    Some("none" | "off") => Mirror::None,
                    Some("x" | "front_back") => Mirror::FrontBack,
                    Some("z" | "left_right") => Mirror::LeftRight,
                    _ => return Err("/lm mirror <none|x|z>".into()),
                };
                let p = self.current()?;
                p.t.mirror = m;
                p.relayout();
                self.save = true;
                out.push(t(ru, "Mirrored", "Отражена"));
            }
            "layer" | "layers" => {
                let y = ctx.feet[1];
                self.layers = match (args.get(1).copied(), args.get(2)) {
                    (Some("all") | None, _) => Layers::All,
                    (Some("here"), _) => Layers::Only(y),
                    (Some("below"), n) => Layers::Below(int(n.or(Some(&"~")), y)?),
                    (Some("above"), n) => Layers::Above(int(n.or(Some(&"~")), y)?),
                    (Some(n), _) => Layers::Only(int(Some(&n), y)?),
                };
                out.push(match self.layers {
                    Layers::All => t(ru, "All layers", "Все слои"),
                    Layers::Only(l) => t(ru, &format!("Layer y = {l}"), &format!("Слой y = {l}")),
                    Layers::Below(l) => t(ru, &format!("Layers up to y = {l}"), &format!("Слои до y = {l}")),
                    Layers::Above(l) => t(ru, &format!("Layers from y = {l}"), &format!("Слои от y = {l}")),
                });
            }
            "printer" => {
                self.printer = match args.get(1).copied() {
                    Some("on") => true,
                    Some("off") => false,
                    _ => !self.printer,
                };
                out.push(printer_line(ru, self.printer));
            }
            "show" | "hide" | "render" => {
                self.hidden = match (sub, args.get(1).copied()) {
                    ("show", _) | (_, Some("on")) => false,
                    ("hide", _) | (_, Some("off")) => true,
                    _ => !self.hidden,
                };
                out.push(if self.hidden { t(ru, "Schematics hidden", "Схемы скрыты") } else { t(ru, "Schematics shown", "Схемы видны") });
            }
            "select" => {
                let n = args.get(1).and_then(|s| s.parse::<usize>().ok()).ok_or("/lm select <n> (/lm info)")?;
                if n == 0 || n > self.placements.len() {
                    return Err(format!("1..{}", self.placements.len()));
                }
                self.selected = n - 1;
                out.push(t(ru, &format!("Selected {}", self.placements[n - 1].file), &format!("Выбрана {}", self.placements[n - 1].file)));
            }
            "unload" | "remove" => {
                if args.get(1) == Some(&"all") {
                    self.placements.clear();
                    self.selected = 0;
                } else {
                    let i = self.selected;
                    if i >= self.placements.len() {
                        return Err(t(ru, "Nothing to remove", "Нечего убирать"));
                    }
                    self.placements.remove(i);
                    self.selected = self.placements.len().saturating_sub(1);
                }
                self.save = true;
                out.push(t(ru, "Removed", "Убрана"));
            }
            "info" => {
                if self.placements.is_empty() {
                    out.push(t(ru, "Nothing placed. /lm load <name>", "Ничего не поставлено. /lm load <имя>"));
                }
                for (i, p) in self.placements.iter().enumerate() {
                    let o = p.t.origin;
                    out.push(format!(
                        "{}{}. §b{}§r {} {} {}, {}°{}",
                        if i == self.selected { "§e> " } else { "  " },
                        i + 1,
                        p.file,
                        o[0],
                        o[1],
                        o[2],
                        u32::from(p.t.rotation) * 90,
                        match p.t.mirror {
                            Mirror::None => "",
                            Mirror::LeftRight => ", mirror z",
                            Mirror::FrontBack => ", mirror x",
                        }
                    ));
                }
                out.push(printer_line(ru, self.printer));
            }
            "materials" | "mats" => {
                let p = self.current()?;
                let mut need: HashMap<&str, u32> = HashMap::new();
                for (pos, s) in &p.layout.blocks {
                    let st = &p.layout.states[*s as usize];
                    if !(ctx.done)(*pos, st) {
                        *need.entry(item_of(&st.name)).or_default() += 1;
                    }
                }
                let mut list: Vec<(&str, u32)> = need.into_iter().collect();
                list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
                if list.is_empty() {
                    out.push(t(ru, "§aEverything is built", "§aВсё построено"));
                } else {
                    out.push(t(ru, "Still needed (have / need):", "Ещё нужно (есть / нужно):"));
                    for (item, n) in list.iter().take(30) {
                        let have = (ctx.have)(item);
                        let colour = if have >= *n { "§a" } else { "§c" };
                        out.push(format!(" {colour}{have}§r / {n} {}", item.trim_start_matches("minecraft:")));
                    }
                    if list.len() > 30 {
                        out.push(format!(" … +{}", list.len() - 30));
                    }
                }
            }
            other => return Err(t(ru, &format!("Unknown: {other}. /lm help"), &format!("Не знаю «{other}». /lm help"))),
        }
        Ok(out)
    }
}

pub fn printer_line(ru: bool, on: bool) -> String {
    match (ru, on) {
        (true, true) => "Принтер §aвключён".into(),
        (true, false) => "Принтер §cвыключен".into(),
        (false, true) => "Printer §aon".into(),
        (false, false) => "Printer §coff".into(),
    }
}

/// The item a block is placed from, where it isn't the block's own name.
pub fn item_of(block: &str) -> &str {
    match block {
        "minecraft:redstone_wire" => "minecraft:redstone",
        "minecraft:tripwire" => "minecraft:string",
        "minecraft:wall_torch" => "minecraft:torch",
        "minecraft:soul_wall_torch" => "minecraft:soul_torch",
        "minecraft:redstone_wall_torch" => "minecraft:redstone_torch",
        "minecraft:copper_wall_torch" => "minecraft:copper_torch",
        "minecraft:water" => "minecraft:water_bucket",
        "minecraft:lava" => "minecraft:lava_bucket",
        "minecraft:wheat" => "minecraft:wheat_seeds",
        "minecraft:carrots" => "minecraft:carrot",
        "minecraft:potatoes" => "minecraft:potato",
        "minecraft:beetroots" => "minecraft:beetroot_seeds",
        "minecraft:cocoa" => "minecraft:cocoa_beans",
        "minecraft:sweet_berry_bush" => "minecraft:sweet_berries",
        "minecraft:bubble_column" => "minecraft:water_bucket",
        "minecraft:powder_snow" => "minecraft:powder_snow_bucket",
        "minecraft:melon_stem" | "minecraft:attached_melon_stem" => "minecraft:melon_seeds",
        "minecraft:pumpkin_stem" | "minecraft:attached_pumpkin_stem" => "minecraft:pumpkin_seeds",
        "minecraft:tall_seagrass" => "minecraft:seagrass",
        "minecraft:kelp_plant" => "minecraft:kelp",
        "minecraft:cave_vines_plant" | "minecraft:cave_vines" => "minecraft:glow_berries",
        "minecraft:twisting_vines_plant" => "minecraft:twisting_vines",
        "minecraft:weeping_vines_plant" => "minecraft:weeping_vines",
        "minecraft:bamboo_sapling" => "minecraft:bamboo",
        "minecraft:big_dripleaf_stem" => "minecraft:big_dripleaf",
        "minecraft:pitcher_crop" => "minecraft:pitcher_pod",
        "minecraft:torchflower_crop" => "minecraft:torchflower_seeds",
        b => {
            if let Some(rest) = b.strip_suffix("_wall_sign") {
                return leak(format!("{rest}_sign"));
            }
            if let Some(rest) = b.strip_suffix("_wall_hanging_sign") {
                return leak(format!("{rest}_hanging_sign"));
            }
            if let Some(rest) = b.strip_suffix("_wall_banner") {
                return leak(format!("{rest}_banner"));
            }
            if b.ends_with("_wall_head") || b.ends_with("_wall_skull") {
                return leak(b.replace("_wall_", "_"));
            }
            if b.starts_with("minecraft:potted_") {
                // A potted plant: the pot goes first.
                return "minecraft:flower_pot";
            }
            b
        }
    }
}
fn leak(s: String) -> &'static str {
    thread_local! {
        static NAMES: RefCell<HashMap<String, &'static str>> = RefCell::new(HashMap::new());
    }
    NAMES.with(|n| *n.borrow_mut().entry(s.clone()).or_insert_with(|| Box::leak(s.into_boxed_str())))
}

pub fn opposite(d: &str) -> &'static str {
    match d {
        "north" => "south",
        "south" => "north",
        "east" => "west",
        "west" => "east",
        "up" => "down",
        _ => "up",
    }
}
pub fn cw(d: &str) -> &'static str {
    match d {
        "north" => "east",
        "east" => "south",
        "south" => "west",
        _ => "north",
    }
}
pub fn ccw(d: &str) -> &'static str {
    match d {
        "north" => "west",
        "west" => "south",
        "south" => "east",
        _ => "north",
    }
}
pub fn vector(d: &str) -> [i32; 3] {
    match d {
        "north" => [0, 0, -1],
        "south" => [0, 0, 1],
        "east" => [1, 0, 0],
        "west" => [-1, 0, 0],
        "up" => [0, 1, 0],
        _ => [0, -1, 0],
    }
}
/// The direction a yaw (degrees, 0 south, 90 west) faces.
pub fn facing_of(yaw: f32) -> &'static str {
    let i = ((f64::from(yaw) / 90.0).round() as i64).rem_euclid(4);
    ["south", "west", "north", "east"][i as usize]
}
/// The yaw that faces a horizontal direction.
pub fn yaw_of(d: &str) -> f32 {
    match d {
        "south" => 0.0,
        "west" => 90.0,
        "north" => 180.0,
        _ => -90.0,
    }
}
/// The look (yaw, pitch) along a direction, up and down included.
fn look_along(d: &str, yaw: f32) -> (f32, f32) {
    match d {
        "up" => (yaw, -90.0),
        "down" => (yaw, 90.0),
        d => (yaw_of(d), 0.0),
    }
}

/// A click the printer makes to place a block: the block clicked, its face
/// (towards the block placed), where on it (from its corner), and the look
/// to send first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Click {
    pub pos: [i32; 3],
    pub face: [i32; 3],
    pub cursor: [f32; 3],
    pub look: Option<(f32, f32)>,
}

/// How a block's `facing` comes from the click (`getStateForPlacement`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rule {
    /// Facing where the player looks horizontally (stairs, doors, gates, beds).
    Look,
    /// Facing back at the player horizontally (furnaces, chests, most).
    Back,
    /// Facing where the player looks, up and down too (observer).
    Look6,
    /// Facing back at the player, up and down too (pistons, dispensers, barrels).
    Back6,
    /// The face clicked (end rods, lightning rods, shulker boxes, amethyst).
    Face,
    /// Against the face clicked (hoppers).
    AgainstFace,
    /// On a wall: the clicked face of the block behind (wall torches, ladders, wall signs).
    Wall,
    /// Turned clockwise from the look (anvils).
    Clockwise,
}
fn rule(name: &str) -> Rule {
    let n = name.trim_start_matches("minecraft:");
    if n.ends_with("_stairs") || n.ends_with("_door") || n.ends_with("_fence_gate") || n.ends_with("_bed") || n.ends_with("campfire") {
        Rule::Look
    } else if n == "observer" {
        Rule::Look6
    } else if matches!(n, "piston" | "sticky_piston" | "dispenser" | "dropper" | "barrel" | "command_block" | "chain_command_block" | "repeating_command_block" | "crafter") {
        Rule::Back6
    } else if n == "end_rod" || n == "lightning_rod" || n.ends_with("shulker_box") || n.ends_with("amethyst_bud") || n == "amethyst_cluster" {
        Rule::Face
    } else if n == "hopper" {
        Rule::AgainstFace
    } else if n.contains("wall_") || n == "ladder" || n == "tripwire_hook" || n == "cocoa" {
        Rule::Wall
    } else if n.ends_with("anvil") {
        Rule::Clockwise
    } else {
        Rule::Back
    }
}

/// How the printer places block `want` at `pos`: the click, or None when it
/// can't from here (no block to click where the placement needs it, or a
/// part another part places, as a door's top half). `solid` says whether a
/// block may be clicked; `yaw` is the player's.
pub fn plan(want: &State, pos: [i32; 3], yaw: f32, solid: &dyn Fn([i32; 3]) -> bool) -> Option<Click> {
    if want.prop("half") == Some("upper") || want.prop("part") == Some("head") {
        return None;
    }
    let add = |p: [i32; 3], v: [i32; 3]| [p[0] + v[0], p[1] + v[1], p[2] + v[2]];
    let neg = |v: [i32; 3]| [-v[0], -v[1], -v[2]];
    // Click the face of `n` that faces `pos`.
    let click_from = |n: [i32; 3], y: Option<f32>, look: Option<(f32, f32)>| -> Option<Click> {
        if !solid(n) {
            return None;
        }
        let face = [pos[0] - n[0], pos[1] - n[1], pos[2] - n[2]];
        let mut cursor = [0.5 + 0.5 * face[0] as f32, 0.5 + 0.5 * face[1] as f32, 0.5 + 0.5 * face[2] as f32];
        if let Some(y) = y {
            if face[1] == 0 {
                cursor[1] = y;
            }
        }
        Some(Click { pos: n, face, cursor, look })
    };
    // Top or bottom halves (slabs, stairs, trapdoors).
    let half = match (want.prop("type"), want.prop("half")) {
        (Some("top"), _) | (_, Some("top")) => Some(true),
        (Some("bottom"), _) | (_, Some("bottom")) => Some(false),
        _ => None,
    };
    let sides = ["north", "south", "east", "west"];
    let all6 = ["down", "up", "north", "south", "east", "west"];
    // Neighbours to try for a block that only cares about a half.
    let by_half = |look: Option<(f32, f32)>| -> Option<Click> {
        match half {
            Some(top) => {
                let first = if top { add(pos, [0, 1, 0]) } else { add(pos, [0, -1, 0]) };
                click_from(first, None, look).or_else(|| {
                    sides.iter().find_map(|d| click_from(add(pos, vector(d)), Some(if top { 0.9 } else { 0.1 }), look))
                })
            }
            None => all6.iter().find_map(|d| click_from(add(pos, vector(d)), None, look)),
        }
    };
    // Trapdoors: a side face sets the facing (its normal) and the cursor the half.
    if want.name.ends_with("_trapdoor") {
        let f = want.prop("facing")?;
        let top = half == Some(true);
        if let Some(c) = click_from(add(pos, neg(vector(f))), Some(if top { 0.9 } else { 0.1 }), None) {
            return Some(c);
        }
        let n = if top { add(pos, [0, 1, 0]) } else { add(pos, [0, -1, 0]) };
        return click_from(n, None, Some((yaw_of(opposite(f)), 0.0)));
    }
    // Buttons, levers, grindstones: the face they're on.
    if let Some(face) = want.prop("face") {
        let f = want.prop("facing").unwrap_or("north");
        return match face {
            "floor" => click_from(add(pos, [0, -1, 0]), None, Some((yaw_of(f), 90.0))),
            "ceiling" => click_from(add(pos, [0, 1, 0]), None, Some((yaw_of(f), -90.0))),
            _ => click_from(add(pos, neg(vector(f))), None, Some((yaw_of(opposite(f)), 0.0))),
        };
    }
    if let Some(axis) = want.prop("axis") {
        let (a, b) = match axis {
            "x" => ("east", "west"),
            "z" => ("north", "south"),
            _ => ("down", "up"),
        };
        return [a, b].iter().find_map(|d| click_from(add(pos, vector(d)), None, None));
    }
    let Some(f) = want.prop("facing") else {
        if let Some(r) = want.prop("rotation").and_then(|r| r.parse::<f32>().ok()) {
            let n = want.name.as_str();
            let yaw = if n.ends_with("_head") || n.ends_with("_skull") { r * 22.5 } else { r * 22.5 + 180.0 };
            return by_half(Some((yaw, 0.0)));
        }
        return by_half(None);
    };
    match rule(&want.name) {
        Rule::Look => by_half(Some(look_along(f, yaw))),
        Rule::Back => by_half(Some(look_along(opposite(f), yaw))),
        Rule::Look6 => by_half(Some(look_along(f, yaw))),
        Rule::Back6 => by_half(Some(look_along(opposite(f), yaw))),
        Rule::Clockwise => by_half(Some(look_along(ccw(f), yaw))),
        Rule::Face | Rule::Wall => click_from(add(pos, neg(vector(f))), None, None),
        Rule::AgainstFace => click_from(add(pos, vector(f)), None, None),
    }
}

/// Keeps the placements of a world: one line each, `file|x|y|z|rotation|mirror`.
pub fn save_text(m: &Mod) -> String {
    m.placements
        .iter()
        .map(|p| {
            let o = p.t.origin;
            let mirror = match p.t.mirror {
                Mirror::None => "none",
                Mirror::LeftRight => "z",
                Mirror::FrontBack => "x",
            };
            format!("{}|{}|{}|{}|{}|{}", p.file, o[0], o[1], o[2], p.t.rotation, mirror)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
/// The placements `save_text` kept, read again from `dir`'s files.
pub fn load_text(text: &str, dir: &Path) -> Vec<Placement> {
    let mut out = Vec::new();
    for line in text.lines() {
        let f: Vec<&str> = line.split('|').collect();
        if f.len() != 6 {
            continue;
        }
        let (Ok(x), Ok(y), Ok(z), Ok(rot)) = (f[1].parse(), f[2].parse(), f[3].parse(), f[4].parse::<u8>()) else {
            continue;
        };
        let mirror = match f[5] {
            "z" => Mirror::LeftRight,
            "x" => Mirror::FrontBack,
            _ => Mirror::None,
        };
        let Ok(bytes) = std::fs::read(dir.join(format!("{}.litematic", f[0]))) else { continue };
        let Ok(s) = schematic::parse(&bytes) else { continue };
        out.push(Placement::new(f[0].to_owned(), s, Transform { origin: [x, y, z], rotation: rot & 3, mirror }));
    }
    out
}

/// The setting key a world's placements are kept under.
pub fn world_key(world: &str) -> String {
    let s: String = world.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' }).collect();
    format!("placements_{s}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn st(name: &str, props: &[(&str, &str)]) -> State {
        let mut props: Vec<(String, String)> = props.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect();
        props.sort();
        State { name: name.into(), props }
    }

    #[test]
    fn the_printer_clicks_the_right_face_for_halves_and_facings() {
        let ground = |p: [i32; 3]| p[1] < 64;
        // A bottom slab on the ground: the top face of the block below.
        let c = plan(&st("minecraft:stone_slab", &[("type", "bottom")]), [0, 64, 0], 0.0, &ground).unwrap();
        assert_eq!((c.pos, c.face), ([0, 63, 0], [0, 1, 0]));
        // A top slab beside a wall: the wall's side, high up.
        let wall = |p: [i32; 3]| p == [1, 64, 0];
        let c = plan(&st("minecraft:stone_slab", &[("type", "top")]), [0, 64, 0], 0.0, &wall).unwrap();
        assert_eq!((c.pos, c.face), ([1, 64, 0], [-1, 0, 0]));
        assert!(c.cursor[1] > 0.5);
        // Stairs facing east: the player looks east.
        let c = plan(&st("minecraft:oak_stairs", &[("facing", "east"), ("half", "bottom")]), [0, 64, 0], 0.0, &ground).unwrap();
        assert_eq!(c.look, Some((-90.0, 0.0)));
        // A furnace facing north: the player looks south, at it.
        let c = plan(&st("minecraft:furnace", &[("facing", "north")]), [0, 64, 0], 0.0, &ground).unwrap();
        assert_eq!(c.look, Some((0.0, 0.0)));
        // A wall torch facing west hangs on the block east of it.
        let east = |p: [i32; 3]| p == [1, 64, 0];
        let c = plan(&st("minecraft:wall_torch", &[("facing", "west")]), [0, 64, 0], 0.0, &east).unwrap();
        assert_eq!((c.pos, c.face), ([1, 64, 0], [-1, 0, 0]));
        // A log along x needs a block east or west.
        assert!(plan(&st("minecraft:oak_log", &[("axis", "x")]), [0, 64, 0], 0.0, &ground).is_none());
        // A door's top half isn't placed on its own.
        assert!(plan(&st("minecraft:oak_door", &[("half", "upper"), ("facing", "north")]), [0, 65, 0], 0.0, &ground).is_none());
    }

    #[test]
    fn layers_filter_heights() {
        assert!(Layers::Only(5).shows(5) && !Layers::Only(5).shows(6));
        assert!(Layers::Below(5).shows(-3) && !Layers::Below(5).shows(6));
        assert!(Layers::Above(5).shows(6) && !Layers::Above(5).shows(4));
    }

    #[test]
    fn items_for_blocks_that_have_other_items() {
        assert_eq!(item_of("minecraft:redstone_wire"), "minecraft:redstone");
        assert_eq!(item_of("minecraft:oak_wall_sign"), "minecraft:oak_sign");
        assert_eq!(item_of("minecraft:stone"), "minecraft:stone");
        assert_eq!(facing_of(0.0), "south");
        assert_eq!(facing_of(-90.0), "east");
        assert_eq!(facing_of(181.0), "north");
    }
}
