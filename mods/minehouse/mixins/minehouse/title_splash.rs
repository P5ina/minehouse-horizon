// No splash over the black title (the title's frame-by-frame poll).
fn minehouse_title_splash(title: &mut Title) {
    if title.splash.is_some() && crate::title::minehouse_chapter::active() {
        title.splash = None;
    }
}
