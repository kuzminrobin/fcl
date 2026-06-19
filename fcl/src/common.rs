pub mod call_log_infra;
pub mod decorators;
#[cfg(feature = "multithreaded")]
pub mod multithreaded;
#[cfg(feature = "std_output_sync")]
mod output_sync;
#[cfg(feature = "single_threaded")]
pub mod singlethreaded;

use call_log_infra::instances::THREAD_LOGGER;

// TODO: 
//  Why are those the macros rather than the functions? Consider making all of those the functions 
//      defined depending on a feature "single_threaded", "multithreaded".
//      Functions are less likely to be optimized out.
//      Actually if all the user-facing macros are made functions, then the `extra_borrow[_mut]` ones can become private.
//  Make sure the `extra_borrow[_mut]` macros are not visible to the user. Not applicable since used by user-visible macros like `logging_is_on!()`.

#[cfg(feature = "single_threaded")]
#[macro_export]
macro_rules! extra_borrow {     
    ($logger:expr) => {
        $logger.borrow()
    }
}
#[cfg(feature = "multithreaded")]
#[macro_export]
macro_rules! extra_borrow {
    ($logger:expr) => { $logger };
}

#[cfg(feature = "single_threaded")]
#[macro_export]
macro_rules! extra_borrow_mut {
    ($logger:expr) => {
        $logger.borrow_mut()
    }
}
#[cfg(feature = "multithreaded")]
#[macro_export]
macro_rules! extra_borrow_mut {
    ($logger:expr) => { $logger };
}

/// Sets a specific thread indent different from the default for the invoking thread.
/// #### Examples
/// ```rs
/// fcl::set_thread_indent!(String::from("                "));
/// ```
#[macro_export]
macro_rules! set_thread_indent {
    ($expr:expr) => {
        fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger| {   // TODO: Consider removing `::call_log_infra::instances` for the pub entities (like `THREAD_LOGGER`) accessible from the user code.
            fcl::extra_borrow_mut!();    // TODO: Consider not exposing `extra_borrow[_mut]!()` to the user. // TODO: Test
            logger
                .borrow_mut()
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
        fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
            use fcl::common::CallLogger;
            let logger = fcl::extra_borrow_mut!(logger);
            logger.borrow_mut().push_logging_is_on($expr)
        })
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
        fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
            use fcl::common::CallLogger;
            let logger = fcl::extra_borrow_mut!(logger); // TODO: Test.
            logger.borrow_mut().pop_logging_is_on()
        })
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
        fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
            use fcl::common::CallLogger;
            let logger = fcl::extra_borrow!(logger); // TODO: Test.
            logger.borrow().logging_is_on()
        })
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
        fcl::common::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
            use fcl::common::CallLogger;
            let logger = fcl::extra_borrow_mut!(logger); // TODO: Test.
            logger.borrow_mut().set_logging_is_on($expr)
        })
    };
}

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
    fn log_call(&mut self, name: &str, 
        #[cfg(feature = "params_logging")]
        param_vals: Option<String>
    );

    /// For the calling thread updates the call graph with a function or closure return
    /// and potentially logs that return.
    /// # Parameters
    /// * Optional string representation of the returned value.
    fn log_ret(&mut self,
        #[cfg(feature = "ret_val_logging")]
        ret_val: Option<String>
    );

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
    /// Updates the call graph of the calling thread with a loop end, e.g. to avoid treating the adjacent loops as one loop.
    fn log_loop_end(&mut self);
}

#[cfg(any(feature = "ret_val_logging", feature = "params_logging"))]
/// A trait to be used for logging the parameters and return values.
pub trait MaybePrint {
    /// Returns the string representation of the instance.
    fn maybe_print(&self) -> String;
}

#[cfg(any(feature = "ret_val_logging", feature = "params_logging"))]
/// The default trait implementation.
/// 
/// NOTE: Requires `#![feature(specialization)]`.
impl<T> MaybePrint for T {
    /// Returns a dummy string representation of the instance.
    default fn maybe_print(&self) -> String {
        String::from("?")
    }
}

#[cfg(any(feature = "ret_val_logging", feature = "params_logging"))]
/// The trait implementation for the types implementing the `std::fmt::Debug` trait.
/// 
/// NOTE: Requires `#![feature(specialization)]`.
impl<T: std::fmt::Debug> MaybePrint for T {
    /// Returns the debug string representation of the instance.
    fn maybe_print(&self) -> String {
        format!("{:?}", self)
    }
}
// TODO: The `MaybePrint` trait implementation for the types implementing the `Display` trait.

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
    pub fn new(func_name: &str, 
        #[cfg(feature = "params_logging")]
        param_vals: Option<String>
    ) -> Self {
        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "single_threaded")]
            let logger = logger.borrow_mut();

            logger.borrow_mut().log_call(func_name, 
                #[cfg(feature = "params_logging")]
                param_vals
            );
        });

        Self { ret_val_str: None }
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
            #[cfg(feature = "single_threaded")]
            let logger = logger.borrow_mut();

            logger.borrow_mut().log_ret(
                #[cfg(feature = "ret_val_logging")]
                self.ret_val_str.take()
            );
        });
    }
}

/// The type to instrument the user's loop bodies to be logged.
pub struct LoopbodyLogger;

impl LoopbodyLogger {
    /// Creates a new `LoopbodyLogger` and logs the loop body start.
    pub fn new() -> Self {
        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "single_threaded")]
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
            #[cfg(feature = "single_threaded")]
            let logger = logger.borrow_mut();

            logger.borrow_mut().log_loopbody_end();
        });
    }
}
