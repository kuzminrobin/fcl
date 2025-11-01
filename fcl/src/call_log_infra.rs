#[cfg(not(feature = "minimal_writer"))]
use std::cell::LazyCell;
#[cfg(not(feature = "singlethreaded"))]
use std::sync::Arc;
use std::{cell::RefCell, collections::HashMap, io::Write, rc::Rc, sync::LazyLock, thread};

use crate::CallLogger;
use crate::decorators::{LogDecorator, ThreadSpecific};
use code_commons::{CallGraph, CoderunNotifiable};

#[cfg(not(feature = "minimal_writer"))]
use crate::output_sync::StdOutputRedirector;
#[cfg(not(feature = "minimal_writer"))]
use writer::{THREAD_SHARED_WRITER, ThreadSharedWriterPtr, WriterAdapter, WriterKind};

#[cfg(not(feature = "minimal_writer"))]
mod writer;

macro_rules! NO_LOGGER_ERR_STR {
    () => {
        "FCL Internal Error: Unexpected lack of logger"
    };
}

/// Per-thread instance of the call logging infrastructure.
/// 
/// Contians the call graph, logging enabling/diabling mechanism, and some other thread-specific functionality.
pub struct CallLogInfra {
    /// The stack whose top entry tells if logging is anabled or disabled.
    /// If empty then the logging is
    /// {curerntly enabled, TODO: review for endless logging, see "mdBook.md"/"Endless Logging"}.
    /// The loggign can be temporarily enabled or disabled by pushing an entry to this stack,
    /// and the previous state can be recovered by popping an entry.
    logging_is_on: Vec<bool>, // Enabled by default (if empty).
    /// The thread-specific functionality not related to the other parts of the logging infrastructure.
    thread_specifics: Rc<RefCell<dyn ThreadSpecific>>,
    /// The thread's call graph.
    call_graph: CallGraph,
}

impl CallLogInfra {
    /// Creates the new call logging infrastructure instance.
    pub fn new(thread_spec_notifyable: Rc<RefCell<dyn LogDecorator>>) -> Self {
        let coderun_notifiable: Rc<RefCell<dyn CoderunNotifiable>> = thread_spec_notifyable.clone();
        let thread_specifics: Rc<RefCell<dyn ThreadSpecific>> = thread_spec_notifyable;
        Self {
            logging_is_on: Vec::with_capacity(4),
            thread_specifics,
            call_graph: CallGraph::new(coderun_notifiable),
        }
    }
}

impl CallLogger for CallLogInfra {
    fn push_logging_is_on(&mut self, is_on: bool) {
        self.logging_is_on.push(is_on)
    }
    fn pop_logging_is_on(&mut self) {
        self.logging_is_on.pop();
    }
    fn logging_is_on(&self) -> bool {
        *self.logging_is_on.last().unwrap_or(&true) // TODO: Consider making the default a member constant or a macro.
    }
    fn set_logging_is_on(&mut self, is_on: bool) {
        self.logging_is_on.pop();
        self.logging_is_on.push(is_on);
    }

    fn set_thread_indent(&mut self, thread_indent: String) {
        self.thread_specifics
            .borrow_mut()
            .set_thread_indent(thread_indent);
    }

    fn log_call(&mut self, name: &str, param_vals: Option<String>) {
        self.call_graph.add_call(name, param_vals);
    }
    fn log_ret(&mut self, ret_val: Option<String>) {
        self.call_graph.add_ret(ret_val);
    }

    fn flush(&mut self) {
        self.call_graph.flush(true)
    }
    fn maybe_flush(&mut self) {}
    fn log_loopbody_start(&mut self) {
        self.call_graph.add_loopbody_start()
    }
    fn log_loopbody_end(&mut self) {
        self.call_graph.add_loopbody_end()
    }
}

/// The ID used for thread indentation mechanism.
type ThreadIndentId = usize;

