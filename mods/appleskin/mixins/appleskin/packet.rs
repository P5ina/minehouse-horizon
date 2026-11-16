// The server's AppleSkin: saturation, exhaustion and natural regeneration.
fn appleskin_packet(
    game: &mut crate::game::Game<'_>,
    packet: &horizon_client::mod_api::ClientboundPlayPacket,
) {
    use crate::game::appleskin as m;
    use horizon_client::mod_api::ClientboundPlayPacket as C;
    let _ = &game;
    match packet {
        C::Login(_) => m::with(|s| s.left_server()),
        C::CustomPayload(p) if p.channel == m::SATURATION => {
            if let Some(v) = m::decode_f32(&p.data) {
                m::with(|s| s.saturation = Some(v));
            }
        }
        C::CustomPayload(p) if p.channel == m::EXHAUSTION => {
            if let Some(v) = m::decode_f32(&p.data) {
                m::with(|s| s.exhaustion = v);
            }
        }
        C::CustomPayload(p) if p.channel == m::NATURAL_REGENERATION => {
            if let Some(v) = m::decode_bool(&p.data) {
                m::with(|s| s.natural_regeneration = v);
            }
        }
        _ => {}
    }
}
