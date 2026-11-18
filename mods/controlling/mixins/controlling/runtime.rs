// Controlling for Horizon: the part of Controlling 26.3
// (https://github.com/jaredlll08/Controlling, MIT) Horizon's Key Binds screen
// lacks. Horizon already has the search box (by name) and marks conflicting
// keys; this adds searching by the bound key and by category. The mod's code is
// the injection in search.rs; this module only names the mod.

pub const ID: &str = "controlling";
pub const NAME: &str = "Controlling";
pub const VERSION: &str = "26.3.5";
pub const DESCRIPTION: &str =
    "Key Binds search also finds a binding by its key (\"G\", \"Left Button\") and by its category.";
