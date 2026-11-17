fn coordinates_after(
    _result: (),
    corners: &mut Corners,
    look: &MinimapLook,
    batch: &mut HudBatch,
    atlas: &HudAtlas,
) {
    // Draw inside the map's reserved square, so larger frames cannot clip at screen edges.
    minimap_coordinates::OUTLINE.with(|style| {
        let (style, round) = style.get();
        if style == 0 {
            return;
        } // Preserve the original Black appearance exactly.
        let i = Corner::ALL
            .iter()
            .position(|c| *c == look.corner)
            .unwrap_or(0);
        let x = if look.corner.left() {
            2.0
        } else {
            corners.width - 2.0 - look.size
        };
        let y = if look.corner.top() {
            corners.used[i] - look.size
        } else {
            corners.height - corners.used[i]
        };
        // Keep four texture-pixel layers, each half a GUI pixel wide.
        const PIXEL: f32 = 0.5;
        let side = (look.size / PIXEL).round() as i32;
        let radius = side as f32 / 2.0;
        for py in 0..side {
            for px in 0..side {
                let dx = px as f32 + 0.5 - radius;
                let dy = py as f32 + 0.5 - radius;
                let edge = if round {
                    radius - dx.hypot(dy)
                } else {
                    (px.min(py).min(side - 1 - px).min(side - 1 - py)) as f32
                };
                if !(0.0..4.0).contains(&edge) {
                    continue;
                }
                let color = if style == 1 {
                    // From content outward: dark grey, two textured greys, pure black.
                    if edge < 1.0 {
                        rgba(0, 0, 0, 255)
                    } else if edge >= 3.0 {
                        rgba(72, 72, 72, 255)
                    } else {
                        let hash = |x: i32, y: i32| {
                            let n = (x as u32)
                                .wrapping_mul(374761393)
                                .wrapping_add((y as u32).wrapping_mul(668265263));
                            let n = (n ^ (n >> 13)).wrapping_mul(1274126177);
                            n ^ (n >> 16)
                        };
                        let fine = hash(px, py);
                        // Small mineral flecks over broader patches avoid uniform noise.
                        let grain = (fine % 9) as i32 - 4 + (hash(px / 3, py / 2) % 13) as i32 - 6;
                        let fleck = match fine & 31 {
                            0 => -9,
                            1 => 7,
                            _ => 0,
                        };
                        let corner_distance = if round {
                            (dx.abs() - dy.abs()).abs()
                        } else {
                            (px.min(side - 1 - px) as f32).hypot(py.min(side - 1 - py) as f32)
                        };
                        let highlight = (1.0 - corner_distance / 16.0).clamp(0.0, 1.0) * 12.0;
                        let bevel = if dx + dy < 0.0 { 3 } else { -3 };
                        let base = if edge < 2.0 { 186 } else { 134 };
                        let grey = (base + grain + fleck + bevel + highlight.round() as i32)
                            .clamp(0, 255) as u8;
                        rgba(grey, grey, grey, 255)
                    }
                } else if edge < 1.0 || edge >= 3.0 {
                    rgba(8, 8, 8, 255)
                } else {
                    // Keep Bamboo's existing beige frame.
                    let grain = ((px * 17 + py * 31 + (px / 3) * (py / 3) * 7) & 15) as i32 - 8;
                    let light = if dx + dy < 0.0 { 24 } else { -20 };
                    let c = [192, 171, 119].map(|v| (v + grain + light).clamp(0, 255) as u8);
                    rgba(c[0], c[1], c[2], 255)
                };
                batch.fill(
                    atlas,
                    x + px as f32 * PIXEL,
                    y + py as f32 * PIXEL,
                    PIXEL,
                    PIXEL,
                    color,
                );
            }
        }
    });
    minimap_coordinates::COORDINATES.with(|c| {
        let text = c.borrow();
        let Some(text) = text.as_deref() else {
            return;
        };
        let width = HudBatch::text_width(atlas, text) * 0.5;
        let (x, y) = if look.corner.top() {
            corners.place(look.corner, look.size, 7.0)
        } else {
            let i = Corner::ALL
                .iter()
                .position(|c| *c == look.corner)
                .unwrap_or(0);
            let y = corners.height - corners.used[i] + look.size + 2.0;
            let x = if look.corner.left() {
                2.0
            } else {
                corners.width - 2.0 - look.size
            };
            (x, y)
        };
        let tx = (x + (look.size - width) / 2.0).round();
        let ty = y + 2.0;
        let start = batch.vertices.len();
        batch.text(atlas, text, tx, ty, rgba(255, 255, 255, 255), true);
        batch.scale_since(start, tx, ty, 0.5, 0.5);
    });
}
