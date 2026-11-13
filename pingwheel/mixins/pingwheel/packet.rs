// A packet from the server: Ping Wheel's pings go on the screen.
fn pingwheel_packet(
    game: &mut crate::game::Game<'_>,
    packet: &horizon_client::mod_api::ClientboundPlayPacket,
) {
    use crate::game::pingwheel as m;
    use horizon_client::mod_api::ClientboundPlayPacket as C;
    if !game.native_on(m::ID) {
        return;
    }
    let C::CustomPayload(payload) = packet else {
        return;
    };
    if payload.channel != m::S2C_PING {
        return;
    }
    if m::debug_on() {
        game.native_chat(&format!("[pingwheel] {}", "ping received"));
    }
    let Some(ping) = m::decode_ping(&payload.data) else {
        return;
    };
    let name = ping
        .author
        .and_then(|a| {
            game.remote_tab_players()
                .and_then(|(players, _, _)| players.into_iter().find(|p| p.uuid == a))
                .map(|p| p.name)
        })
        .unwrap_or_default();
    m::with(|s| s.add(ping, name));
}
