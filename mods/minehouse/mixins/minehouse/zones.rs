// The spawn zones' borders (`ZoneRenderer`), each frame, as world primitives.
fn minehouse_zones(
    game: &crate::game::Game<'_>,
    primitives: &mut horizon_client::world_primitives::WorldPrimitives,
) {
    use crate::game::minehouse as m;
    if !game.native_on(m::ID) {
        return;
    }
    let Some(p) = game.native_player() else {
        return;
    };
    let draws = m::with(|s| if s.shapes.is_empty() { Vec::new() } else { m::draw_shapes(&s.shapes, [p.x, p.y, p.z]) });
    for d in draws {
        match d {
            m::Draw::Line { from, to, color, width } => {
                primitives.line(from, to, color, width);
            }
            m::Draw::Wall { from, to, height, color } => {
                for (min, max) in m::wall_boxes(from, to, height) {
                    primitives.filled_box(min, max, color);
                }
            }
            m::Draw::Point { at, color, size } => {
                let r = f64::from(size) * 0.01;
                primitives.filled_box([at[0] - r, at[1] - r, at[2] - r], [at[0] + r, at[1] + r, at[2] + r], color);
            }
            m::Draw::Text { at, text, color, scale } => {
                primitives.text(at, text, color, scale);
            }
        }
    }
}
