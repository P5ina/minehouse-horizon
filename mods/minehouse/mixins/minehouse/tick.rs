// Each client tick: register our channels once a server has the player (the
// server mods learn from it that this client draws the cost and the zones,
// and has the wardrobe), send what the wardrobe's clicks asked for, and say
// so when the server has no wardrobe.
fn minehouse_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::minehouse as m;
    use horizon_client::mod_api::ServerboundPlayPacket as S;
    use horizon_protocol::packets::configuration::CustomPayload;
    // MineHouse players get Horizon's Mouse Tweaks controls on: once, so a
    // player who switches them off in Extras keeps them off.
    if !m::with(|s| s.tweaks_checked) && game.native_on(m::ID) {
        m::with(|s| s.tweaks_checked = true);
        let dir = game.native_data_dir(m::ID);
        let marker = dir.join(m::MOUSE_TWEAKS_MARKER);
        if !marker.exists() {
            if !game.settings.extras.mouse_tweaks {
                game.settings.extras.mouse_tweaks = true;
                game.settings.save(&crate::game::Game::options_path());
            }
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(&marker, "");
        }
    }
    if game.remote.is_none() || game.native_player().is_none() {
        m::with(|s| s.left_server());
        return;
    }
    if !game.native_on(m::ID) {
        return;
    }
    if !m::with(|s| s.registered) {
        let register = S::CustomPayload(CustomPayload {
            channel: m::REGISTER.into(),
            data: m::encode_register(&m::CHANNELS),
        });
        if game.native_send_packet(register).is_ok() {
            m::with(|s| s.registered = true);
        }
    }
    for action in m::with(|s| std::mem::take(&mut s.out)) {
        let packet = S::CustomPayload(CustomPayload { channel: m::WARDROBE_ACTION.into(), data: action.encode() });
        let _ = game.native_send_packet(packet);
    }
    let silent = m::with(|s| s.asked.is_some_and(|at| at.elapsed() >= m::NO_WARDROBE_AFTER));
    if silent {
        m::with(|s| s.asked = None);
        let line = horizon_client::mod_api::Text::literal("На этом сервере нет гардероба");
        game.native_client_action_bar(line, false);
    }
}
