use std::sync::OnceLock;

use super::{EnvDefSource, EnvDefs};

macro_rules! sources {
    ($($filename:literal),*) => {
        &[$(
            EnvDefSource::new_static($filename, include_str!(concat!("../../../bundled-defs/", $filename))),
        )*]
    }
}

pub static BUNDLED_SOURCES: &[EnvDefSource] = sources![
    "builtin_funcs.ydef",
    "context_funcs.ydef",
    "ext_plugin_funcs.ydef",
    "general_funcs.ydef",
    "interaction_funcs.ydef"
];

pub fn load() -> &'static EnvDefs {
    /// Get the bundled envdefs. The envdefs are parsed on first call and cached
    /// for subsequent calls.
    static BUNDLED_ENVDEFS: OnceLock<EnvDefs> = OnceLock::new();
    BUNDLED_ENVDEFS.get_or_init(|| super::parse(BUNDLED_SOURCES).expect("bundled sources should be valid"))
}

#[test]
fn bundled_sources_are_valid() {
    super::parse(BUNDLED_SOURCES).expect("bundled sources should be valid");
}
