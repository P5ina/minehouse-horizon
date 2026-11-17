// A click on Zoom's settings page: change the setting and keep it.
fn zoom_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::official_zoom as zoom;
    if mod_id != zoom::ID {
        return;
    }
    let mut settings = zoom::settings(|key| game.native_setting(zoom::ID, key));
    settings.click(button);
    zoom::set_settings(settings);
    for (key, value) in settings.stored() {
        game.native_set_setting(zoom::ID, key, &value);
    }
}
