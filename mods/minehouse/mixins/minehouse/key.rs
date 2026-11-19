// The wardrobe key (O) while playing: ask the server to open it.
fn minehouse_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::minehouse as m;
    use horizon_protocol::packets::play::Serverbound as S;
    use horizon_protocol::packets::configuration::CustomPayload;
    if !pressed || !game.native_on(m::ID) || !game.native_playing() {
        return;
    }
    let bound = game.native_mod_key(m::ID, m::KEY_WARDROBE, m::DEFAULT_WARDROBE);
    if bound.is_empty() || bound == "key.keyboard.unknown" || key != bound {
        return;
    }
    let packet = S::CustomPayload(CustomPayload { channel: m::WARDROBE_ACTION.into(), data: m::Action::Open.encode() });
    if game.native_send_packet(packet).is_ok() {
        m::with(|s| {
            if s.wardrobe.is_none() {
                s.asked = Some(std::time::Instant::now());
            }
        });
    }
}
