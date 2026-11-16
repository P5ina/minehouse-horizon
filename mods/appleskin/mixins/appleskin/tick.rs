// Each client tick: register AppleSkin's channels once a server has the
// player, and move the flash (`HUDOverlayHandler.onClientTick`).
fn appleskin_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::appleskin as m;
    use horizon_client::mod_api::ServerboundPlayPacket as S;
    use horizon_protocol::packets::configuration::CustomPayload;
    if game.remote.is_none() || game.native_player().is_none() {
        m::with(|s| s.left_server());
        return;
    }
    if !game.native_on(m::ID) {
        return;
    }
    m::with(|s| s.flash.tick());
    if m::with(|s| s.registered) {
        return;
    }
    let register = S::CustomPayload(CustomPayload {
        channel: m::REGISTER.into(),
        data: m::encode_register(&m::CHANNELS),
    });
    if game.native_send_packet(register).is_ok() {
        m::with(|s| s.registered = true);
    }
}
