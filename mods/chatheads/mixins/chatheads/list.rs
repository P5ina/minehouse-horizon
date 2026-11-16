// The mod's row in Options > Mods.
fn chatheads_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::chatheads as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
