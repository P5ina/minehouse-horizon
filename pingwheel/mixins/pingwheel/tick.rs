// Each client tick: register Ping Wheel's channels once a server has the
// player (Fabric sends a channel only to a client that registered it), and
// age the pings on screen.
fn pingwheel_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::pingwheel as m;
    use horizon_client::mod_api::ServerboundPlayPacket as S;
    use horizon_protocol::packets::configuration::CustomPayload;
    if !game.native_on(m::ID) {
        return;
    }
    if game.native_player().is_none() {
        m::with(|s| {
            s.registered = false;
            s.pings.clear();
        });
        return;
    }
    if !m::with(|s| s.registered) {
        let register = S::CustomPayload(CustomPayload {
            channel: m::REGISTER.into(),
            data: m::encode_register(&[m::S2C_PING]),
        });
        let channel = S::CustomPayload(CustomPayload {
            channel: m::C2S_CHANNEL.into(),
            data: m::encode_channel(""),
        });
        // Not joined yet (single player, a reconfiguring server): try next tick.
        let sent = game.native_send_packet(register);
        if sent.is_ok() {
            let _ = game.native_send_packet(channel);
            m::with(|s| s.registered = true);
            if m::debug_on() {
        game.native_chat(&format!("[pingwheel] {}", "registered ping-wheel channels"));
    }
        }
    }
    m::with(|s| s.tick());
}
