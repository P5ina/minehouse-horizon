// `/vc …`: the voice chat's own command, answered in the chat.
fn voicechat_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    use plasmovoice_engine as vc;
    if *ran || !game.native_on(vc::ID) {
        return;
    }
    {
        let game = &*game;
        vc::load(&|k| game.native_setting(vc::ID, k));
    }
    // `/vc menu` opens the settings screen, as V does.
    if command.trim() == "vc menu" || command.trim() == "vc settings" {
        *ran = true;
        vc::menu_open(None);
        if !game.native_mod_screen_open(vc::ID) {
            vc::menu_sync(false);
        }
        return;
    }
    let Some(lines) = vc::command(command) else {
        return;
    };
    *ran = true;
    for line in lines {
        game.native_chat(&line);
    }
    for (k, v) in vc::take_store() {
        game.native_set_setting(vc::ID, &k, &v);
    }
}