/// Container of the resources for the automatic thread indentation mechanism.
///
/// To visually separate the logs of different threads
/// it is reasonable to log different threads in different "columns". That is
/// indent threads' output from the left edge of the log. Such that different threads
/// have different indentation. For example, if there are 2 threads, the `main()` thread
/// (thread 0) can be logged starting with the left-most position of the log lines (0-indent), the other thread
/// can be logged indented to the right by half of the log width (half-the-width indent).
/// If there are 4 threads then it is reasonable if the threads other than `main()`
/// are indented by 1/4, 1/2, and 3/4 of the log width. Or in other words the _thread indent step_ is
/// the quarter of the log width. The good indent step depends on the number of threads.
///
/// The algorithm is not aware of how many threads will be used, that is why
/// it expects the indent step to be provided by the user. If the indent step is not provided
/// then the default is used (TODO: describe the {default specified in a separate file}).
///
/// The algorithm contains the storage of the indents currently used or vacated by the threads.
/// Upon thread spawning the `check_out()` method is used to get the index of the thread
/// in the indent storage and the thread indent to be used for logging this thread.
///
/// Upon thread termination the `check_in()` is used to vacate the indent for the specified thread index.
/// Upon subsequent thread spawning the index and indent will be reused.

// TODO: Examples.
struct ThreadIndents {
    /// The container of the indents used or vacated by the threads.
    /// The vacant indents are reused starting with the lowest index,
    /// proportional to the lowest indent.
    indents_taken: Vec<bool>,
    /// The thread indent step - a string of white-spaces.
    ///
    /// The particular thread's indent is generated by using this thread indent step `index` times, 
    /// where `index` is of the `indents_taken` container.
    thread_indent_step: String,
}
impl ThreadIndents {
    /// Creates a new empty `ThreadIndents`.
    ///
    /// # Parameters
    /// * `thread_indent_step`: User-provided thread indent step.
    fn new(thread_indent_step: Option<String>) -> Self {
        Self {
            indents_taken: vec![false, false, false, false],
            thread_indent_step: thread_indent_step.unwrap_or(String::from(
                "                                                  ",
            )), // 50 spaces // TODO: Consider extracting all the defaults to a separate file.
        }
    }
    /// Generates and returns the thread's indent by using the thread indent step `index` times.
    fn idx_to_string(&self, index: usize) -> String {
        let mut ret_val = String::with_capacity(index * self.thread_indent_step.len());
        for _ in 0..index {
            ret_val.push_str(&self.thread_indent_step)
        }
        ret_val
    }
    /// Returns the tuple
    /// * of the thread's indent ID in the indent storage
    /// * and the thread indent string.
    fn check_out(&mut self) -> (ThreadIndentId, String) {
        for (index, taken) in self.indents_taken.iter().enumerate() {
            if !taken {
                self.indents_taken[index] = true;
                return (index, self.idx_to_string(index));
            }
        }
        self.indents_taken.push(true);
        let index = self.indents_taken.len() - 1;
        return (index, self.idx_to_string(index));
    }
    /// Vacates the thread's indent ID in the indent storage 
    /// (and the corresponding thread indent) for subsequent reuse.
    fn check_in(&mut self, id: ThreadIndentId) {
        self.indents_taken[id] = false;
    }
}

/// The container of the resources necessary for the output synchronization 
/// between 
/// * the FCL on the one hand 
/// * and the user's code's stdoutput (to `stdout` or `stderr`) 
/// and the panic handler's output (to `stderr`) on the other hand.
/// 
/// The output synchronization mechanism is used for flushing the FCL's cache 
/// before the user's code's stdoutput and panic handler output,
/// such that if all the output goes to the same place, the user's (and panic's) output 
/// is shown at the right moment of the function call log.
#[cfg(not(feature = "minimal_writer"))]
struct OutputSync {
    /// Optional pointer to the thread-shared writer.
    /// TODO: Used for ...
    thread_shared_writer: Option<ThreadSharedWriterPtr>,
    /// Optional `stderr` redirector for withholding the user's code's output until the FCL's cache is flushed.
    stderr_redirector: Option<StdOutputRedirector>,
    /// Optional `stdout` redirector for withholding the user's code's and panic handler's output 
    /// until the FCL's cache is flushed.
    stdout_redirector: Option<StdOutputRedirector>,
    /// The `ThreadId` of the `main()` thread. It is used for determining the moment 
    /// when the `main()` thread is panicking, in order to revert the stdoutput redirection.
    main_thread_id: thread::ThreadId,
}

