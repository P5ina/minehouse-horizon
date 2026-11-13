//! Plasmo Voice's own icons (`assets/plasmovoice/textures/icons`, LGPL-3.0,
//! see LICENSE-plasmovoice-assets), as one RGBA atlas the game uploads into
//! the mod image slot: 16 by 16 each, 16 to a row.
pub const SIZE: u32 = 16;
pub const PER_ROW: u32 = 16;

const RGBA: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/icons.rgba"));
const NAMES: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/icons.txt"));

pub fn names() -> impl Iterator<Item = &'static str> {
    NAMES.lines().map(str::trim).filter(|l| !l.is_empty())
}

/// `plasmovoice:textures/icons/speaker.png` → `speaker`, when it is one of ours.
pub fn name_of(location: &str) -> Option<&'static str> {
    let base = location.rsplit('/').next()?.strip_suffix(".png").unwrap_or(location);
    names().find(|n| *n == base)
}

/// The icon's cell in the atlas, `[x, y, w, h]` in its pixels.
pub fn cell(name: &str) -> Option<[f32; 4]> {
    let i = names().position(|n| n == name)? as u32;
    Some([(i % PER_ROW * SIZE) as f32, (i / PER_ROW * SIZE) as f32, SIZE as f32, SIZE as f32])
}

/// The atlas: its width, height and pixels.
pub fn atlas() -> (u32, u32, Vec<u8>) {
    let count = names().count() as u32;
    let (w, h) = (PER_ROW * SIZE, count.div_ceil(PER_ROW) * SIZE);
    let mut out = vec![0u8; (w * h * 4) as usize];
    for i in 0..count {
        let (cx, cy) = (i % PER_ROW * SIZE, i / PER_ROW * SIZE);
        for row in 0..SIZE {
            let src = ((i * SIZE * SIZE + row * SIZE) * 4) as usize;
            let dst = (((cy + row) * w + cx) * 4) as usize;
            out[dst..dst + (SIZE * 4) as usize].copy_from_slice(&RGBA[src..src + (SIZE * 4) as usize]);
        }
    }
    (w, h, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_its_pixels_and_a_cell() {
        assert_eq!(RGBA.len(), names().count() * 16 * 16 * 4);
        assert_eq!(name_of("plasmovoice:textures/icons/speaker_muted.png"), Some("speaker_muted"));
        assert_eq!(cell("headset_disabled"), Some([0.0, 0.0, 16.0, 16.0]));
        let (w, h, px) = atlas();
        assert_eq!((w, px.len()), (256, (w * h * 4) as usize));
        // The microphone icon is not empty.
        let [x, y, _, _] = cell("microphone").unwrap();
        let opaque = (0..16).any(|r| (0..16).any(|c| px[(((y as u32 + r) * w + x as u32 + c) * 4 + 3) as usize] > 0));
        assert!(opaque);
    }
}
