// The mod's row in Options > Mods, with its key.
fn pingwheel_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::pingwheel as m;
    mods.push(crate::game::native_hooks::NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: vec![[m::KEY.into(), "Ping".into(), m::DEFAULT_KEY.into()]],
    });
}
