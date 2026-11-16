// AppleSkin for Horizon: a port of AppleSkin 3.0 for Fabric 26.3
// (https://github.com/squeek502/AppleSkin, public domain): the saturation
// over the food bar, the exhaustion under it, what held food would restore
// (hunger, saturation and health, flashing), and food values in tooltips.
// It reads the server's AppleSkin channels (`appleskin:saturation`,
// `appleskin:exhaustion`, `appleskin:natural_regeneration`).
// This file is both the mod's Cargo library (its tests) and the module the
// bundle adds to the game.

pub const ID: &str = "appleskin";
pub const NAME: &str = "AppleSkin";
pub const VERSION: &str = "3.0.10";
pub const DESCRIPTION: &str = "Adds various food-related HUD improvements.";

pub const SATURATION: &str = "appleskin:saturation";
pub const EXHAUSTION: &str = "appleskin:exhaustion";
pub const NATURAL_REGENERATION: &str = "appleskin:natural_regeneration";
pub const CHANNELS: [&str; 3] = [SATURATION, EXHAUSTION, NATURAL_REGENERATION];
pub const REGISTER: &str = "minecraft:register";

pub fn encode_register(channels: &[&str]) -> Vec<u8> {
    channels.join("\0").into_bytes()
}

pub fn decode_f32(data: &[u8]) -> Option<f32> {
    let b: [u8; 4] = data.get(..4)?.try_into().ok()?;
    Some(f32::from_be_bytes(b))
}

pub fn decode_bool(data: &[u8]) -> Option<bool> {
    data.first().map(|b| *b != 0)
}

// --- FoodHelper ----------------------------------------------------------------

pub const MAX_EXHAUSTION: f32 = 4.0;
pub const REGEN_EXHAUSTION_INCREMENT: f32 = 6.0;

/// Effects whose `MobEffectCategory` is HARMFUL: a food giving one is "rotten"
/// (`FoodHelper.isRotten`) and is drawn with the hunger icons.
pub const HARMFUL: [&str; 17] = [
    "minecraft:slowness",
    "minecraft:mining_fatigue",
    "minecraft:instant_damage",
    "minecraft:nausea",
    "minecraft:blindness",
    "minecraft:hunger",
    "minecraft:weakness",
    "minecraft:poison",
    "minecraft:wither",
    "minecraft:levitation",
    "minecraft:unluck",
    "minecraft:darkness",
    "minecraft:wind_charged",
    "minecraft:weaving",
    "minecraft:oozing",
    "minecraft:infested",
    "minecraft:bad_omen",
];

pub fn harmful(effect: &str) -> bool {
    let id = if effect.contains(':') { effect.to_string() } else { format!("minecraft:{effect}") };
    HARMFUL.contains(&id.as_str())
}

/// `FoodHelper.getEstimatedHealthIncrement(food, saturation, exhaustion)`.
pub fn estimated_regen(mut food: i32, mut saturation: f32, mut exhaustion: f32) -> f32 {
    let mut health = 0.0f32;
    if !exhaustion.is_finite() || !saturation.is_finite() {
        return 0.0;
    }
    while food >= 18 {
        while exhaustion > MAX_EXHAUSTION {
            exhaustion -= MAX_EXHAUSTION;
            if saturation > 0.0 {
                saturation = (saturation - 1.0).max(0.0);
            } else {
                food -= 1;
            }
        }
        if food >= 20 && saturation > f32::MIN_POSITIVE {
            let limited = saturation.min(REGEN_EXHAUSTION_INCREMENT);
            let until = next_up(MAX_EXHAUSTION) - exhaustion;
            let n = ((until / limited).ceil() as i32).max(1);
            health += limited / REGEN_EXHAUSTION_INCREMENT * n as f32;
            exhaustion += limited * n as f32;
        } else if food >= 18 {
            health += 1.0;
            exhaustion += REGEN_EXHAUSTION_INCREMENT;
        }
    }
    health
}

fn next_up(v: f32) -> f32 {
    f32::from_bits(v.to_bits() + 1)
}

/// What eating restores: the food's nutrition and saturation, and whether it
/// is rotten, and the regeneration effect it gives (amplifier, duration).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Food {
    pub nutrition: i32,
    pub saturation: f32,
    pub rotten: bool,
    pub regeneration: Option<(i32, i32)>,
}

/// The player's state the overlays read.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Player {
    pub health: f32,
    pub max_health: f32,
    pub food: i32,
    pub saturation: f32,
    pub exhaustion: f32,
    pub natural_regeneration: bool,
}

