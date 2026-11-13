// The ping key: mark the block the camera looks at for the players near you.
fn pingwheel_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::pingwheel as m;
    use horizon_client::mod_api::ServerboundPlayPacket as S;
    use horizon_protocol::packets::configuration::CustomPayload;
    if !pressed || !game.native_on(m::ID) || !game.native_playing() {
        return;
    }
    if key != game.native_mod_key(m::ID, m::KEY, m::DEFAULT_KEY) {
        return;
    }
    let Some(cam) = game.native_camera() else {
        return;
    };
    let dir = m::view_vector(cam.yaw, cam.pitch);
    let Some(hit) = game.clip_ray_fluids(cam.position, dir, m::RAYCAST_DISTANCE, true) else {
        return;
    };
    let ping = m::Ping {
        channel: String::new(),
        pos: hit.point,
        entity: None,
        sequence: m::with(|s| s.next_sequence()),
        dimension: m::dimension_hash(&cam.dimension),
        author: None,
    };
    let _ = game.native_send_packet(S::CustomPayload(CustomPayload {
        channel: m::C2S_PING.into(),
        data: m::encode_ping(&ping),
    }));
}
