// `AutoclickController.tick`: toggles, then each button's click or hold.
// Clicks go through the game's own mouse handling (attack, use), and only
// while playing with nothing open; leaving the world switches both off.
fn autoclicker_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::autoclicker as m;
    use winit::event::MouseButton;
    let on = game.native_on(m::ID);
    if !on || game.player.is_none() {
        let held = m::with(|s| {
            let held = s.held;
            *s = m::State::default();
            held
        });
        for (i, button) in [MouseButton::Left, MouseButton::Right].into_iter().enumerate() {
            if held[i] {
                let _ = game.play_mouse(button, false);
            }
        }
        return;
    }
    let russian = horizon_client::lang::code().starts_with("ru");
    let queued = m::with(|s| std::mem::take(&mut s.queued));
    for toggle in queued {
        let on = m::with(|s| match toggle {
            m::Toggle::Left => s.left.toggle_click(),
            m::Toggle::Right => s.right.toggle_click(),
            m::Toggle::HoldLeft => s.left.toggle_hold(),
            m::Toggle::HoldRight => s.right.toggle_hold(),
        });
        game.titles.set_overlay(horizon_client::text::Text::literal(m::message(russian, toggle, on)), false);
    }
    let ms = |k: &str| game.native_setting(m::ID, k).and_then(|v| v.parse().ok()).unwrap_or(0);
    let ticks = [m::interval_ticks(ms(m::SETTING_LEFT)), m::interval_ticks(ms(m::SETTING_RIGHT))];
    let playing = game.native_playing();
    let buttons = [MouseButton::Left, MouseButton::Right];
    for (i, button) in buttons.into_iter().enumerate() {
        let (holding, was_held) = m::with(|s| {
            let modes = if i == 0 { &s.left } else { &s.right };
            (modes.holding, s.held[i])
        });
        // A hold let go (or taken over by clicking): release the button.
        if was_held && !holding {
            let _ = game.play_mouse(button, false);
        }
        // Opening a screen lets go of the buttons: press again once it closes.
        let down = if i == 0 { game.left } else { game.right };
        if holding && playing && !down {
            let _ = game.play_mouse(button, true);
        }
        m::with(|s| s.held[i] = holding);
        if !playing {
            continue;
        }
        let fire = m::with(|s| {
            let modes = if i == 0 { &mut s.left } else { &mut s.right };
            modes.clicker.tick(ticks[i])
        });
        if fire && !holding {
            let _ = game.play_mouse(button, true);
            let _ = game.play_mouse(button, false);
        }
    }
}
