// The settings page (`AutoclickerSettingsScreen`): one button per interval.
fn autoclicker_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::autoclicker as m;
    if mod_id != m::ID {
        return;
    }
    let ms = |k: &str| game.native_setting(m::ID, k).and_then(|v| v.parse().ok()).unwrap_or(0);
    let russian = horizon_client::lang::code().starts_with("ru");
    buttons.extend(m::buttons(russian, ms(m::SETTING_LEFT), ms(m::SETTING_RIGHT)));
}
