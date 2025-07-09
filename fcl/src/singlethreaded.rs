#![cfg(feature = "singlethreaded")]

use fcl_traits::CallLogger;

/// #### Examples
/// ```rs
/// fcl::set_thread_indent!(String::from("                "));
/// ```
#[macro_export]
macro_rules! set_thread_indent {
    ($expr:expr) => {
        fcl::call_log_infra::instances::THREAD_LOGGER
            .with(|logger| logger.borrow_mut().borrow_mut().set_thread_indent($expr))
    };
}
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
