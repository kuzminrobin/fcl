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
    /// For the calling thread temporarily switches logging
    /// * on, if the passed argument is `true`, 
    /// * or off, if `false`.
    /// 
    /// In other words pushes to the On/Off Stack a new state specified by the argument.
    fn push_logging_is_on(&mut self, is_on: bool);
    /// Recovers the previous logging on/off state for the calling thread.
    /// 
    /// In other words pops an entry from the On/Off Stack, if it isn't empty.
    fn pop_logging_is_on(&mut self);
    /// Tells if logging for the calling thread is 
    /// * on, by returning `true`,
    /// * or off, by returning `false`.
    /// 
    /// In other words returns a copy of the On/Off Stack top.
    fn logging_is_on(&self) -> bool;
    /// For the calling thread switches logging
    /// * on, if the passed argument is `true`, 
    /// * or off, if `false`.
    /// 
    /// In other words replaces the entry on top of the On/Off Stack.
    fn set_logging_is_on(&mut self, is_on: bool);

    /// Sets the indent for the calling thread's log.
    /// 
    /// Is used to visually separate the logs by different threads. 
    /// For example, if the instrumented user's code has 2 threads, then it makes sense to log 
    /// the spawned thread indented by half of the console width 
    /// to visually separate the spawned thread's log from the `main()` thread's log.
    fn set_thread_indent(&mut self, _thread_indent: String) {}

    /// For the calling thread updates the call graph with a function or closure call 
    /// and potentially logs that call.
    /// # Parameters
    /// * Function or closure name.
    /// * Optional string representation of the parameter names and values.
    fn log_call(&mut self, name: &str, param_vals: Option<String>);

    /// For the calling thread updates the call graph with a function or closure return 
    /// and potentially logs that return.
    /// # Parameters
    /// * Optional string representation of the returned value.
    fn log_ret(&mut self, ret_val: Option<String>);

    /// Unconditionally flushes the data cached in the call graph 
    /// (TODO: likely "of the previous thread", see below).
    /// 
    /// Is called, for example,
    /// upon thread context switch. If the new thread tries to log something then 
    /// the previous thread's log cache is flushed first.
    fn flush(&mut self) {}
    /// Flushes the data cached in the call graph upon certain condition, such as 
    /// the instrumented user's code own output or panic hook output.
    fn maybe_flush(&mut self);

    /// Updates the call graph of the calling thread with a loop body start and potentially logs that start.
    fn log_loopbody_start(&mut self);
    /// Updates the call graph of the calling thread with a loop body end and potentially logs that end.
    fn log_loopbody_end(&mut self);
}

/// A trait to be used for logging the parameters and return values.
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

/// The type for instrumenting a user's function or a closure to be logged.
/// 
/// Its constructor logs the function or closure call, and the destructor logs the return.
/// ### Examples
/// The instrumented user's function and closure:
/// ```
/// #[fcl_proc_macros::loggable] // The procedural macro that adds the instrumentation.
/// fn f() { // The user's function definition.
///     let _c = Some(5).map(
///         |value| true    // The user's closure definition.
///     ); 
/// }
/// ```
/// The result of the macro expansion:
/// ```ignore
/// fn f() {
///     . . .
///     // The instrumentation. 
///     // The instance whose constructor logs the call `f() {`
///     // and destructor logs the return `} // f().`.
///     let mut callee_logger = CalleeLogger::new("f"..);
///     . . .
///     let _c = Some(5).map(
///         |value| {
///             . . .
///             // The instrumentation. 
///             // The instance whose constructor logs the call `f()::closure{4,9:4,20} {`
///             // and destructor logs the return `} // f()::closure{4,9:4,20}.`.
///             let mut callee_logger = fcl::CalleeLogger::new("f()::closure{1,1:1,0}"..);
///             . . .
///             true    
///         }
///     ); 
/// }
/// ```
pub struct CalleeLogger {
    /// The optional string representation of the returned value.
    ret_val_str: Option<String>,
}

impl CalleeLogger {
    /// Creates a new `CalleeLogger` and logs the function/closure's call.
    /// ### Parameters.
    /// * The optional string representation of the user function's parameter names and values.
    pub fn new(func_name: &str, param_vals: Option<String>) -> Self {
        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "singlethreaded")]
            let logger = logger.borrow_mut();

            logger.borrow_mut().log_call(func_name, param_vals);
        });

        Self {
            ret_val_str: None,
        }
    }

    /// Sets a string representation of the value returned by the instrumented user's function/closure.
    pub fn set_ret_val(&mut self, ret_val_str: String) {
        self.ret_val_str = Some(ret_val_str);
    }
}
impl Drop for CalleeLogger {
    /// Logs the function or closure return.
    fn drop(&mut self) {
        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "singlethreaded")]
            let logger = logger.borrow_mut();

            logger.borrow_mut().log_ret(self.ret_val_str.take());
        });
    }
}

/// The type to instrument the user's loop bodies to be logged.
pub struct LoopbodyLogger;

impl LoopbodyLogger {
    /// Creates a new `LoopbodyLogger` and logs the loop body start.
    pub fn new() -> Self {
        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "singlethreaded")]
            let logger = logger.borrow_mut();

            logger.borrow_mut().log_loopbody_start();
        });
        Self
    }
}
impl Drop for LoopbodyLogger {
    /// Logs the loop body end.
    fn drop(&mut self) {
        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "singlethreaded")]
            let logger = logger.borrow_mut();

            logger.borrow_mut().log_loopbody_end();
        });
    }
}

#[cfg(test)]
mod tests_algo_basics;
#[cfg(test)]
mod tests_algo_add_call;