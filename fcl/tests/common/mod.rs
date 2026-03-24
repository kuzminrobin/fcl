#![allow(dead_code)]

#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::THREAD_LOGGER;

// TODO: Use everywhere.
macro_rules! substitute_log_writer {
    () => {{
        // Create the mock log writer and substitute the default one with it:
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));

        fcl::call_log_infra::instances::THREAD_DECORATOR
            .with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        log
    }};
}
pub(crate) use substitute_log_writer;

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

macro_rules! assert_except_closure_coords {
    ($log:expr, $start:expr, $end:expr) => {{
        flush_log();
        let log_contents = unsafe { String::from(std::str::from_utf8_unchecked(&*$log.borrow())) };

        let optional_start_match_index = log_contents.find($start);
        // Assert: The `start` is found,
        if let Some(match_start_index) = optional_start_match_index {
            // at the beginning of the `log_contents`.
            assert_eq!(0, match_start_index);

            let shortest_closure_coords = &"0,0:0,0";
            let min_expected_log_len = $start.len() + shortest_closure_coords.len();
            assert!(
                min_expected_log_len <= log_contents.len(),
                concat!(
                    "The log is too short.\n",
                    "Log: \"{}\"\n",
                    "Expected at least the length of \"{}{}\"",
                ),
                log_contents,
                $start,
                shortest_closure_coords
            );
            // The tail after the {`start` followed by the closure coordinates}:
            let tail = &log_contents[min_expected_log_len..];
            // Assert: The end is found in the tail.
            let optional_end_match_index = tail.find($end);
            assert!(
                optional_end_match_index.is_some(),
                concat!(
                    "Failed to find the end search substring in the tail:\n",
                    "tail: \"{}\"\n",
                    "end : \"{}\"",
                ),
                tail,
                $end
            );
        } else {
            assert!(
                false,
                concat!(
                    "Failed to find the start search substring:\n",
                    "log_contents: \"{}\"\n",
                    "start       : \"{}\"",
                ),
                log_contents, $start
            );
        }
        $log.borrow_mut().clear();
    }};
}
pub(crate) use assert_except_closure_coords;

/// Flushes the log (to log the cached calls, repeat count, to prevent subsequent call caching).
pub fn flush_log() {
    // Flush the log:
    THREAD_LOGGER.with(|logger| {
        #[cfg(feature = "singlethreaded")]
        let logger = logger.borrow_mut();

        logger.borrow_mut().flush();
    });
}
