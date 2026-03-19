#![allow(dead_code)]

#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::{THREAD_LOGGER};

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

// TODO: Doc-comment.
pub fn flush_log() {
    // Flush the log:
    THREAD_LOGGER.with(|logger| {
        #[cfg(feature = "singlethreaded")]
        let logger = logger.borrow_mut();

        logger.borrow_mut().flush();
    });
}
