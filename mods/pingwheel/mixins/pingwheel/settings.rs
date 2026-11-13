// Ping Wheel Settings (`SettingsScreen`): each row as its option reads.
fn pingwheel_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::pingwheel as m;
    if mod_id != m::ID {
        return;
    }
    let g = &*game;
    m::with(|s| {
        s.load(&|k| g.native_setting(m::ID, k));
        buttons.extend(s.config.buttons(&s.channel));
    });
}
