// The mod's row in Options > Mods, with Ping Wheel's two keys.
fn pingwheel_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::pingwheel as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: vec![
            [m::KEY_PING.into(), "Ping Location".into(), m::DEFAULT_PING.into()],
            [m::KEY_SETTINGS.into(), "Open Settings".into(), m::DEFAULT_SETTINGS.into()],
        ],
    });
}
