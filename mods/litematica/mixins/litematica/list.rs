// The mod's row in Options > Mods, with its keys.
fn litematica_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use litematica_engine as lm;
    mods.push(crate::game::native_hooks::NativeMod {
        id: lm::ID.into(),
        title: format!("{} {}", lm::NAME, lm::VERSION),
        description: lm::DESCRIPTION.into(),
        keys: vec![
            [lm::KEY_PRINTER.into(), "Toggle Printer".into(), lm::DEFAULT_PRINTER.into()],
            [lm::KEY_RENDER.into(), "Toggle Schematic Rendering".into(), lm::DEFAULT_RENDER.into()],
        ],
    });
}