/// The thread arbiter that synchronizes the output to the thread-shared writer by multiple threads.
pub struct CallLoggerArbiter {
    /// Collection of thread loggers and thread indent IDs used by the corresponding threads.
    thread_loggers: HashMap<thread::ThreadId, (Box<dyn CallLogger>, ThreadIndentId)>, // TODO: thread_loggers -> thread_log_info?
    /// The ID of the last thread that was updating its log.
    last_fcl_update_thread: Option<thread::ThreadId>,
    /// Containter of the thread indents by thread indent ID.
    thread_indents: ThreadIndents,

    /// Container of the output synchronization resources.
    #[cfg(not(feature = "minimal_writer"))]
    output_sync: OutputSync,
}

impl CallLoggerArbiter {
    /// Cretaes new `CallLoggerArbiter` instance.
    pub fn new(
        #[cfg(not(feature = "minimal_writer"))] thread_shared_writer: Option<ThreadSharedWriterPtr>,
    ) -> Self {
        return Self {
            thread_loggers: HashMap::new(),
            last_fcl_update_thread: None,
            thread_indents: ThreadIndents::new(None),

            #[cfg(not(feature = "minimal_writer"))]
            output_sync: OutputSync {
                thread_shared_writer,
                stderr_redirector: None,
                stdout_redirector: None,
                main_thread_id: thread::current().id(), // TODO: Provide from outside, otherwise relies on CallLoggerArbiter being created by the main thread only.
            },
        };
    }

    /// For the calling thread 
    /// * adds to the collection of thread loggers 
    /// the thread logger passed as an argument,
    /// * assigns the thread indent to be used by that logger.
    pub fn add_thread_logger(&mut self, mut thread_logger: Box<dyn CallLogger>) {
        let (thread_indent_id, thread_indent) = self.thread_indents.check_out();
        thread_logger.set_thread_indent(thread_indent);
        if self
            .thread_loggers
            .insert(thread::current().id(), (thread_logger, thread_indent_id))
            .is_some()
        {
            // TODO: Consider outputting to stderr, flushing stderr, and terminating 
            // (or returning failure and panicking after unlocking the mutex). 
            // It's not good to panic when the mutex is locked.
            debug_assert!( 
                false,
                "Internal Error: Unexpected repeated thread registration"
            );
        }
    }
    /// For the calling thread 
    /// * flushes and removes the thread logger,
    /// * vacates the thread indent ID.
    /// If all the thread loggers are removed then reverts the stdoutput redirection.
    pub fn remove_thread_logger(&mut self) {
        let current_thread_id = thread::current().id();
        if let Some((_logger, thread_indent_id)) = self.get_thread_logger(current_thread_id) {
            // TODO: Consider `if let &mut Some` or `if let Some(&mut (` to avoid the line below and repeated `if let Some((logger`.
            let thread_indent_id = *thread_indent_id; // Released the mutable borrow.

            // Flush the possible trailing repeat count and std output.
            #[cfg(not(feature = "minimal_writer"))]
            self.sync_fcl_and_std_output(true);
            #[cfg(feature = "minimal_writer")]
            if let Some((logger, ..)) = self.get_thread_logger(thread::current().id()) {
                // TODO: Consider `_logger.flush()` instead of the whole repeated `if let`.
                logger.flush();
            }

            if self.thread_loggers.remove(&current_thread_id).is_none() {
                debug_assert!(false, "Internal Error: Unregistering failed");
            }
            self.thread_indents.check_in(thread_indent_id);
        } // else (no logger) the logger for the current thread is assumed 
        // having been removed in the FCL's panic hook, whereas this function is now called during the 
        // thread-local data destruction in the unwinding panic runtime (after the FCL's panic hook). 

        #[cfg(not(feature = "minimal_writer"))]
        if self.thread_loggers.is_empty() {
            // The last remaining (`main()`) thread has terminated, its thread_local data are being destroyed;
            // or main() has panicked earlier and the last thread has terminated while
            // the panic hook is still running for the main() thread.
            // Flush the buffered std output and do not buffer any more.
            self.output_sync.stderr_redirector = None;
            self.output_sync.stdout_redirector = None;
        }
        if self.last_fcl_update_thread == Some(current_thread_id) {
            self.last_fcl_update_thread = None; // Prevent the subsequent flush attempt for the terminated thread.
        }
    }

