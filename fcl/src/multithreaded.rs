use std::{cell::RefCell, rc::Rc, sync::Arc};
pub use std::{
    sync::{Mutex, MutexGuard},
};

use fcl_traits::CallLogger;
use crate::call_log_infra::CallLoggerArbiter;
// use super::*;

// mod multithreaded {
    /// #### Examples
    /// ```rs
    /// fcl::set_thread_indent!(String::from("                "));
    /// ```
    #[macro_export]
    macro_rules! set_thread_indent {
        ($expr:expr) => {
            fcl::call_log_infra::instances::THREAD_LOGGER
                .with(|logger| logger.borrow_mut().set_thread_indent($expr))
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
                .with(|logger| logger.borrow_mut().push_logging_is_on($expr))
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
                .with(|logger| logger.borrow_mut().pop_logging_is_on())
        };
    }

    /// #### Examples
    /// ```rs
    /// let on = fcl::logging_is_on!();
    /// ```
    #[macro_export]
    macro_rules! logging_is_on {
        () => {
            fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| logger.borrow().logging_is_on())
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
                .with(|logger| logger.borrow_mut().set_logging_is_on($expr))
        };
    }
// }


pub struct ThreadGatekeeper {
    call_logger_arbiter: Rc<RefCell<CallLoggerArbiter>>,
}
impl ThreadGatekeeper {
    pub fn new(call_logger_arbiter: Rc<RefCell<CallLoggerArbiter>>) -> Self {
        Self {
            call_logger_arbiter,
        }
    }
    pub fn add_thread_logger(&mut self, thread_logger: Box<dyn CallLogger>) {
        self.call_logger_arbiter
            .borrow_mut()
            .add_thread_logger(thread_logger)
    }
    pub fn remove_thread_logger(&mut self) {
        self.call_logger_arbiter.borrow_mut().remove_thread_logger()
    }
}
impl CallLogger for ThreadGatekeeper {
    fn push_logging_is_on(&mut self, is_on: bool) {
        self.call_logger_arbiter
            .borrow_mut()
            .push_logging_is_on(is_on)
    }
    fn pop_logging_is_on(&mut self) {
        self.call_logger_arbiter.borrow_mut().pop_logging_is_on()
    }
    fn logging_is_on(&self) -> bool {
        self.call_logger_arbiter.borrow().logging_is_on()
    }
    fn set_logging_is_on(&mut self, is_on: bool) {
        self.call_logger_arbiter
            .borrow_mut()
            .set_logging_is_on(is_on)
    }
    fn set_thread_indent(&mut self, _thread_indent: String) {
        self.call_logger_arbiter
            .borrow_mut()
            .set_thread_indent(_thread_indent)
    }
    fn log_call(&mut self, name: &str, param_vals: Option<String>) {
        self.call_logger_arbiter
            .borrow_mut()
            .log_call(name, param_vals)
    }
    fn log_ret(&mut self, ret_val: Option<String>) {
        self.call_logger_arbiter.borrow_mut().log_ret(ret_val)
    }
    fn flush(&mut self) {
        self.call_logger_arbiter.borrow_mut().flush()
    }
    fn maybe_flush(&mut self) {
        self.call_logger_arbiter.borrow_mut().maybe_flush()
    }
    fn log_loopbody_start(&mut self) {
        self.call_logger_arbiter.borrow_mut().log_loopbody_start()
    }
    fn log_loopbody_end(&mut self) {
        self.call_logger_arbiter.borrow_mut().log_loopbody_end()
    }
    // fn log_loop_end(&mut self);
    // fn set_loop_ret_val(&mut self, ret_val: String);
}

pub struct ThreadGateAdapter {
    gatekeeper: Arc<Mutex<ThreadGatekeeper>>,
}
impl ThreadGateAdapter {
    pub fn new(gatekeeper: Arc<Mutex<ThreadGatekeeper>>) -> Self {
        Self { gatekeeper }
    }
    fn get_gatekeeper(&self) -> MutexGuard<'_, ThreadGatekeeper> {
        match self.gatekeeper.lock() {
            Ok(guard) => {
                return guard;
            }
            Err(poison_error) => {
                println!(
                    "Internal Error: A poisoned mutex detected (a thread has panicked while holding that mutex): '{:?}'. {}",
                    poison_error, "Trying to recover the mutex."
                );
                return poison_error.into_inner();
            }
        }
    }
}
impl Drop for ThreadGateAdapter {
    fn drop(&mut self) {
        self.get_gatekeeper().remove_thread_logger();
    }
}
impl CallLogger for ThreadGateAdapter {
    fn push_logging_is_on(&mut self, is_on: bool) {
        self.get_gatekeeper().push_logging_is_on(is_on);
    }
    fn pop_logging_is_on(&mut self) {
        self.get_gatekeeper().pop_logging_is_on();
    }
    fn logging_is_on(&self) -> bool {
        self.get_gatekeeper().logging_is_on()
    }
    fn set_logging_is_on(&mut self, is_on: bool) {
        self.get_gatekeeper().set_logging_is_on(is_on)
    }

    fn set_thread_indent(&mut self, thread_indent: String) {
        self.get_gatekeeper().set_thread_indent(thread_indent)
    }

    fn log_call(&mut self, name: &str, param_vals: Option<String>) {
        self.get_gatekeeper().log_call(name, param_vals)
    }
    fn log_ret(&mut self, ret_val: Option<String>) {
        self.get_gatekeeper().log_ret(ret_val)
    }
    fn maybe_flush(&mut self) {
        self.get_gatekeeper().maybe_flush();
    }
    // NOTE: Reuses the trait's `fn flush(&mut self) {}` that does nothing.
    fn log_loopbody_start(&mut self) {
        self.get_gatekeeper().log_loopbody_start()
    }
    fn log_loopbody_end(&mut self) {
        self.get_gatekeeper().log_loopbody_end()
    }
    // fn log_loop_end(&mut self) {
    //     self.get_gatekeeper().log_loop_end()
    // }
    // fn set_loop_ret_val(&mut self, ret_val: String) {
    //     self.get_gatekeeper().set_loop_ret_val(ret_val)
    // }
}
