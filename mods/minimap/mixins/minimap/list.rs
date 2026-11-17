// The mod's row in Options > Mods.
fn official_minimap_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::official_minimap as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