    /// Replaces the default panic hook with the `CallLoggerArbiter`'s own.
    #[cfg(not(feature = "minimal_writer"))]
    pub fn set_panic_sync(&mut self) {
        unsafe {
            *(*ORIGINAL_PANIC_HANDLER).borrow_mut() = Some(std::panic::take_hook());
        }
        std::panic::set_hook(Box::new(Self::panic_hook))
    }

    /// Sets up the stdoutput (`stdout` and `stderr`) redirectors for the user's code.
    #[cfg(not(feature = "minimal_writer"))]
    pub fn set_std_output_sync(&mut self) {
        let Some(thread_shared_writer) = self.output_sync.thread_shared_writer.clone() else {
            return;
        };

        // Set stdout and stderr redirection to a corresponding buffer (set std output buffering):
        let writer_kind = thread_shared_writer.borrow().get_writer_kind();
        self.output_sync.stderr_redirector =
            Self::set_stdx_sync(writer_kind, StdOutputRedirector::new_stderr());
        self.output_sync.stdout_redirector =
            Self::set_stdx_sync(writer_kind, StdOutputRedirector::new_stdout());

        // Recover {FCL's own logging directly to the original stdout (or stderr)}
        // while still buffering the user program's {stdout and stderr} output.
        // Get the original std writer:
        let get_original_writer_result = if writer_kind == WriterKind::Stderr {
            self.output_sync
                .stderr_redirector
                .as_ref()
                .map(|redirector| redirector.clone_original_writer())
        } else if writer_kind == WriterKind::Stdout {
            self.output_sync
                .stdout_redirector
                .as_ref()
                .map(|redirector| redirector.clone_original_writer())
        } else {
            // FCL is outputing to a non-std stream (file, socket, pipe, etc.).
            // Nothing to recover.
            None
        };
        // Tell Thread Shared Writer to write the FCL's own output to the original std writer:
        get_original_writer_result.map(|result| match result {
            Ok(file) => thread_shared_writer.borrow_mut().set_writer(file),
            Err(e) => {
                // Something is wrong with std{out|err}, log the error to the opposite stream (std{err|out}):
                let report_stream: &mut dyn std::io::Write = if writer_kind == WriterKind::Stderr {
                    &mut std::io::stdout()
                } else {
                    &mut std::io::stderr()
                };
                let _ignore_another_error = writeln!(
                    report_stream,
                    "Warning: Failed to sync FCL and {:?} output: '{}'",
                    writer_kind, e
                );
                // TODO: Consider returning `false` [and an error message] in addition to 
                // (or instead of) logging the error and moving forward.
            }
        });
    }

