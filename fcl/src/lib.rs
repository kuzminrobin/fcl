#![feature(specialization)]

pub mod call_log_infra;
#[cfg(not(feature = "singlethreaded"))]
pub mod multithreaded;
mod output_sync;
#[cfg(feature = "singlethreaded")]
pub mod singlethreaded;
mod writer;

#[cfg(feature = "singlethreaded")]
use fcl_traits::CallLogger;

use call_log_infra::instances::THREAD_LOGGER;

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
