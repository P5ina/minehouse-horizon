// Syncmatica's messages from the server (`syncmatica:main`): the handshake,
// the shared placements and downloads, answered at once.
fn litematica_packet(game: &mut crate::game::Game<'_>, packet: &horizon_protocol::packets::play::Clientbound) {
    use litematica_engine as lm;
    let horizon_protocol::packets::play::Clientbound::CustomPayload(p) = packet else {
        return;
    };
    if p.channel != lm::sync::CHANNEL || !game.native_on(lm::ID) {
        return;
    }
    let dir = game.native_instance_dir().join("schematics");
    let ru = horizon_client::lang::code().starts_with("ru");
    let (lines, out) = lm::with(|m| {
        let lines = match m.sync.receive(&p.data) {
            Ok(events) => m.sync_events(events, &dir, ru),
            Err(_) => Vec::new(),
        };
        (lines, std::mem::take(&mut m.sync.out))
    });
    for data in out {
        game.send(horizon_protocol::packets::play::Serverbound::CustomPayload(
            horizon_protocol::packets::configuration::CustomPayload { channel: lm::sync::CHANNEL.into(), data },
        ));
    }
    for line in lines {
        game.native_chat(&line);
    }
}
