// The mod's row in Options > Mods, with its key.
fn worldmap_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::worldmap as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: vec![[m::KEY_OPEN.into(), "Open World Map".into(), m::DEFAULT_OPEN.into()]],
    });
}
