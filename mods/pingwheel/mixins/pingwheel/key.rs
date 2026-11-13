// Ping Wheel's keys: Ping Location queues a ping for the next tick
// (`InputUtils.consumePingHotkey`: bound to Pick Block, only when the crosshair
// is on nothing pickable), Open Settings shows its settings, and the player
// list key shows who pinged (Player Info: Hold).
fn pingwheel_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::pingwheel as m;
    if !game.native_on(m::ID) {
        return;
    }
    if key == m::PLAYER_LIST {
        m::with(|s| s.player_list_held = pressed);
    }
    if !pressed || !game.native_playing() {
        return;
    }
    if key == game.native_mod_key(m::ID, m::KEY_PING, m::DEFAULT_PING) {
        m::with(|s| s.queued = true);
    } else if key == game.native_mod_key(m::ID, m::KEY_SETTINGS, m::DEFAULT_SETTINGS) {
        m::with(|s| s.open_settings = true);
    }
}
