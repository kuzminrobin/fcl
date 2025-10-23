/// Sets a specific thread indent different from the default for the invoking thread.
/// #### Examples
/// ```rs
/// fcl::set_thread_indent!(String::from("                "));
/// ```
#[macro_export]
macro_rules! set_thread_indent {
    ($expr:expr) => {
        fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
            logger
                .borrow_mut()
                .borrow_mut() // This is the difference from multithreaded.rs. The 
                // `#[cfg(feature = "singlethreaded")]` cannot be applied to this line only.
                .set_thread_indent($expr)
        })
    };
}

/// Temporarily enables or disables the call logging for the invoking thread.
/// #### Examples
/// ```rs
/// fcl::push_logging_is_on!(true); // Temporarily enable logging.
/// fcl::pop_logging_is_on!();  // Revert to previous logging state.
///
/// fcl::push_logging_is_on!(false); // Temporarily disable logging.
/// fcl::pop_logging_is_on!();  // Revert to previous logging state.
/// ```
#[macro_export]
macro_rules! push_logging_is_on {
    ($expr:expr) => {
        fcl::call_log_infra::instances::THREAD_LOGGER
            .with(|logger| logger.borrow_mut().borrow_mut().push_logging_is_on($expr))
    };
}

/// Reverts to the previous logging state (enabled/disabled) for the invoking thread.
/// #### Examples
/// ```rs
/// fcl::push_logging_is_on!(true); // Temporarily enable logging.
/// fcl::pop_logging_is_on!();  // Revert to previous logging state.
///
/// fcl::push_logging_is_on!(false); // Temporarily disable logging.
/// fcl::pop_logging_is_on!();  // Revert to previous logging state.
/// ```
#[macro_export]
macro_rules! pop_logging_is_on {
    () => {
        fcl::call_log_infra::instances::THREAD_LOGGER
            .with(|logger| logger.borrow_mut().borrow_mut().pop_logging_is_on())
    };
}

/// Tells if call logging is enabled (by returning `true`) or disabled (by returning `false`) 
/// #### Examples
/// ```rs
/// let on = fcl::logging_is_on!();
/// ```
#[macro_export]
macro_rules! logging_is_on {
    () => {
        fcl::call_log_infra::instances::THREAD_LOGGER
            .with(|logger| logger.borrow().borrow().logging_is_on())
    };
}

/// Enables (if the argument is `true`) or disables (if the argument is `false`) the call logging
/// for the invoking thread.
/// #### Examples
/// ```rs
/// fcl::set_logging_is_on!(false); // Disable logging.
/// fcl::set_logging_is_on!(true); // Enable logging.
/// ```
#[macro_export]
macro_rules! set_logging_is_on {
    ($expr:expr) => {
        fcl::call_log_infra::instances::THREAD_LOGGER
            .with(|logger| logger.borrow_mut().borrow_mut().set_logging_is_on($expr))
    };
}

impl Drop for crate::call_log_infra::CallLoggerArbiter {
    /// Flushes
    /// * the repeat count,
    /// * the std output
    /// in the end of `main()`, if the `main()` itself is not logged (but the internals are).
    fn drop(&mut self) {
        self.remove_thread_logger(); // In multithreaded case this is done by `ThreadGateAdapter::drop()`.
    }
}
