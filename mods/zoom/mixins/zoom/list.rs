// Zoom's row in Options > Mods, with its key.
fn zoom_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::official_zoom as zoom;
    mods.push(crate::game::native_hooks::NativeMod {
        id: zoom::ID.into(),
        title: format!("Zoom {}", zoom::VERSION),
        description: zoom::DESCRIPTION.into(),
        keys: vec![[zoom::KEY.into(), "Zoom key".into(), zoom::DEFAULT_KEY.into()]],
    });
}
