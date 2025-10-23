#![feature(specialization)]

pub mod call_log_infra;
pub mod decorators;
#[cfg(not(feature = "singlethreaded"))]
pub mod multithreaded;
mod output_sync;
#[cfg(feature = "singlethreaded")]
pub mod singlethreaded;

use call_log_infra::instances::THREAD_LOGGER;

pub trait CallLogger {
    fn push_logging_is_on(&mut self, is_on: bool);
    fn pop_logging_is_on(&mut self);
    fn logging_is_on(&self) -> bool;
    fn set_logging_is_on(&mut self, is_on: bool);

    fn set_thread_indent(&mut self, _thread_indent: String) {}

    fn log_call(&mut self, name: &str, param_vals: Option<String>);
    fn log_ret(&mut self, ret_val: Option<String>);
    fn flush(&mut self) {}
    fn maybe_flush(&mut self); // TODO: Whats' the diff from `flush()`?
    fn log_loopbody_start(&mut self);
    fn log_loopbody_end(&mut self);
}

pub trait MaybePrint {
    fn maybe_print(&self) -> String;
}
impl<T> MaybePrint for T {
    default fn maybe_print(&self) -> String {
        String::from("?")
    }
}

impl<T: std::fmt::Debug> MaybePrint for T {
    fn maybe_print(&self) -> String {
        format!("{:?}", self)
    }
}

struct LoggerCommon {
    call_logged: bool,
}
pub struct FunctionLogger {
    common: LoggerCommon,
    ret_val_str: Option<String>,
}

impl FunctionLogger {
    pub fn new(func_name: &str, param_vals: Option<String>) -> Self {
        let mut call_logged = false;

        THREAD_LOGGER.with(|logger| {
            // TODO: Consider 
            // let mut logger_borrow = logger.borrow_mut();
            // #[cfg(feature = "singlethreaded")]
            // let mut logger_borrow = logger_borrow.borrow_mut();
            #[cfg(feature = "singlethreaded")] 
            let intermediate_borrow = logger.borrow_mut();
            #[cfg(feature = "singlethreaded")]
            let mut logger_borrow = intermediate_borrow.borrow_mut();

            #[cfg(not(feature = "singlethreaded"))]
            let mut logger_borrow = logger.borrow_mut();

            if logger_borrow.logging_is_on() {
                logger_borrow.log_call(func_name, param_vals);
                call_logged = true;
            }
        });

        Self {
            common: LoggerCommon { call_logged },
            ret_val_str: None,
        }
    }
    pub fn set_ret_val(&mut self, ret_val_str: String) {
        self.ret_val_str = Some(ret_val_str);
    }
}
impl Drop for FunctionLogger {
    fn drop(&mut self) {
        if self.common.call_logged {
            THREAD_LOGGER.with(|logger| {
                #[cfg(feature = "singlethreaded")]
                let intermediate_borrow = logger.borrow_mut();
                #[cfg(feature = "singlethreaded")]
                let mut logger_borrow = intermediate_borrow.borrow_mut();

                #[cfg(not(feature = "singlethreaded"))]
                let mut logger_borrow = logger.borrow_mut();

                logger_borrow.log_ret(self.ret_val_str.take());
            });
        }
    }
}

pub struct LoopbodyLogger {
    common: LoggerCommon,
}

impl LoopbodyLogger {
    pub fn new() -> Self {
        let mut call_logged = false;
        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "singlethreaded")]
            let intermediate_borrow = logger.borrow_mut();
            #[cfg(feature = "singlethreaded")]
            let mut logger_borrow = intermediate_borrow.borrow_mut();

            #[cfg(not(feature = "singlethreaded"))]
            let mut logger_borrow = logger.borrow_mut();

            if logger_borrow.logging_is_on() {
                logger_borrow.log_loopbody_start();
                call_logged = true;
            }
        });
        Self {
            common: LoggerCommon { call_logged },
        }
    }
}
impl Drop for LoopbodyLogger {
    fn drop(&mut self) {
        if self.common.call_logged {
            THREAD_LOGGER.with(|logger| {
                #[cfg(feature = "singlethreaded")]
                let intermediate_borrow = logger.borrow_mut();
                #[cfg(feature = "singlethreaded")]
                let mut logger_borrow = intermediate_borrow.borrow_mut();
                #[cfg(not(feature = "singlethreaded"))]
                let mut logger_borrow = logger.borrow_mut();

                logger_borrow.log_loopbody_end();
            });
        }
    }
}

#[cfg(test)]
mod tests;