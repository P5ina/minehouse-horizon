thread_local! {
 pub static COORDINATES: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}
thread_local! {
 pub static OUTLINE: std::cell::Cell<(usize, bool)> = const { std::cell::Cell::new((0, false)) };
}
