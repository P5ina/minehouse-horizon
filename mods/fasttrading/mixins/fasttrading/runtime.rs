// Fast Trading for Horizon: a port of Fast Trading 0.2 for Fabric
// (https://github.com/bendy1234/FastTrading, MIT): a button beside the trade's
// result that repeats the selected trade until it can't go on, refilling the
// payment slots the way the offer button does (`AutofillBehavior.DEFAULT`), one
// action a tick (`ticksBetweenActions` 1), stopping when the price goes up
// (`PriceChangeStopBehavior.ON_INCREASE`). A result that can be damaged (a tool,
// armour) needs Shift, as the original's `DAMAGEABLE` block and its override key.
// This file is both the mod's Cargo library (its tests) and the module the
// bundle adds to the game.

use std::cell::RefCell;

pub const ID: &str = "fasttrading";
pub const NAME: &str = "Fast Trading";
pub const VERSION: &str = "0.2.5";
pub const DESCRIPTION: &str =
    "Adds a button to the villager trading screen that repeats the selected trade until it runs out.";

/// The button in the merchant panel (`leftPos + 247`, `topPos + 36`), 20 by 18
/// as its texture is drawn.
pub const BUTTON: [f32; 4] = [247.0, 36.0, 20.0, 18.0];
/// Ticks a refill may take to come back from the server before the run gives up.
pub const PATIENCE: u32 = 20;
pub const MERCHANT_MENU: &str = "minecraft:merchant";

/// Whether `mouse` is on the button of the panel at `panel` (`[x, y, w, h]`).
pub fn on_button(panel: [f32; 4], mouse: [f32; 2]) -> bool {
    let (x, y) = (panel[0] + BUTTON[0], panel[1] + BUTTON[1]);
    mouse[0] >= x && mouse[0] < x + BUTTON[2] && mouse[1] >= y && mouse[1] < y + BUTTON[3]
}

/// What the screen shows for the selected trade (`MerchantScreenHooks.State`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    CanPerform,
    NoSelection,
    OutOfStock,
    NoRoom,
    NotEnough,
    Blocked,
}

/// One tick's look at the open merchant menu.
#[derive(Clone, Copy, Debug, Default)]
pub struct Seen {
    pub selected: usize,
    pub offers: usize,
    /// The selected offer's current price (cost A's count).
    pub cost: u32,
    pub out_of_stock: bool,
    /// The inventory and the payment slots hold the price.
    pub can_pay: bool,
    /// The inventory takes one more result.
    pub room: bool,
    /// The result slot holds the trade's result.
    pub result_ready: bool,
    /// The result can be damaged, and Shift wasn't held to trade it anyway.
    pub blocked: bool,
}
impl Seen {
    pub fn why(&self) -> Why {
        if self.selected >= self.offers {
            Why::NoSelection
        } else if self.out_of_stock {
            Why::OutOfStock
        } else if !self.room {
            Why::NoRoom
        } else if !self.can_pay {
            Why::NotEnough
        } else if self.blocked {
            Why::Blocked
        } else {
            Why::CanPerform
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Autofill,
    Trade,
}
/// What the run does this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Press the offer button again: the server fills the payment slots.
    Select(usize),
    /// Shift-click the result.
    TakeResult,
    Wait,
    /// Done: the payment slots go back to the inventory.
    Stop,
}

/// A running speed trade (`SpeedTradeButton` past `INACTIVE`).
#[derive(Clone, Copy, Debug)]
pub struct Run {
    pub index: usize,
    pub cost: u32,
    pub phase: Phase,
    pub waited: u32,
}
impl Run {
    pub fn start(seen: &Seen) -> Self {
        Run { index: seen.selected, cost: seen.cost, phase: Phase::Autofill, waited: 0 }
    }
    /// `SpeedTradeButton.tick`: one action, or the end of the run.
    pub fn step(&mut self, seen: &Seen) -> Action {
        // shouldStop: another trade picked, the price went up, out of stock.
        if seen.selected != self.index || seen.cost > self.cost || seen.out_of_stock {
            return Action::Stop;
        }
        match self.phase {
            Phase::Autofill => {
                if !seen.can_pay || !seen.room {
                    return Action::Stop;
                }
                self.phase = Phase::Trade;
                self.waited = 0;
                Action::Select(self.index)
            }
            Phase::Trade => {
                if seen.result_ready {
                    self.phase = Phase::Autofill;
                    Action::TakeResult
                } else if self.waited >= PATIENCE {
                    Action::Stop
                } else {
                    self.waited += 1;
                    Action::Wait
                }
            }
        }
    }
}

#[derive(Default)]
pub struct State {
    /// The merchant menu as the last tick saw it, for drawing the button.
    pub seen: Option<Seen>,
    pub run: Option<Run>,
    /// A click on the button, for the next tick (screen hooks only read).
    pub pressed: Option<bool>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}
pub fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

/// English, or Russian when the game is in Russian.
pub fn tr(russian: bool, en: &'static str, ru: &'static str) -> &'static str {
    if russian { ru } else { en }
}

