// `/lm …`: Litematica's commands, answered in the chat.
fn litematica_command(game: &mut crate::game::Game<'_>, command: &str, ran: &mut bool) {
    use litematica_engine as lm;
    if *ran || !game.native_on(lm::ID) || command.split_whitespace().next() != Some(lm::COMMAND) {
        return;
    }
    *ran = true;
    let Some(p) = game.native_player() else { return };
    let dir = game.native_instance_dir().join("schematics");
    let _ = std::fs::create_dir_all(&dir);
    let lines = {
        let g = &*game;
        let have = |item: &str| -> u32 {
            g.inventory.iter().chain(std::iter::once(&g.offhand)).flatten().filter(|s| &*s.item == item).map(|s| s.count).sum()
        };
        let done = |pos: [i32; 3], want: &lm::schematic::State| -> bool {
            if !g.world.chunks.contains_key(&(pos[0] >> 4, pos[2] >> 4)) {
                return false;
            }
            let have = g.world.block(pos[0], pos[1], pos[2]);
            g.native_state_block(have) == Some(want.name.as_str())
        };
        let ctx = lm::Ctx {
            russian: horizon_client::lang::code().starts_with("ru"),
            dir: &dir,
            feet: [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32],
            facing: lm::facing_of(p.yaw),
            have: &have,
            done: &done,
        };
        lm::command(command, &ctx)
    };
    for line in lines.unwrap_or_default() {
        game.native_chat(&line);
    }
}
