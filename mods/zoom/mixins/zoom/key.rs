// The zoom key went down or up: the player's choice of key, kept as the
// WebAssembly Zoom kept it.
fn zoom_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::official_zoom as zoom;
    // Turned off in Options > Mods: nothing until the next build leaves it out.
    if !game.native_on(zoom::ID) {
        return;
    }
    let bound = game.native_mod_key(zoom::ID, zoom::KEY, zoom::DEFAULT_KEY);
    if key != bound {
        return;
    }
    let playing = game.native_playing();
    let game = &*game;
    zoom::key(playing, pressed, |name| game.native_setting(zoom::ID, name));
}
