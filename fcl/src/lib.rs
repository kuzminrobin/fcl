#![feature(specialization)]

pub mod call_log_infra;
pub mod decorators;
#[cfg(not(feature = "singlethreaded"))]
pub mod multithreaded;
mod output_sync;
#[cfg(feature = "singlethreaded")]
pub mod singlethreaded;

use call_log_infra::instances::THREAD_LOGGER;

/// A trait to be implemented by a per-thread call logging instance.
/// 
/// The trait assumes the following.
/// 1. The implementing instance has a logging enabling/disabling mechanism
/// in the form of the On/Off Stack whose top entry tells if logging is enabled.
/// If the On/Off Stack is empty then the logging is
/// {TODO: One of
/// * enabled by default, and manually disabled (or not enabled) for `main()`, the thread functions, 
/// and other long-living  functions (see "Loggign Endlessly" in the mdBook), which is easy (seems the most preferable approach),
/// * a specific entry/macro in the file of the defaults (affecting all the threads?), 
/// * specified from the outside on a per-thread basis during the instance creation?,
/// * disabled by default, then how to enable for main() and the thread functions? (seems the least preferable approach)
/// }.
/// 2. The implementing instance has a notion of a thread indent 
/// used for visual separation of different threads' log output.
/// 3. The implementing instance has a notion of logging the functions/closures, loop bodies, 
/// and flushing the log cache.
pub trait CallLogger {
    /// Temporarily switches logging
    /// * on, if the passed argument is `true`, 
    /// * or off, if `false`.
    /// 
    /// In other words pushes to the On/Off Stack a new state specified by the argument.
    fn push_logging_is_on(&mut self, is_on: bool);
    /// Recovers the previous logging on/off state.
    /// 
    /// In other words pops an entry from the On/Off Stack, if it isn't empty.
    fn pop_logging_is_on(&mut self);
    /// Tells if logging is 
    /// * on, by returning `true`,
    /// * or off, by returning `false`.
    /// 
    /// In other words returns a copy of the On/Off Stack top.
    fn logging_is_on(&self) -> bool;
    /// Switches logging
    /// * on, if the passed argument is `true`, 
    /// * or off, if `false`.
    /// 
    /// In other words replaces the entry on top of the On/Off Stack.
    fn set_logging_is_on(&mut self, is_on: bool);

    /// Sets the indent for the calling thread's log.
    /// 
    /// For example, if the instrumented user's code has 2 threads, then it makes sense to log 
    /// the spawned thread indented by half of the console width 
    /// to visually separate the spawned thread's log from the `main()` thread's log.
    fn set_thread_indent(&mut self, _thread_indent: String) {}

    /// Updates the call graph with a function or closure call and potentially logs that call.
    /// # Parameters
    /// * Function or closure name.
    /// * Optional string representation of the parameter names and values.
    fn log_call(&mut self, name: &str, param_vals: Option<String>);

    /// Updates the call graph with a function or closure return and potentially logs that return.
    /// # Parameters
    /// * Optional string representation of the returned value.
    fn log_ret(&mut self, ret_val: Option<String>);

    /// Unconditionally flushes the data cached in the call graph. 
    /// 
    /// Is called, for example,
    /// upon thread context switch. If the new thread tries to log something then 
    /// the previous thread's log cache is flushed.
    fn flush(&mut self) {}
    /// Flushes the data cached in the call graph upon certain condition, such as 
    /// the instrumented user's code own output or panic handling output.
    fn maybe_flush(&mut self);

    /// Updates the call graph with a loop body start and potentially logs that start.
    fn log_loopbody_start(&mut self);
    /// Updates the call graph with a loop body end and potentially logs that end.
    fn log_loopbody_end(&mut self);
}

/// The trait to be used for logging the parameters and return values.
pub trait MaybePrint {
    /// Returns the string representation of the instance.
    fn maybe_print(&self) -> String;
}
/// The default trait implementation.
impl<T> MaybePrint for T {
    /// Returns a dummy string representation of the instance. 
    default fn maybe_print(&self) -> String {
        String::from("?")
    }
}
/// The trait implementation for the types implementing the `std::fmt::Debug` trait.
impl<T: std::fmt::Debug> MaybePrint for T {
    /// Returns the debug string representation of the instance.
    fn maybe_print(&self) -> String {
        format!("{:?}", self)
    }
}

/// The common part of the instances used for instrumenting the user's code.
struct LoggerCommon {
    /// Tells if the function/closure call or loop body start has been logged.
    /// For example, during the user's function call the logging was enabled.
    call_logged: bool, // TODO: Consider -> is_logged or has_been_logged.
}
/// The type to instrument a user's function or a closure to be logged.
pub struct FunctionLogger {
    /// The common part.
    common: LoggerCommon,
    /// The optional string representation of the returned value.
    ret_val_str: Option<String>,
}

impl FunctionLogger {
    /// Creates a new instance to log the function/closure's call and return.
    /// # Parameters.
    /// * The optional string representation of the user's function's parameters and their values.
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

    /// Sets a string representation of the value returned by the instrumented user's function/closure.
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