    /// The `CallLoggerArbiter`'s own panic hook.
    /// 
    /// The hook tries to borrow the `CALL_LOGGER_ARBITER`, 
    /// if successful (i.e. the panic likely happened in the user's code) 
    /// then the hook does the following { 
    /// * flushes 
    ///   * the FCL's cache for the panicking thread 
    ///   * and the redirected stdoutput, if any;
    /// * removes the thread logger for the panicking thread;
    /// * if it is the `main()` thread that is panicking, then reverts the stdoutput redirection.
    /// 
    /// }
    /// 
    /// else (the `CALL_LOGGER_ARBITER` is borrowed, i.e. the panic could happen in the FCL itself)
    /// the hook tries to borrow the `THREAD_SHARED_WRITER` directly. 
    /// If successful then logs the panic info to the `THREAD_SHARED_WRITER`,  
    /// else logs the panic info to the `stdout` and `stderr` even though they both a likely 
    /// redirected to a buffer that will hardly be flushed.
    /// 
    /// Then the hook calls the default panic hook that 
    /// * logs the panic to the `stderr`
    /// * and invokes the (aborting or unwinding) panic runtime.
    #[cfg(not(feature = "minimal_writer"))]
    fn panic_hook(panic_hook_info: &std::panic::PanicHookInfo<'_>) {
        unsafe {
            let mut arbiter = None;
            match (*CALL_LOGGER_ARBITER).try_borrow_mut() {
                Ok(_arbiter) => arbiter = Some(_arbiter),
                Err(_e) => {
                    const SYNC_MSG: &str = &"FCL failed to synchronize its cache and buffers with the panic report below";
                    const DEBUGGER_MSG: &str = &"If the panic report is not shown, attach the debugger to see the panic details";
                    match (*THREAD_SHARED_WRITER).try_borrow_mut() {
                        Ok(mut writer) => {
                            let _ignore_write_error = writeln!(
                                writer,
                                "{} '{}'.\n{}. {}.",
                                "While FCL was busy (arbiter borrowed) one of the threads has panicked:",
                                panic_hook_info,
                                SYNC_MSG,
                                DEBUGGER_MSG
                            );
                        }
                        Err(_e) => {
                            const DOUBLE_BUSY_MSG: &str = "While FCL was busy (arbiter and writer borrowed) one of the threads has panicked:";
                            let msg = format!(
                                "{} '{}'.\n{}. {}.",
                                DOUBLE_BUSY_MSG, panic_hook_info, SYNC_MSG, DEBUGGER_MSG
                            );
                            let stdout_msg = format!("(stdout) {}", &msg);
                            println!("{}", &stdout_msg);

                            let stderr_msg = format!("(stderr) {}", &msg);
                            eprintln!("{}", &stderr_msg);
                        }
                    }
                }
            }
            if let Some(mut arbiter) = arbiter {
                // Flush the panicking thread's FCL cache and stdoutput redirected to the buffers:
                arbiter.sync_fcl_and_std_output(true);
                // Remove the panicking thread's logger to release the thread's heap data
                // and to prevent subsequent returns logging in case of the unwinding panic runtime 
                // (see details below for the default panic hook).
                arbiter.remove_thread_logger();
                // If the main() thread is panicking:
                if thread::current().id() == arbiter.output_sync.main_thread_id {
                    // Stop buffering the std output so that the default panic hook below 
                    // logs the panic data directly to `stderr` as usually, since 
                    // there will be no more calls and returns logging and stdoutput buffers flushing.
                    arbiter.output_sync.stderr_redirector = None;
                    arbiter.output_sync.stdout_redirector = None;
                } // Otherwise (some other thread is panicking), the default panic hook below
                // will log the panic to the `stderr` redirected to a buffer. And upon subsequent 
                // call or return logging by the other thread that buffer will be flushed to the FCL's log.
            }
            // Call the default panic hook that 
            // * will log the panic to the `stderr` 
            // * and invoke the (aborting or unwinding) panic runtime.
            // In case of an unwinding panic runtime the destructors of the panicking thread will be executed,
            // including those of the `FunctionLogger` and `LoopbodyLogger` instances, that will try to 
            // log the function/closure returns and loop body ends 
            // but will fail since the thread loggers are removed above.
            (*ORIGINAL_PANIC_HANDLER)
                .borrow()
                .as_ref()
                .map(|handler| handler(panic_hook_info));
        }
    }

    #[cfg(not(feature = "minimal_writer"))]
    fn set_stdx_sync(
        writer_kind: WriterKind,
        stdx_redirector_result: std::io::Result<StdOutputRedirector>,
    ) -> Option<StdOutputRedirector> {
        let stderr_redirector = match stdx_redirector_result {
            Err(e) => {
                eprintln!(
                    "Warning: Failed to sync FCL and {:?} output: '{}'",
                    writer_kind, e
                );
                None
            }
            Ok(redirector) => Some(redirector),
        };
        stderr_redirector
    }

