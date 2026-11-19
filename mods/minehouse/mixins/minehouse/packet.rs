// A packet from the server: the anvil's real cost, the zones to draw, the
// wardrobe, which window is open and the end of Chapter 1; a new `Login` (a
// proxy moving us on) starts clean.
fn minehouse_packet(
    game: &mut crate::game::Game<'_>,
    packet: &horizon_client::mod_api::ClientboundPlayPacket,
) {
    use crate::game::minehouse as m;
    use horizon_client::mod_api::ClientboundPlayPacket as C;
    match packet {
        C::Login(_) => {
            m::with(|s| s.left_server());
            // Back on MineHouse after the end of Chapter 1: the title is itself again.
            horizon_client::title::minehouse_chapter::joined(game.server_address());
        }
        C::CustomPayload(p) if p.channel == m::CHAPTER_END => horizon_client::title::minehouse_chapter::mark(),
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
        C::CustomPayload(p) if p.channel == m::WARDROBE_STATE => {
            let Some(w) = m::decode_wardrobe(&p.data) else { return };
            let open = w.open;
            m::with(|s| {
                s.asked = None;
                s.wardrobe = Some(w);
            });
            if open && game.native_on(m::ID) && game.native_mod_screen() != Some(m::ID) && game.native_mod_screen_open(m::ID) {
                // Opened: show what is worn on the first tab.
                m::with(|s| {
                    let w = s.wardrobe.clone().unwrap_or_default();
                    s.ui = m::WardrobeUi::default();
                    s.ui.select_slot(&w, 0);
                });
            }
        }
        _ => {}
    }
}
