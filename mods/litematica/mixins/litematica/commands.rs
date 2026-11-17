// `/lm` completes and colours as the game's own commands do.
fn litematica_commands(commands: &mut Vec<&'static str>) {
    commands.push(litematica_engine::COMMAND);
}