    fn get_thread_logger(
        &mut self,
        thread_id: thread::ThreadId,
    ) -> Option<&mut (Box<dyn CallLogger>, usize)> {
        self.thread_loggers.get_mut(&thread_id)
    }
    #[cfg(not(feature = "minimal_writer"))]
    fn sync_fcl_and_std_output(&mut self, full_flush: bool) {
        // {Previuous thread}'s activity, if any, ended with
        // * either FCL updates (cached or flushed), in which case there's no buffered std output,
        // * or buffered std output.
        // The cached FCL updates or buffered std output need to be flushed, in any order.

        // If there was an earlier FCL update
        if let Some(last_fcl_update_thread) = self.last_fcl_update_thread {
            // by a different thread
            if thread::current().id() != last_fcl_update_thread {
                // Flush the previous thread's cached FCL updates, if any:
                if let Some((logger, ..)) = self.get_thread_logger(last_fcl_update_thread) {
                    logger.flush();
                } else {
                    debug_assert!(false, NO_LOGGER_ERR_STR!());
                }

                // Flush the previous (and current) thread's buffered std output, if any:
                if let Some(redirector) = &mut self.output_sync.stderr_redirector {
                    redirector.flush()
                }
                if let Some(redirector) = &mut self.output_sync.stdout_redirector {
                    redirector.flush()
                }
                // The pregvious thread's activity is fully flushed.
            } else {
                // The previous FCL update was done by the current thread.
                // If that FCL update was the last thing then there is no buffered std output,
                //   and no any flush should happen.
                // Else (the std output was buffered after the last (potentially cached) FCL update) {
                //   Flush the cached FCL updates, if any,
                //   Flush the buffered std output.
                // }

                // Read stderr buffer:
                let mut stderr_buf_content = String::new();
                if let Some(redirector) = &mut self.output_sync.stderr_redirector {
                    let _ignore_error = Some(
                        redirector
                            .get_buffer_reader()
                            .read_to_string(&mut stderr_buf_content),
                    );
                }

                // Read stdout buffer:
                let mut stdout_buf_content = String::new();
                if let Some(redirector) = &mut self.output_sync.stdout_redirector {
                    // If there's any buffered std output, flush the thread's own FCL updates and the std output:
                    let _ignore_error = Some(
                        redirector
                            .get_buffer_reader()
                            .read_to_string(&mut stdout_buf_content),
                    );
                }

                // If there was any std output or a full flush is in progress, flush the FCL updates:
                if !stderr_buf_content.is_empty() || !stdout_buf_content.is_empty() || full_flush {
                    if let Some((logger, ..)) = self.get_thread_logger(thread::current().id()) {
                        logger.flush();
                    } else {
                        debug_assert!(false, NO_LOGGER_ERR_STR!());
                    }
                }
                // Flush the buffered stderr output (to the original stderr):
                if !stderr_buf_content.is_empty() {
                    if let Some(redirector) = &mut self.output_sync.stderr_redirector {
                        let _ignore_error = redirector
                            .get_original_writer()
                            .write_all(stderr_buf_content.as_bytes());
                        // An error upon redirector flush means that the program (instrumented with this FCL)
                        // has done something with this (redirected) std output handle
                        // (like set another redirection or something).
                        // What else (other than ignoring) can we do with the flush error upon every FCL update?
                        // We don't want to log the error upon every FCL update, do we?
                        // TODO: Add this to documentation.
                    }
                }

                // Flush the buffered stdout output (to the original stdout):
                if !stdout_buf_content.is_empty() {
                    if let Some(redirector) = &mut self.output_sync.stdout_redirector {
                        let _ignore_error = redirector
                            .get_original_writer()
                            .write_all(stdout_buf_content.as_bytes());
                        // An error upon redirector flush means that the program (instrumented with this FCL)
                        // has done something with this (redirected) std output handle
                        // (like set another redirection or something).
                        // What else (other than ignoring) can we do with the flush error upon every FCL update?
                        // We don't want to log the error upon every FCL update, do we?
                        // TODO: Add this to documentation.
                    }
                }

                // Then continue the FCL updates.
            }
        } else {
            // There were no earlier FCL updates since start or enabling
            // (the first-most FCL update is about to happen).
            // If there's any buffered std output by this moment then flush that std output.

            // If redirection is active
            if let Some(redirector) = &mut self.output_sync.stderr_redirector {
                redirector.flush()
            }
            // Else (redirection is inactive, failed to set redirection (and reported an error) earlier)
            //   Do nothing (proceed to the FCL updates).

            // If redirection is active
            if let Some(redirector) = &mut self.output_sync.stdout_redirector {
                redirector.flush()
            }
            // Else (redirection is inactive, failed to set redirection (and reported an error) earlier)
            //   Do nothing (proceed to the FCL updates).
        }
    }
}

impl CallLogger for CallLoggerArbiter {
    fn push_logging_is_on(&mut self, is_on: bool) {
        if let Some((logger, ..)) = self.get_thread_logger(thread::current().id()) {
            logger.push_logging_is_on(is_on);
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        }
    }
    fn pop_logging_is_on(&mut self) {
        if let Some((logger, ..)) = self.get_thread_logger(thread::current().id()) {
            logger.pop_logging_is_on();
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        }
    }
    fn logging_is_on(&self) -> bool {
        if let Some((logger, ..)) = self.thread_loggers.get(&thread::current().id()) {
            return logger.logging_is_on();
        } else {
            panic!(NO_LOGGER_ERR_STR!());
        }
    }
    fn set_logging_is_on(&mut self, is_on: bool) {
        if let Some((logger, ..)) = self.get_thread_logger(thread::current().id()) {
            logger.set_logging_is_on(is_on);
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        }
    }

