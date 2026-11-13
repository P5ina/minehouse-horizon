// A packet from the server: Ping Wheel's pings (`PingManager.acceptPingPacket`).
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
    let Some(ping) = m::decode_ping(&payload.data) else {
        return;
    };
    let Some(me) = game.native_player() else {
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
    let (pos, dimension) = (ping.pos, ping.dimension);
    let (accepted, volume) = m::with(|s| (s.accept(ping, name, [me.x, me.y, me.z]), s.config.ping_volume));
    // `playSoundInDimension`: Ping Wheel's own sound, where the ping is.
    let here = game.native_camera().map(|c| m::dimension_hash(&c.dimension));
    if accepted && volume > 0 && here == Some(dimension) {
        let pcm = horizon_client::native_data!("pingwheel", "ping.pcm");
        let samples = m::ping_sound(pcm, volume as f32 / 100.0);
        let source = (u128::from(pos[0].to_bits()) << 64) ^ u128::from(pos[2].to_bits());
        game.native_push_voice(m::ID, source, &samples, 48_000, None);
    }
}
