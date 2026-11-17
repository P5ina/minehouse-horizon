// Zoom's settings page: the WebAssembly Zoom's buttons.
fn zoom_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::official_zoom as zoom;
    if mod_id != zoom::ID {
        return;
    }
    let game = &*game;
    let settings = zoom::settings(|key| game.native_setting(zoom::ID, key));
    buttons.extend(settings.labels());
}
