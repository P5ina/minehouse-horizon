// A click on a settings row steps its value (Shift: back); the channel row
// reminds of `/pingwheel channel <name>`.
fn pingwheel_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::pingwheel as m;
    if mod_id != m::ID {
        return;
    }
    let back = game.native_has_shift_down();
    if button == 9 {
        game.native_chat("You only send and receive Pings to and from players on the same channel: /pingwheel channel <channel_name>");
        return;
    }
    let store = m::with(|s| {
        s.config.click(button, back);
        s.save();
        std::mem::take(&mut s.store)
    });
    for (k, v) in store {
        game.native_set_setting(m::ID, &k, &v);
    }
}
