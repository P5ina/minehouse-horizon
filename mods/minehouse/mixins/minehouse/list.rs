// The mod's row in Options > Mods, with the wardrobe key.
fn minehouse_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::minehouse as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: vec![[m::KEY_WARDROBE.into(), "Open Wardrobe".into(), m::DEFAULT_WARDROBE.into()]],
    });
}
