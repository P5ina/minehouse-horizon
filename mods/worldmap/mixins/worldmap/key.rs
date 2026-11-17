// The map key opens the map while playing.
fn worldmap_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::worldmap as m;
    if !pressed || !game.native_on(m::ID) || !game.native_playing() {
        return;
    }
    let bound = game.native_mod_key(m::ID, m::KEY_OPEN, m::DEFAULT_OPEN);
    if bound != m::UNBOUND && key == bound {
        game.native_mod_screen_open(m::ID);
    }
}
