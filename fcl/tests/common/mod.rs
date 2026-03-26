#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;
#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::THREAD_LOGGER;

// TODO: Doc-comment.
pub(crate) const COORDS_ONLY_RE_SLICE: &str = r"^\d+,\d+:\d+,\d+$";
pub(crate) const COORDS_RE_SLICE: &str = r"\d+,\d+:\d+,\d+";

pub(crate) fn substitute_log_writer() -> Rc<RefCell<Vec<u8>>> {
    // Create the mock log writer and substitute the default one with it:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));

    fcl::call_log_infra::instances::THREAD_DECORATOR
        .with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    log
}

// TODO: Consider coverting to a macro to preserve the error coordinates in `panic`.
pub(crate) fn zero_out_closure_coords(log: Rc<RefCell<Vec<u8>>>) -> String {
    let log_contents = unsafe { String::from(std::str::from_utf8_unchecked(&*log.borrow())) };
    let coords_regex = match regex::Regex::new(COORDS_RE_SLICE) {
        Result::Ok(coords_regex) => coords_regex,
        Result::Err(error) => panic!(
            "Test Crate Internal Error: Failed to create Regex from \"{}\", error: \"{}\"",
            COORDS_RE_SLICE, error),
    };
    let output = coords_regex.replace_all(&log_contents, "0,0:0,0");
    output.to_string()
}


// TODO: Use everywhere.
// TODO:
// * Remove `#[macro_export]`.
// * `pub(crate) use crate::test_assert;`
#[macro_export]
// TODO: Doc-comment.
// NOTE: Extracting this to a macro rather than a function
// in order to preserve the line numbers (inside of the tests) in the assertion failure reports.
macro_rules! test_assert {
    ($log:ident, $expected_str:expr $(,)?) => {
        unsafe {
            let log_contents = String::from(std::str::from_utf8_unchecked(&*$log.borrow()));
            assert_eq!(log_contents, $expected_str)
        }
    };
}
pub use crate::test_assert; // Re-export as `crate::common::test_assert` (in addition to `crate::test_assert`).
// TODO: Consider `-> pub(crate) use test_assert`.

macro_rules! get_coords_slice {
    ($log_contents:expr, $beginning:expr, $end:expr $(,)?) => {{

        // Check the closure coordiantes:
        let coords_start_idx = $beginning.len(); // `0..`

        // NOTE: Redundant if invoked from `assert_begin_coords_end!()` that asserts the log ends with `end`.
        assert!(
            $end.len() <= $log_contents.len(),
            concat!(
                "The log is shorter than the expected end:\n",
                "actual log:\n",
                "\"{}\"\n",
                "expected end:\n",
                "\"{}\"\n",
            ),
            $log_contents,
            $end
        );
        let coords_end_idx = $log_contents.len() - $end.len();    // 0..=log_contents.len()

        assert!(
            coords_start_idx <= $log_contents.len() // NOTE: Redundant if invoked from `assert_begin_coords_end!()` that asserts the log starts with `beginning`.
                && coords_start_idx <= coords_end_idx,
            concat!(
                "The log is too short:\n",
                "actual log:\n",
                "\"{}\"\n",
                "expected at least the len of:\n",
                "\"{}{}\"\n",
            ),
            $log_contents,
            $beginning,
            $end,
        );

        &$log_contents[coords_start_idx .. coords_end_idx]
    }}
}
pub(crate) use get_coords_slice;

macro_rules! assert_coords_slice {
    ($coords_slice:expr) => {{
        // NOTE: Failed to make the var below a global const (non-const init function?):
        // Compiler Error: "cycle detected when checking if `common::coords_regex` is a trivial const".
        let coords_regex = match regex::Regex::new(COORDS_ONLY_RE_SLICE) {
            Result::Ok(coords_regex) => coords_regex,
            Result::Err(error) => panic!(
                "Test Crate Internal Error: Failed to create Regex from \"{}\", error: \"{}\"",
                COORDS_ONLY_RE_SLICE, error),
        };

        let optional_coords_re_match = coords_regex.find($coords_slice);
        assert!(optional_coords_re_match.is_some(),
            concat!(
                "Closure coordinates don't match the expected regular expression:\n",
                "actual        : \"{}\"\n",
                "expected regex: \"{}\"\n",
            ),
            $coords_slice,
            COORDS_ONLY_RE_SLICE
        );
    }}
}
pub(crate) use assert_coords_slice;

/// Asserts that
/// * the `log` starts with the `beginning`,
/// * the `log` ends with the `end`,
/// * the log slice between `beginning` and `end` matches the closure coordinates regular expression.
///
/// ### Parameters
/// * The log to search for `beginning` and `end` in.
///   Is expected to be the one created with `substitute_log_writer()`.
/// * The substring the log is expected to start with.
/// * The substring the log is expected to end with.
// TODO: Rename assert_except_closure_coords
macro_rules! assert_begin_coords_end {
    ($log:expr, $beginning:expr, $end:expr $(,)?) => {{
        let log_contents = unsafe { String::from(std::str::from_utf8_unchecked(&*$log.borrow())) };

        assert!(
            log_contents.starts_with($beginning),
            concat!(
                "The log starts with an unexpected contents:\n",
                "log contents:\n", 
                "\"{}\"\n",
                "expected beginning:\n",
                "\"{}\"\n",
            ),
            log_contents,
            $beginning
        );
        assert!(
            log_contents.ends_with($end),
            concat!(
                "The log ends with an unexpected contents:\n",
                "log contents:\n",
                "\"{}\"\n",
                "expected end:\n",
                "\"{}\"\n",
            ),
            log_contents,
            $end
        );

        let coords_slice = $crate::common::get_coords_slice!(log_contents, $beginning, $end);
        $crate::common::assert_coords_slice!(coords_slice);

    }};
}
pub(crate) use assert_begin_coords_end;

/// * Flushes the log
/// * Invokes `assert_except_closure_coords!()` with the same parameters
/// * Clears the log
///
/// ### Parameters
/// * The log to search for `beginning` and `end` in.
///   Expected to be the one created with `substitute_log_writer()`.
/// * The substring the log is expected to start with.
/// * The substring the log is expected to end with.
// TODO: Consider renaming.
macro_rules! assert_coords_are_in_between {
    ($log:expr, $beginning:expr, $end:expr $(,)?) => {{
        flush_log();
        assert_begin_coords_end!($log, $beginning, $end,);
        $log.borrow_mut().clear();
    }};
}
pub(crate) use assert_coords_are_in_between;

/// Flushes the log (to log the cached calls, repeat count, to prevent subsequent call caching).
pub fn flush_log() {
    // Flush the log:
    THREAD_LOGGER.with(|logger| {
        #[cfg(feature = "singlethreaded")]
        let logger = logger.borrow_mut();

        logger.borrow_mut().flush();
    });
}
