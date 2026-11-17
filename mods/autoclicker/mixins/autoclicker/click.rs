// A click on an interval steps it (Shift: back); it applies at once.
fn autoclicker_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::autoclicker as m;
    if mod_id != m::ID {
        return;
    }
    let key = match button {
        0 => m::SETTING_LEFT,
        1 => m::SETTING_RIGHT,
        _ => return,
    };
    let now = game.native_setting(m::ID, key).and_then(|v| v.parse().ok()).unwrap_or(0);
    let back = game.native_has_shift_down();
    game.native_set_setting(m::ID, key, &m::step(now, back).to_string());
}
