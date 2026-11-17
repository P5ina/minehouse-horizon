// Autoclicker for Horizon: a port of Simple AutoClicker for Fabric
// (https://github.com/johnnyscriptt/simpleautoclicker, MIT): keys toggle
// left- and right-click autoclicking, each with its own interval, or holding
// the button down; clicks go through the game's own attack and use, only while
// playing with no screen open, and both stop when the player leaves the world.
// This file is both the mod's Cargo library (its tests) and the module the
// bundle adds to the game.

use std::cell::RefCell;

pub const ID: &str = "autoclicker";
pub const NAME: &str = "Simple AutoClicker";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str =
    "Toggleable left/right autoclicking or holding, with an interval for each button.";

pub const KEY_LEFT: &str = "toggle_left";
pub const KEY_RIGHT: &str = "toggle_right";
pub const KEY_HOLD_LEFT: &str = "hold_left";
pub const KEY_HOLD_RIGHT: &str = "hold_right";
pub const DEFAULT_LEFT: &str = "key.keyboard.comma";
pub const DEFAULT_RIGHT: &str = "key.keyboard.period";
pub const UNBOUND: &str = "key.keyboard.unknown";

pub const SETTING_LEFT: &str = "left_interval_ms";
pub const SETTING_RIGHT: &str = "right_interval_ms";
pub const MS_PER_TICK: u32 = 50;
pub const DEFAULT_INTERVAL_MS: u32 = 200;
/// The intervals a settings button steps through (the original's slider runs
/// 50 ms to 10 s; the game ticks every 50 ms anyway).
pub const INTERVALS_MS: [u32; 15] =
    [50, 100, 150, 200, 250, 300, 400, 500, 750, 1000, 1500, 2000, 3000, 5000, 10000];

/// `AutoclickerConfig.clampMs`: 0 or a missing value is the default.
pub fn clamp_ms(ms: u32) -> u32 {
    if ms == 0 { DEFAULT_INTERVAL_MS } else { ms.clamp(50, 10000) }
}
/// `AutoclickerConfig.leftIntervalTicks`: whole ticks, at least one.
pub fn interval_ticks(ms: u32) -> u32 {
    (clamp_ms(ms) / MS_PER_TICK).max(1)
}
/// The next (or, `back`, previous) step after `ms`, wrapping around.
pub fn step(ms: u32, back: bool) -> u32 {
    let ms = clamp_ms(ms);
    if back {
        INTERVALS_MS.iter().rev().copied().find(|&v| v < ms).unwrap_or(INTERVALS_MS[INTERVALS_MS.len() - 1])
    } else {
        INTERVALS_MS.iter().copied().find(|&v| v > ms).unwrap_or(INTERVALS_MS[0])
    }
}

/// `ClickerTimer`: on enabling, the first tick fires; then every interval.
#[derive(Clone, Copy, Debug, Default)]
pub struct Clicker {
    pub enabled: bool,
    cooldown: u32,
}
impl Clicker {
    pub fn toggle(&mut self) -> bool {
        self.enabled = !self.enabled;
        self.cooldown = 0;
        self.enabled
    }
    pub fn disable(&mut self) {
        *self = Clicker::default();
    }
    pub fn tick(&mut self, interval_ticks: u32) -> bool {
        if !self.enabled {
            return false;
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return false;
        }
        self.cooldown = interval_ticks.max(1) - 1;
        true
    }
}

/// `ButtonModes`: clicking or holding one button; turning one on turns the
/// other off.
#[derive(Clone, Copy, Debug, Default)]
pub struct Modes {
    pub clicker: Clicker,
    pub holding: bool,
}
impl Modes {
    pub fn toggle_click(&mut self) -> bool {
        let on = self.clicker.toggle();
        if on {
            self.holding = false;
        }
        on
    }
    pub fn toggle_hold(&mut self) -> bool {
        self.holding = !self.holding;
        if self.holding {
            self.clicker.disable();
        }
        self.holding
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    Left,
    Right,
    HoldLeft,
    HoldRight,
}

#[derive(Default)]
pub struct State {
    pub left: Modes,
    pub right: Modes,
    /// Toggles pressed since the last tick (keys arrive between ticks).
    pub queued: Vec<Toggle>,
    /// The hold was on last tick: its button is ours to let go.
    pub held: [bool; 2],
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}
pub fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// The action bar line for a toggle (`simpleautoclicker.toggle.*`).
pub fn message(russian: bool, toggle: Toggle, on: bool) -> String {
    let (en, ru) = match toggle {
        Toggle::Left => ("Left autoclick", "Автоклик ЛКМ"),
        Toggle::Right => ("Right autoclick", "Автоклик ПКМ"),
        Toggle::HoldLeft => ("Hold left click", "Удержание ЛКМ"),
        Toggle::HoldRight => ("Hold right click", "Удержание ПКМ"),
    };
    let state = match (russian, on) {
        (false, true) => "ON",
        (false, false) => "OFF",
        (true, true) => "ВКЛ",
        (true, false) => "ВЫКЛ",
    };
    format!("{}: {state}", if russian { ru } else { en })
}

/// The settings page's button labels.
pub fn buttons(russian: bool, left_ms: u32, right_ms: u32) -> Vec<String> {
    let (l, r) = if russian { ("Левая кнопка", "Правая кнопка") } else { ("Left", "Right") };
    let ms = if russian { "мс" } else { "ms" };
    vec![format!("{l}: {} {ms}", clamp_ms(left_ms)), format!("{r}: {} {ms}", clamp_ms(right_ms))]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clicker_fires_at_once_then_every_interval() {
        let mut c = Clicker::default();
        assert!(!c.tick(4));
        c.toggle();
        let fired: Vec<bool> = (0..9).map(|_| c.tick(4)).collect();
        assert_eq!(fired, [true, false, false, false, true, false, false, false, true]);
        c.toggle();
        assert!(!c.tick(4));
    }

    #[test]
    fn clicking_and_holding_exclude_each_other() {
        let mut m = Modes::default();
        assert!(m.toggle_click());
        assert!(m.toggle_hold());
        assert!(!m.clicker.enabled);
        assert!(m.toggle_click());
        assert!(!m.holding);
    }

    #[test]
    fn intervals_round_to_ticks_and_step_through_the_list() {
        assert_eq!(interval_ticks(0), 4);
        assert_eq!(interval_ticks(20), 1);
        assert_eq!(interval_ticks(175), 3);
        assert_eq!(interval_ticks(99_999), 200);
        assert_eq!(step(200, false), 250);
        assert_eq!(step(200, true), 150);
        assert_eq!(step(10000, false), 50);
        assert_eq!(step(50, true), 10000);
        assert_eq!(step(175, false), 200);
        assert_eq!(step(175, true), 150);
    }
}