    fn set_thread_indent(&mut self, thread_indent: String) {
        if let Some((logger, ..)) = self.get_thread_logger(thread::current().id()) {
            logger.set_thread_indent(thread_indent);
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        }
    }

    fn log_call(&mut self, name: &str, param_vals: Option<String>) {
        #[cfg(not(feature = "minimal_writer"))]
        self.sync_fcl_and_std_output(false);

        let current_thread_id = thread::current().id();
        if let Some((logger, ..)) = self.get_thread_logger(current_thread_id) {
            logger.log_call(name, param_vals);
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        }
        self.last_fcl_update_thread = Some(current_thread_id);
    }
    fn log_ret(&mut self, ret_val: Option<String>) {
        let current_thread_id = thread::current().id();

        // Potentially a call (from a `FunctionLogger` destructor) during stack unwinding in the unwinding panic runtime.
        // In that case suppress flushing and {logging that looks like fake returns in the panicking thread}
        // (by removing the thread's logger from the HashMap in the FCL's panic hook
        // and ignoring the absence of the thread's logger in the code below).
        // NOTE: Two `if`s below (instead of one `if let`) are to work around the double exclusive borrow between `logger` and `self`.
        #[cfg(not(feature = "minimal_writer"))]
        if self.get_thread_logger(current_thread_id).is_some() {
            self.sync_fcl_and_std_output(false);
        } // else (no logger) the stack unwinding of the current thread is in progress. Do nothing (suppress flushing).
        if let Some((logger, ..)) = self.get_thread_logger(current_thread_id) {
            logger.log_ret(ret_val);
            self.last_fcl_update_thread = Some(current_thread_id);
        } // else (no logger) the stack unwinding of the current thread is in progress. That is, this function 
        // has been called from a `FunctionLogger` destructor invoked by the unwinding panic runtime in the context
        // of the panicing thread.
        // Do nothing (suppress the fake return logging).
    }
    fn maybe_flush(&mut self) {
        #[cfg(not(feature = "minimal_writer"))]
        self.sync_fcl_and_std_output(false);
    }
    fn flush(&mut self) {
        #[cfg(not(feature = "minimal_writer"))]
        self.sync_fcl_and_std_output(true);
    }
    fn log_loopbody_start(&mut self) {
        #[cfg(not(feature = "minimal_writer"))]
        self.sync_fcl_and_std_output(false);

        let current_thread_id = thread::current().id();
        if let Some((logger, ..)) = self.get_thread_logger(current_thread_id) {
            logger.log_loopbody_start();
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        }
        self.last_fcl_update_thread = Some(current_thread_id);
    }
    fn log_loopbody_end(&mut self) {
        let current_thread_id = thread::current().id();

        // Potentially a call (from a `LoopbodyLogger` destructor) during stack unwinding in the unwinding panic runtime.
        // In that case suppress flushing and {logging that looks like fake loopbody ends in the panicking thread}
        // (by removing the thread's logger from the HashMap in the FCL's panic hook
        // and ignoring the absence of the thread's logger in the code below).
        // NOTE: Two `if`s below (instead of one `if let`) are to work around the double exclusive borrow between `logger` and `self`.
        #[cfg(not(feature = "minimal_writer"))]
        if self.get_thread_logger(current_thread_id).is_some() {
            self.sync_fcl_and_std_output(false);
        } // else (no logger) the stack unwinding of the current thread is in progress. Do nothing (suppress flushing).
        if let Some((logger, ..)) = self.get_thread_logger(current_thread_id) {
            logger.log_loopbody_end();
            self.last_fcl_update_thread = Some(current_thread_id);
        } // else (no logger) the stack unwinding of the current thread is in progress. That is, this function 
        // has been called from a `LoopbodyLogger` destructor invoked by the unwinding panic runtime in the context
        // of the panicing thread.
        // Do nothing (suppress the fake loopbody end logging).
    }
}

