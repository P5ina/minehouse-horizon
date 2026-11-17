// Keys over the map: the map key closes it again, Space puts the player back
// in the middle. Escape closes it as any screen.
fn worldmap_keys(game: &crate::game::Game<'_>, key: &str, canceled: &mut bool) {
    use crate::game::worldmap as m;
    if game.native_mod_screen() != Some(m::ID) {
        return;
    }
    let bound = game.native_mod_key(m::ID, m::KEY_OPEN, m::DEFAULT_OPEN);
    if bound != m::UNBOUND && key == bound {
        *canceled = true;
        m::with(|s| s.close = true);
    } else if key == "key.keyboard.space" {
        *canceled = true;
        m::with(|s| s.recenter = true);
    }
}
