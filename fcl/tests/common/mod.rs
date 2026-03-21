#![allow(dead_code)]

#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::THREAD_LOGGER;

// TODO: Use everywhere.
macro_rules! substitute_log_writer {
    () => {
        {
            // Create the mock log writer and substitute the default one with it:
            let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));

            fcl::call_log_infra::instances::THREAD_DECORATOR
                .with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

            log
        }
    };
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

/// Flushes the log (to log the cached calls, repeat count, to prevent subsequent call caching).
pub fn flush_log() {
    // Flush the log:
    THREAD_LOGGER.with(|logger| {
        #[cfg(feature = "singlethreaded")]
        let logger = logger.borrow_mut();

        logger.borrow_mut().flush();
    });
}
