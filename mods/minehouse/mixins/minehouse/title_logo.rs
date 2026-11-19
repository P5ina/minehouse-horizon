// TitleScreen's logo: after the end of Chapter 1 the title is black and the dot
// is drawn in its place (`minehouse_chapter::draw`).
fn minehouse_title_logo(atlas: &HudAtlas, batch: &mut HudBatch, w: f32) -> std::ops::ControlFlow<()> {
    if crate::title::minehouse_chapter::active() {
        crate::title::minehouse_chapter::draw(batch, atlas, w);
        return std::ops::ControlFlow::Break(());
    }
    std::ops::ControlFlow::Continue(())
}