// Global data shared by all the threads:
pub static mut CALL_LOGGER_ARBITER: LazyLock<Rc<RefCell<CallLoggerArbiter>>> =
    LazyLock::new(|| {
        Rc::new(RefCell::new({
            #[cfg(not(feature = "minimal_writer"))]
            let arbiter = {
                let mut arbiter =
                    unsafe { CallLoggerArbiter::new(Some((*THREAD_SHARED_WRITER).clone())) };
                arbiter.set_std_output_sync();
                arbiter.set_panic_sync();
                arbiter
            };
            #[cfg(feature = "minimal_writer")]
            let arbiter = CallLoggerArbiter::new();
            arbiter
        }))
    });

#[cfg(not(feature = "minimal_writer"))]
static mut ORIGINAL_PANIC_HANDLER: LazyCell< // TODO: Consider ORIGINAL_PANIC_HANDLER -> DEFAULT_PANIC_HANDLER.
    RefCell<Option<Box<dyn Fn(&std::panic::PanicHookInfo<'_>)>>>,
> = LazyCell::new(|| RefCell::new(None));

#[cfg(feature = "singlethreaded")]
pub mod instances {
    use super::*;
    thread_local! {
        // TODO: Update the chart with this info.
        pub static THREAD_DECORATOR: Rc<RefCell<dyn LogDecorator>> = unsafe {
            #[cfg(not(feature = "minimal_writer"))]
            let writer: Option<Box<dyn Write>> = Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone())));
            #[cfg(feature = "minimal_writer")]
            let writer: Option<Box<dyn Write>> = None;

            std::rc::Rc::new(std::cell::RefCell::new(
                // TODO: Consider making the default decorator type a macro in a separate file of defaults.
                // crate::decorators::TreeLikeDecorator::new(
                //     writer,
                //     None, None, None))))))
                crate::decorators::CodeLikeDecorator::new(
                    writer,
                    None)))
        };

        pub static THREAD_LOGGER: RefCell<Rc<RefCell<CallLoggerArbiter>>> = unsafe {
            let logging_infra = Box::new(CallLogInfra::new(
                THREAD_DECORATOR.with(|decorator| decorator.clone())));
            (*CALL_LOGGER_ARBITER).borrow_mut().add_thread_logger(logging_infra);

            RefCell::new((*CALL_LOGGER_ARBITER).clone())
        };
    }
}

#[cfg(not(feature = "singlethreaded"))]
pub mod instances {
    use super::*;
    use crate::multithreaded::*;
    static mut THREAD_GATEKEEPER: LazyLock<Arc<Mutex<ThreadGatekeeper>>> =
        LazyLock::new(|| unsafe {
            Arc::new(Mutex::new(ThreadGatekeeper::new(
                (*CALL_LOGGER_ARBITER).clone(),
            )))
        });
    thread_local! {
        // TODO: Update the chart with this info.
        pub static THREAD_DECORATOR: Rc<RefCell<dyn LogDecorator>> = unsafe {
            // TODO: Consider making the default decorator type a macro in a separate file of defaults.
            std::rc::Rc::new(std::cell::RefCell::new(
            // crate::decorators::TreeLikeDecorator::new(
            //     Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
            //     None, None, None))))))
            crate::decorators::CodeLikeDecorator::new(
                Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
                None)))
        };

        pub static THREAD_LOGGER: RefCell<Box<dyn CallLogger>> = unsafe {
            let logging_infra = Box::new(CallLogInfra::new(
                THREAD_DECORATOR.with(|decorator| decorator.clone())));
            match (*THREAD_GATEKEEPER).lock() {
                Ok(mut gatekeeper) => gatekeeper.add_thread_logger(logging_infra),
                Err(e) => {
                    macro_rules! MUTEX_LOCK_FAILURE {
                        () => {
                            "FCL Internal Error: Thread failed to lock mutex because another thread panicked while holding that mutex"
                        };
                    }
                    println!("(stdout) {} ({}).", MUTEX_LOCK_FAILURE!(), e);
                    eprintln!("(stderr) {} ({}).", MUTEX_LOCK_FAILURE!(), e);
                    debug_assert!(false, "{} ({}).", MUTEX_LOCK_FAILURE!(), e); // TODO: Consider recovering the poison mutex or panicking.
                }
            }
            RefCell::new(Box::new(ThreadGateAdapter::new((*THREAD_GATEKEEPER).clone())))
        };
    }
}