/// The tooltip lines (`SpeedTradeButton.applyTooltip`).
pub fn tooltip(russian: bool, running: bool, why: Why) -> Vec<&'static str> {
    if running {
        return vec![tr(russian, "Trading in progress, please wait...", "Торговля в процессе, подождите...")];
    }
    let mut lines = Vec::new();
    match why {
        Why::CanPerform => lines.push(tr(russian, "Click to speed trade", "Нажмите для ускоренной торговли")),
        why => {
            lines.push(tr(russian, "Can't speed trade!", "Невозможно ускорить торговлю!"));
            lines.push(match why {
                Why::NoSelection => tr(russian, "Select a trade!", "Выберите сделку!"),
                Why::OutOfStock => tr(russian, "Out of stock!", "Нет в наличии!"),
                Why::NoRoom => tr(russian, "No room for the result!", "Нет места для результата торговли!"),
                Why::NotEnough => tr(russian, "Not enough items to trade!", "Недостаточно предметов для торговли!"),
                _ => tr(russian, "The result can be damaged: hold Shift to trade it anyway", "Результат может быть повреждён: зажмите Shift, чтобы всё равно торговать"),
            });
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen() -> Seen {
        Seen { selected: 2, offers: 5, cost: 10, can_pay: true, room: true, ..Seen::default() }
    }

    #[test]
    fn a_run_refills_then_takes_until_the_payment_runs_out() {
        let mut s = seen();
        let mut run = Run::start(&s);
        assert_eq!(run.step(&s), Action::Select(2));
        assert_eq!(run.step(&s), Action::Wait);
        s.result_ready = true;
        assert_eq!(run.step(&s), Action::TakeResult);
        assert_eq!(run.step(&s), Action::Select(2));
        s.can_pay = false;
        s.result_ready = false;
        run.phase = Phase::Autofill;
        assert_eq!(run.step(&s), Action::Stop);
    }

    #[test]
    fn a_dearer_price_another_trade_or_no_answer_stops_it() {
        let s = seen();
        let mut run = Run::start(&s);
        assert_eq!(run.step(&Seen { cost: 11, ..s }), Action::Stop);
        assert_eq!(run.step(&Seen { selected: 1, ..s }), Action::Stop);
        assert_eq!(run.step(&Seen { cost: 8, ..s }), Action::Select(2));
        for _ in 0..PATIENCE {
            assert_eq!(run.step(&s), Action::Wait);
        }
        assert_eq!(run.step(&s), Action::Stop);
    }

    #[test]
    fn the_button_sits_beside_the_result() {
        let panel = [100.0, 50.0, 276.0, 166.0];
        assert!(on_button(panel, [347.0, 86.0]));
        assert!(on_button(panel, [366.9, 103.9]));
        assert!(!on_button(panel, [367.0, 86.0]));
        assert!(!on_button(panel, [346.0, 86.0]));
    }

    #[test]
    fn why_names_the_first_thing_in_the_way() {
        assert_eq!(seen().why(), Why::CanPerform);
        assert_eq!(Seen { selected: 5, ..seen() }.why(), Why::NoSelection);
        assert_eq!(Seen { out_of_stock: true, can_pay: false, ..seen() }.why(), Why::OutOfStock);
        assert_eq!(Seen { can_pay: false, blocked: true, ..seen() }.why(), Why::NotEnough);
        assert_eq!(Seen { blocked: true, ..seen() }.why(), Why::Blocked);
    }
}
