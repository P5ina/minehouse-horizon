// `AnvilScreenMixin.minehouse$noTooExpensive`: while the real cost shows, the
// "Too Expensive!" test sees the clamp the server sent, so it never fires.
fn minehouse_anvil_limit(value: i32) -> i32 {
    use crate::game::minehouse as m;
    if m::with(|s| s.drawing_real) { m::clamp(value) } else { value }
}