/// `FoodHelper.getEstimatedHealthIncrement(player, food)`.
pub fn estimated_health(p: &Player, f: &Food) -> f32 {
    if p.health >= p.max_health {
        return 0.0;
    }
    let food = (p.food + f.nutrition).min(20);
    let mut inc = 0.0;
    if food >= 18 && p.natural_regeneration {
        let saturation = (p.saturation + f.saturation).min(food as f32);
        inc = estimated_regen(food, saturation, p.exhaustion);
    }
    if let Some((amplifier, duration)) = f.regeneration {
        inc += (duration as f32 / (50i32 >> amplifier.clamp(0, 31)).max(1) as f32).floor();
    }
    inc
}

// --- HUDOverlayHandler -------------------------------------------------------

/// `onClientTick`'s flash: up and down by an eighth each tick between -0.5
/// and 1.5, clamped to 0..1 (times `maxHudOverlayFlashAlpha`, 0.65).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flash {
    pub unclamped: f32,
    pub dir: f32,
    pub alpha: f32,
}

pub const MAX_FLASH_ALPHA: f32 = 0.65;

impl Default for Flash {
    fn default() -> Self {
        Self { unclamped: 0.0, dir: 1.0, alpha: 0.0 }
    }
}

impl Flash {
    pub fn tick(&mut self) {
        self.unclamped += self.dir * 0.125;
        if self.unclamped >= 1.5 {
            self.dir = -1.0;
        } else if self.unclamped <= -0.5 {
            self.dir = 1.0;
        }
        self.alpha = self.unclamped.clamp(0.0, 1.0) * MAX_FLASH_ALPHA;
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// White with `alpha` (0..1), as ARGB.
pub fn white(alpha: f32) -> u32 {
    ((alpha.clamp(0.0, 1.0) * 255.0) as u32) << 24 | 0xFF_FFFF
}

/// One thing to draw over the HUD, in GUI pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum Overlay {
    /// One of the icon runs of [`SATURATION`](SATURATION_ICONS) or the exhaustion bar, at `at`,
    /// only its columns from `from_x`, times `alpha`.
    Icon { runs: Runs, at: [i32; 2], from_x: u8, alpha: f32 },
    Sprite { name: &'static str, at: [i32; 2], color: u32 },
}

/// `drawSaturationOverlay`: the outlines for `level + gained` saturation, from
/// the current level's tail when `gained` is not 0. `food` is where each of the
/// ten food icons is drawn (right to left).
pub fn saturation_overlay(gained: f32, level: f32, food: &[[i32; 2]], alpha: f32, out: &mut Vec<Overlay>) {
    if level + gained < 0.0 {
        return;
    }
    let modified = (level + gained).clamp(0.0, 20.0);
    let start = if gained != 0.0 { (level / 2.0).max(0.0) as usize } else { 0 };
    let end = (modified / 2.0).ceil() as usize;
    for i in start..end {
        let Some(&at) = food.get(i) else { continue };
        let effective = modified / 2.0 - i as f32;
        let icon = if effective >= 1.0 {
            3
        } else if effective > 0.5 {
            2
        } else if effective > 0.25 {
            1
        } else {
            0
        };
        out.push(Overlay::Icon { runs: SATURATION_ICONS[icon], at, from_x: 0, alpha });
    }
}

pub fn food_sprite(rotten: bool, full: Option<bool>) -> &'static str {
    match (rotten, full) {
        (false, None) => "hud/food_empty",
        (false, Some(false)) => "hud/food_half",
        (false, Some(true)) => "hud/food_full",
        (true, None) => "hud/food_empty_hunger",
        (true, Some(false)) => "hud/food_half_hunger",
        (true, Some(true)) => "hud/food_full_hunger",
    }
}

/// `drawHungerOverlay`: the food points `restored` would add, flashing.
pub fn hunger_overlay(restored: i32, level: i32, food: &[[i32; 2]], alpha: f32, rotten: bool, out: &mut Vec<Overlay>) {
    if restored <= 0 {
        return;
    }
    let modified = (level + restored).clamp(0, 20);
    let start = (level / 2).max(0) as usize;
    let end = (modified as f32 / 2.0).ceil() as usize;
    for i in start..end {
        let Some(&at) = food.get(i) else { continue };
        out.push(Overlay::Sprite { name: food_sprite(rotten, None), at, color: white(alpha * 0.25) });
        let half = i as i32 * 2 + 1 == modified;
        out.push(Overlay::Sprite { name: food_sprite(rotten, Some(!half)), at, color: white(alpha) });
    }
}

/// `drawHealthOverlay`: the hearts eating would restore, flashing. `hearts[i]`
/// is where heart `i` (from the left) is drawn.
pub fn health_overlay(health: f32, modified: f32, hearts: &[[i32; 2]], alpha: f32, hardcore: bool, out: &mut Vec<Overlay>) {
    if modified <= health {
        return;
    }
    let fixed = modified.ceil() as i32;
    let start = (health.ceil() / 2.0).max(0.0) as usize;
    let end = (modified / 2.0).ceil().max(0.0) as usize;
    let (container, full, half) = if hardcore {
        ("hud/heart/container_hardcore", "hud/heart/hardcore_full", "hud/heart/hardcore_half")
    } else {
        ("hud/heart/container", "hud/heart/full", "hud/heart/half")
    };
    for i in start..end {
        let Some(&at) = hearts.get(i) else { continue };
        out.push(Overlay::Sprite { name: container, at, color: white(alpha * 0.25) });
        let is_half = i as i32 * 2 + 1 == fixed;
        out.push(Overlay::Sprite { name: if is_half { half } else { full }, at, color: white(alpha) });
    }
}

/// `drawExhaustionOverlay`: the bar's right part, `exhaustion / 4` of 81 pixels,
/// ending at `right`, at 0.75 alpha.
pub fn exhaustion_overlay(exhaustion: f32, right: i32, top: i32) -> Option<Overlay> {
    let ratio = (exhaustion / MAX_EXHAUSTION).clamp(0.0, 1.0);
    let width = (ratio * 81.0) as i32;
    (width > 0).then(|| Overlay::Icon { runs: EXHAUSTION_BAR, at: [right - 81, top], from_x: (81 - width) as u8, alpha: 0.75 })
}

/// `shouldShowEstimatedHealth`, from what the HUD knows.
pub fn show_estimated_health(p: &Player, peaceful: bool, poisoned_or_withered: bool, regenerating: bool) -> bool {
    !peaceful && p.food < 18 && !poisoned_or_withered && !regenerating
}

/// The pixel runs of `runs` from column `from_x`, at `at`, as fills `[x0, y0, x1, y1, argb]`.
pub fn fills(runs: Runs, at: [i32; 2], from_x: u8, alpha: f32) -> Vec<[i32; 5]> {
    let mut out = Vec::new();
    for &(x, y, len, argb) in runs {
        let (x0, x1) = (x.max(from_x), x.saturating_add(len));
        if x0 >= x1 {
            continue;
        }
        let a = (((argb >> 24) as f32) * alpha.clamp(0.0, 1.0)) as u32;
        if a == 0 {
            continue;
        }
        out.push([at[0] + i32::from(x0), at[1] + i32::from(y), at[0] + i32::from(x1), at[1] + i32::from(y) + 1, (a << 24 | argb & 0xFF_FFFF) as i32]);
    }
    out
}

// --- TooltipOverlayHandler ---------------------------------------------------

/// `FoodOverlay`: how many icons each row has, and the "xN" text when it
/// needs more than ten (or no saturation icon).
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipFood {
    pub food: Food,
    pub hunger_bars: i32,
    pub hunger_text: Option<String>,
    pub saturation_bars: i32,
    pub saturation_text: Option<String>,
}

impl TooltipFood {
    pub fn new(food: Food) -> Self {
        let mut hunger_bars = (food.nutrition.abs() as f32 / 2.0).ceil() as i32;
        let mut hunger_text = None;
        if hunger_bars > 10 {
            hunger_text = Some(format!("x{}", food.nutrition.signum().max(-1) * hunger_bars));
            hunger_bars = 1;
        }
        let mut saturation_bars = (food.saturation.abs() / 2.0).ceil() as i32;
        let mut saturation_text = None;
        if saturation_bars > 10 || saturation_bars == 0 {
            let sign = if food.saturation < 0.0 { -1 } else { 1 };
            saturation_text = Some(format!("x{}", sign * saturation_bars));
            saturation_bars = 1;
        }
        Self { food, hunger_bars, hunger_text, saturation_bars, saturation_text }
    }
    pub fn shown(&self) -> bool {
        self.hunger_bars > 0
    }
    /// `getWidth`, given the text width function.
    pub fn width(&self, text_width: impl Fn(&str) -> f32) -> f32 {
        let hunger = self.hunger_bars as f32 * 9.0 + self.hunger_text.as_deref().map_or(0.0, &text_width);
        let saturation = self.saturation_bars as f32 * 7.0 + self.saturation_text.as_deref().map_or(0.0, &text_width);
        hunger.max(saturation)
    }
    /// `getHeight`: hunger + spacing + saturation + 3.
    pub const HEIGHT: f32 = 20.0;
    /// The small saturation icon of bar `i` (0-based, counting by 2 points):
    /// its runs and whether it is faded.
    pub fn saturation_icon(&self, i: i32) -> (Runs, bool) {
        let abs = self.food.saturation.abs();
        let effective = (abs - (i * 2) as f32) / 2.0;
        let idx = if effective >= 1.0 {
            3
        } else if effective > 0.5 {
            2
        } else if effective > 0.25 {
            1
        } else if effective > 0.0 {
            0
        } else {
            4
        };
        let runs = if self.food.saturation >= 0.0 { SATURATION_SMALL[idx] } else { SATURATION_SMALL_NEGATIVE[idx] };
        (runs, abs <= (i * 2) as f32)
    }
}

// --- State ---------------------------------------------------------------------

pub struct State {
    pub registered: bool,
    /// From the server's AppleSkin, when it sent them.
    pub saturation: Option<f32>,
    pub exhaustion: f32,
    pub natural_regeneration: bool,
    pub flash: Flash,
}

impl Default for State {
    fn default() -> Self {
        Self { registered: false, saturation: None, exhaustion: 0.0, natural_regeneration: true, flash: Flash::default() }
    }
}

impl State {
    pub fn left_server(&mut self) {
        *self = Self::default();
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
    fn payloads() {
        assert_eq!(decode_f32(&2.5f32.to_be_bytes()), Some(2.5));
        assert_eq!(decode_f32(&[0, 0]), None);
        assert_eq!(decode_bool(&[0]), Some(false));
    }

    #[test]
    fn regen_estimate() {
        // Full food and saturation: fast regen; 18 food without saturation: slow.
        assert!(estimated_regen(20, 5.0, 0.0) > 0.0);
        assert!(estimated_regen(17, 5.0, 0.0) == 0.0);
        assert!(estimated_regen(18, 0.0, 0.0) >= 1.0);
        let p = Player { health: 10.0, max_health: 20.0, food: 16, saturation: 0.0, exhaustion: 0.0, natural_regeneration: true };
        let steak = Food { nutrition: 8, saturation: 12.8, rotten: false, regeneration: None };
        assert!(estimated_health(&p, &steak) > 0.0);
        let full = Player { health: 20.0, ..p };
        assert_eq!(estimated_health(&full, &steak), 0.0);
        let gapple = Food { nutrition: 4, saturation: 9.6, rotten: false, regeneration: Some((1, 100)) };
        let off = Player { natural_regeneration: false, food: 2, ..p };
        assert_eq!(estimated_health(&off, &gapple), 4.0);
    }

    #[test]
    fn flash_cycles() {
        let mut f = Flash::default();
        let mut seen_max = 0.0f32;
        for _ in 0..40 {
            f.tick();
            seen_max = seen_max.max(f.alpha);
            assert!((0.0..=MAX_FLASH_ALPHA).contains(&f.alpha));
        }
        assert_eq!(seen_max, MAX_FLASH_ALPHA);
    }

    fn food_row() -> Vec<[i32; 2]> {
        (0..10).map(|i| [200 - i * 8 - 9, 100]).collect()
    }

    #[test]
    fn overlays() {
        let mut out = Vec::new();
        saturation_overlay(0.0, 5.0, &food_row(), 1.0, &mut out);
        assert_eq!(out.len(), 3);
        assert!(matches!(out[2], Overlay::Icon { at: [175, 100], .. }));
        out.clear();
        hunger_overlay(3, 10, &food_row(), 0.5, false, &mut out);
        // Icons 5 and 6 (13 food: 6 is half).
        assert_eq!(out.len(), 4);
        assert!(matches!(out[3], Overlay::Sprite { name: "hud/food_half", .. }));
        assert!(exhaustion_overlay(0.0, 200, 100).is_none());
        let Some(Overlay::Icon { from_x, .. }) = exhaustion_overlay(2.0, 200, 100) else { panic!() };
        assert_eq!(from_x, 41);
        let f = fills(EXHAUSTION_BAR, [0, 0], 41, 1.0);
        assert!(f.iter().all(|r| r[0] >= 41));
    }

    #[test]
    fn tooltip() {
        let t = TooltipFood::new(Food { nutrition: 8, saturation: 12.8, rotten: false, regeneration: None });
        assert_eq!((t.hunger_bars, t.saturation_bars), (4, 7));
        assert!(t.hunger_text.is_none());
        let none = TooltipFood::new(Food { nutrition: 1, saturation: 0.0, ..t.food });
        assert_eq!(none.saturation_text.as_deref(), Some("x0"));
        assert_eq!(none.saturation_icon(0).1, true);
        assert_eq!(t.width(|s| s.len() as f32 * 6.0), 49.0);
    }

    #[test]
    fn harmful_effects() {
        assert!(harmful("minecraft:hunger"));
        assert!(harmful("poison"));
        assert!(!harmful("minecraft:regeneration"));
    }
}

