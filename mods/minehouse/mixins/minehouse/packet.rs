// A packet from the server: the anvil's real cost, the zones to draw, and
// which window is open; a new `Login` (a proxy moving us on) starts clean.
fn minehouse_packet(
    game: &mut crate::game::Game<'_>,
    packet: &horizon_client::mod_api::ClientboundPlayPacket,
) {
    use crate::game::minehouse as m;
    use horizon_client::mod_api::ClientboundPlayPacket as C;
    let _ = &game;
    match packet {
        C::Login(_) => m::with(|s| s.left_server()),
        C::OpenScreen(p) => m::with(|s| s.open_container = Some(p.container)),
        C::ContainerClose(p) => m::with(|s| {
            if s.open_container == Some(p.container) {
                s.open_container = None;
            }
        }),
        C::CustomPayload(p) if p.channel == m::ANVIL_COST => {
            if let Some((container, cost)) = m::decode_anvil_cost(&p.data) {
                m::with(|s| s.anvil = m::AnvilCost { container: Some(container), cost });
            }
        }
        C::CustomPayload(p) if p.channel == m::ZONE_SHAPES => {
            if let Some(shapes) = m::decode_zone_shapes(&p.data) {
                m::with(|s| s.shapes = shapes);
            }
        }
        _ => {}
    }
}
