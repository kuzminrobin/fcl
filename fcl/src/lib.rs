// NOTE: crate-level attribute should be an inner attribute
// NOTE: the `#![feature]` attribute can only be used at the crate root
#![feature(specialization)]     // MaybePrint, maybe_print

#[cfg(all(feature = "single_threaded", feature = "multithreaded"))] // TODO: Test in all crates
compile_error!("Feature \"single_threaded\" and feature \"multithreaded\" cannot be enabled at the same time");

#[cfg(not(feature = "common"))]
#[macro_export]
macro_rules! set_thread_indent {
    ($expr:expr) => {};
}

#[cfg(not(feature = "common"))]
#[macro_export]
macro_rules! push_logging_is_on {
    ($expr:expr) => {};
}

#[cfg(not(feature = "common"))]
#[macro_export]
macro_rules! pop_logging_is_on {
    () => {};
}

#[cfg(not(feature = "common"))]
#[macro_export]
macro_rules! logging_is_on {
    () => { false };
}

#[cfg(not(feature = "common"))]
#[macro_export]
macro_rules! set_logging_is_on {
    ($expr:expr) => {};
}

#[cfg(feature = "common")]
pub mod common;
