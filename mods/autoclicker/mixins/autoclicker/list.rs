// The mod's row in Options > Mods, with its four keys.
fn autoclicker_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::autoclicker as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: vec![
            [m::KEY_LEFT.into(), "Toggle Left Autoclick".into(), m::DEFAULT_LEFT.into()],
            [m::KEY_RIGHT.into(), "Toggle Right Autoclick".into(), m::DEFAULT_RIGHT.into()],
            [m::KEY_HOLD_LEFT.into(), "Toggle Hold Left Click".into(), m::UNBOUND.into()],
            [m::KEY_HOLD_RIGHT.into(), "Toggle Hold Right Click".into(), m::UNBOUND.into()],
        ],
    });
}
