use code_commons::{CallGraph, CoderunNotifiable};
use fcl_traits::{CallLogger, CoderunThreadSpecificNotifyable, ThreadSpecifics};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, LazyLock, Mutex, MutexGuard},
    thread,
};

use crate::{
    output_sync::StdOutputRedirector,
    writer::{ThreadSharedWriter, ThreadSharedWriterPtr, WriterAdapter, WriterKind},
};

macro_rules! NO_LOGGER_ERR_STR {
    () => { "Internal Error: Unexpected lack of logger" }
}

pub struct CallLogInfra {
    logging_is_on: Vec<bool>, // Enabled by default (if empty). // TODO: Test.
    thread_specifics: Rc<RefCell<dyn ThreadSpecifics>>,
    call_graph: CallGraph,
}

impl CallLogInfra {
    pub fn new(thread_spec_notifyable: Rc<RefCell<dyn CoderunThreadSpecificNotifyable>>) -> Self {
        // NOTE: Curious trick. // TODO: Document it.
        let coderun_notifiable: Rc<RefCell<dyn CoderunNotifiable>> = thread_spec_notifyable.clone();
        let thread_specifics: Rc<RefCell<dyn ThreadSpecifics>> = thread_spec_notifyable;
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
        *self.logging_is_on.last().unwrap_or(&true)
    }
    fn set_logging_is_on(&mut self, is_on: bool) {
        self.logging_is_on.pop();
        self.logging_is_on.push(is_on);
    }

    fn set_thread_indent(&mut self, thread_indent: &'static str) {
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

    // TODO: Consider making this impl conditional, for multithreaded case only.
    fn flush(&mut self) {
        self.call_graph.flush()
    }
    fn maybe_flush(&mut self) {}
    fn log_loopbody_start(&mut self) {
        self.call_graph.add_loopbody_start()
    }
    fn log_loopbody_end(&mut self) {
        self.call_graph.add_loopbody_end()
    }

}

pub struct CallLoggerArbiter {
    thread_shared_writer: ThreadSharedWriterPtr, // = Arc<RefCell<ThreadSharedWriter>>
    thread_loggers: HashMap<thread::ThreadId, Box<dyn CallLogger>>,
    last_fcl_update_thread: Option<thread::ThreadId>,
    stderr_redirector: Option<StdOutputRedirector>,
    stdout_redirector: Option<StdOutputRedirector>,
    main_thread_id: thread::ThreadId,
}

impl CallLoggerArbiter {
    pub fn new(thread_shared_writer: ThreadSharedWriterPtr) -> Self {
        return Self {
            thread_shared_writer,
            thread_loggers: HashMap::new(),
            last_fcl_update_thread: None,
            stderr_redirector: None,
            stdout_redirector: None,
            main_thread_id: thread::current().id(),
        };
    }

    fn panic_hook(panic_hook_info: &std::panic::PanicHookInfo<'_>) {
        unsafe {
            match (*CALL_LOGGER_ARBITER).lock() {
                Ok(mut guard) => {
                    guard.sync_fcl_and_std_output(true);
                    guard.remove_thread_logger();
                    if thread::current().id() == guard.main_thread_id {
                        // The main() thread is panicking.
                        // Lower down the probability of unclear sporadic freezing (deadlock?),
                        // by flushing the buffered std output and not buffering any more.
                        guard.stderr_redirector = None; 
                        guard.stdout_redirector = None;
                    }

                    // TODO: In a single-threaded case cancel the std output buffering.
                }
                Err(_e) => {
                    println!("Internal Error: Unexpected mutex lock failure in panic_hook(): '{:?}'", _e);
                }
            }
            (*ORIGINAL_PANIC_HOOK).borrow().as_ref().map(|hook| hook(panic_hook_info));
        }
    }

    pub fn set_panic_sync(&mut self) {
        unsafe { *(*ORIGINAL_PANIC_HOOK).borrow_mut() = Some(std::panic::take_hook()); }
        std::panic::set_hook(Box::new(Self::panic_hook))
    }

    pub fn set_stdx_sync(
        &mut self,
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
            Ok(redirector) => {
                Some(redirector)
            }
        };
        stderr_redirector
    }

    pub fn set_std_output_sync(&mut self) {
        // Set stdout and stderr redirection to a corresponding buffer (set std output buffering):
        let writer_kind = self.thread_shared_writer.borrow().get_writer_kind();
        self.stderr_redirector = self.set_stdx_sync(writer_kind, StdOutputRedirector::new_stderr());
        self.stdout_redirector = self.set_stdx_sync(writer_kind, StdOutputRedirector::new_stdout());

        // Recover {FCL's own logging directly to the original stdout (or stderr)}
        // while still buffering the program's {stdout and stderr} output.
        // Get the original std writer:
        let get_original_writer_result = if writer_kind == WriterKind::Stderr {
            self.stderr_redirector
                .as_ref()
                .map(|redirector| redirector.clone_original_writer())
        } else if writer_kind == WriterKind::Stdout {
            self.stdout_redirector
                .as_ref()
                .map(|redirector| redirector.clone_original_writer())
        } else {
            // FCL is outputing to a non-std stream (file, socket, pipe, etc.).
            // Nothing to recover.
            None
        };
        // Tell Thread Shared Writer to write the FCL's own output to the original std writer:
        get_original_writer_result.map(|result| match result {
            Ok(file) => self.thread_shared_writer.borrow_mut().set_writer(file),
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
                    writer_kind,
                    e
                );
            }
        });
    }
    pub fn add_thread_logger(&mut self, thread_logger: Box<dyn CallLogger>) {
        if self
            .thread_loggers
            .insert(thread::current().id(), thread_logger)
            .is_some()
        {
            debug_assert!(
                false,
                "Internal error suspected: Unexpected repeated thread registration"
            );
        }
    }
    pub fn remove_thread_logger(&mut self) {
        let current_thread_id = thread::current().id();
        if let Some(_logger) = self.get_thread_logger(current_thread_id) {
            // Flush the possible trailing repeat count and std output.
            self.sync_fcl_and_std_output(true); 

            if self.thread_loggers.remove(&current_thread_id).is_none() {
                // TODO: Switch to debug_assert!() when freezing is clarified.
                println!("remove_thread_logger debug_assert"); 
                // debug_assert!(
                //     false,
                //     "Internal Error: Unregistering failed"
                // );
            }
        } // else (no logger) the logger for the current thread is assumed removed in the panic handler.

        if self.thread_loggers.is_empty() {
            // The last remaining main() thread has terminated, its thread_local data are being destroyed;
            // or main() has panicked earlier and the last thread has terminated while 
            // the panic hook is still running for the main() thread.
            // Flush the buffered std output and do not buffer any more.
            self.stderr_redirector = None; 
            self.stdout_redirector = None;
        }
        if self.last_fcl_update_thread == Some(current_thread_id) {
            self.last_fcl_update_thread = None; // Prevent the subsequent flush attempt for the terminated thread.
        }
    }
    fn get_thread_logger(&mut self, thread_id: thread::ThreadId) -> Option<&mut Box<dyn CallLogger>> {
        self.thread_loggers.get_mut(&thread_id)
    }
    // fn get_thread_logger(&mut self, thread_id: thread::ThreadId) -> &mut Box<dyn CallLogger> {
    //     if let Some(logger) = self.thread_loggers.get_mut(&thread_id) {
    //         return logger;
    //     } else {
    //         panic!("Internal error: Logging by unregistered thread");
    //     }
    // }
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
                if let Some(logger) = self.get_thread_logger(last_fcl_update_thread) {
                    logger.flush();
                } else {
                    debug_assert!(false, NO_LOGGER_ERR_STR!());
                } 

                // Flush the previous (and current) thread's buffered std output, if any:
                if let Some(redirector) = &mut self.stderr_redirector {
                    redirector.flush()
                }
                if let Some(redirector) = &mut self.stdout_redirector {
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
                if let Some(redirector) = &mut self.stderr_redirector {
                    let _ignore_error = Some(redirector
                        .get_buffer_reader()
                        .read_to_string(&mut stderr_buf_content));
                }

                // Read stdout buffer:
                let mut stdout_buf_content = String::new();
                if let Some(redirector) = &mut self.stdout_redirector {
                    // If there's any buffered std output, flush the thread's own FCL updates and the std output:
                    let _ignore_error = Some(redirector
                        .get_buffer_reader()
                        .read_to_string(&mut stdout_buf_content));
                }

                // If there was any std output or a full flush is in progress, flush the FCL updates:
                if !stderr_buf_content.is_empty() || !stdout_buf_content.is_empty() 
                   || full_flush
                {
                    if let Some(logger) = self.get_thread_logger(thread::current().id()) {
                        logger.flush();
                    } else {
                        debug_assert!(false, NO_LOGGER_ERR_STR!());
                    } 
                }
                // Flush the buffered stderr output (to the original stderr):
                if !stderr_buf_content.is_empty() {
                    if let Some(redirector) = &mut self.stderr_redirector {
                        let _ignore_error = redirector
                            .get_original_writer()
                            .write_all(stderr_buf_content.as_bytes());
                        // An error upon redirector flush means that the program (instrumented with this FCL)
                        // has done something with this (redirected) std output handle
                        // (like set another redirection or something).
                        // What else (other than ignoring) can we do with the flush error upon every FCL update?
                        // We don't want to log the error upon every FCL update, do we?
                    }
                }

                // Flush the buffered stdout output (to the original stdout):
                if !stdout_buf_content.is_empty() {
                    if let Some(redirector) = &mut self.stdout_redirector {
                        let _ignore_error = redirector
                            .get_original_writer()
                            .write_all(stdout_buf_content.as_bytes());
                        // An error upon redirector flush means that the program (instrumented with this FCL)
                        // has done something with this (redirected) std output handle
                        // (like set another redirection or something).
                        // What else (other than ignoring) can we do with the flush error upon every FCL update?
                        // We don't want to log the error upon every FCL update, do we?
                    }
                }

                // Then continue the FCL updates.
            }
        } else {
            // There were no earlier FCL updates since start or enabling
            // (the first-most FCL update is about to happen).
            // If there's any buffered std output by this moment then flush that std output.

            // If redirection is active
            if let Some(redirector) = &mut self.stderr_redirector {
                redirector.flush()
            }
            // Else (redirection is inactive, failed to set redirection (and reported an error) earlier)
            //   Do nothing (proceed to the FCL updates).

            // If redirection is active
            if let Some(redirector) = &mut self.stdout_redirector {
                redirector.flush()
            }
            // Else (redirection is inactive, failed to set redirection (and reported an error) earlier)
            //   Do nothing (proceed to the FCL updates).
        }
    }
}

impl CallLogger for CallLoggerArbiter {
    fn push_logging_is_on(&mut self, is_on: bool) {
        if let Some(logger) = self.get_thread_logger(thread::current().id()) {
            logger.push_logging_is_on(is_on);
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        } 
    }
    fn pop_logging_is_on(&mut self) {
        if let Some(logger) = self.get_thread_logger(thread::current().id()) {
            logger.pop_logging_is_on();
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        } 
    }
    fn logging_is_on(&self) -> bool {
        if let Some(logger) = self.thread_loggers.get(&thread::current().id()) {
            return logger.logging_is_on();
        } else {
            println!("logging_is_on() panic");  // TODO: Remove when freezing is clarified.
            panic!(NO_LOGGER_ERR_STR!());
        }
    }
    fn set_logging_is_on(&mut self, is_on: bool) {
        if let Some(logger) = self.get_thread_logger(thread::current().id()) {
            logger.set_logging_is_on(is_on);
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        } 
    }

    fn set_thread_indent(&mut self, thread_indent: &'static str) {
        if let Some(logger) = self.get_thread_logger(thread::current().id()) {
            logger.set_thread_indent(thread_indent);
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        } 
    }

    fn log_call(&mut self, name: &str, param_vals: Option<String>) {
        self.sync_fcl_and_std_output(false);

        let current_thread_id = thread::current().id();
        if let Some(logger) = self.get_thread_logger(current_thread_id) {
            logger.log_call(name, param_vals);
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        } 
        self.last_fcl_update_thread = Some(current_thread_id);
    }
    fn log_ret(&mut self, ret_val: Option<String>) {
        let current_thread_id = thread::current().id();

        // TODO: Come up with a more strict terminology regarding the "regular panic handler" as opposed to "(FCL's panic hook)".

        // Potentially a call (from a `FunctionLogger` destructor) during stack unwinding in the regular panic handler. 
        // In that case suppress flushing and {logging the fake returns in the panicking thread}
        // (by removing the thread's logger from the HashMap in the FCL's panic hook 
        // and ignoring the absence of the thread's logger in the code below). 
        // NOTE: Two `if`s below (instead of one `if let`) are to work around the double exclusive borrow between `logger` and `self`.
        if self.get_thread_logger(current_thread_id).is_some() {
            self.sync_fcl_and_std_output(false);
        } // else (no logger) the stack unwinding of the current thread is in progress. Do nothing (suppress flushing).
        if let Some(logger) = self.get_thread_logger(current_thread_id) {
            logger.log_ret(ret_val);
            self.last_fcl_update_thread = Some(current_thread_id);
        } // else (no logger) the stack unwinding of the current thread is in progress. Do nothing (suppress 
        // the fake return logging).
    }
    fn maybe_flush(&mut self) {
        self.sync_fcl_and_std_output(false);
    }
    // NOTE: Reuses the trait's `fn flush(&mut self) {}` that does nothing.
    fn log_loopbody_start(&mut self) {
        self.sync_fcl_and_std_output(false);

        let current_thread_id = thread::current().id();
        if let Some(logger) = self.get_thread_logger(current_thread_id) {
            logger.log_loopbody_start();
        } else {
            debug_assert!(false, NO_LOGGER_ERR_STR!());
        } 
        self.last_fcl_update_thread = Some(current_thread_id);

    }
    fn log_loopbody_end(&mut self) {
        let current_thread_id = thread::current().id();

        // Potentially a call (from a `LoopbodyLogger` destructor) during stack unwinding in the regular panic handler.
        // In that case suppress flushing and {logging the fake loopbody ends in the panicking thread}
        // (by removing the thread's logger from the HashMap in the FCL's panic hook 
        // and ignoring the absence of the thread's logger in the code below). 
        // NOTE: Two `if`s below (instead of one `if let`) are to work around the double exclusive borrow between `logger` and `self`.
        if self.get_thread_logger(current_thread_id).is_some() {
            self.sync_fcl_and_std_output(false);
        } // else (no logger) the stack unwinding of the current thread is in progress. Do nothing (suppress flushing).
        if let Some(logger) = self.get_thread_logger(current_thread_id) {
            logger.log_loopbody_end();
            self.last_fcl_update_thread = Some(current_thread_id);
        } // else (no logger) the stack unwinding of the current thread is in progress. Do nothing (suppress 
        // the fake loopbody end logging).
    }
}

// TODO: Remove after the freezing is clarified.
impl Drop for CallLoggerArbiter {
    fn drop(&mut self) {
        println!("<CallLoggerArbiter as Drop>::drop()");
    }
}

struct CallLoggerAdapter {
    arbiter: Arc<Mutex<CallLoggerArbiter>>,
}
impl CallLoggerAdapter {
    fn new(arbiter: Arc<Mutex<CallLoggerArbiter>>) -> Self {
        Self { arbiter }
    }
    fn get_arbiter(&self) -> MutexGuard<'_, CallLoggerArbiter> {
        match self.arbiter.lock() {
            // TODO: Revert after the freezing is clarified.
            Ok(guard) => {
                return guard;
            }
            Err(poisoned) => {
                println!("Internal Error: Unexpected mutex lock failure in get_arbiter(): '{:?}'", poisoned);
                // panic!("Internal Error: Unexpected mutex lock failure: '{:?}'", e);
                return poisoned.into_inner()
            }
        }
        // if let Ok(guard) = self.arbiter.lock() {
        //     return guard;
        // } else {
        //     debug_assert!(false, "Unexpected mutex lock failure")
        // }
    }
}
impl Drop for CallLoggerAdapter {
    fn drop(&mut self) {
        self.get_arbiter().remove_thread_logger();
    }
}
impl CallLogger for CallLoggerAdapter {
    fn push_logging_is_on(&mut self, is_on: bool) {
        self.get_arbiter().push_logging_is_on(is_on);
    }
    fn pop_logging_is_on(&mut self) {
        self.get_arbiter().pop_logging_is_on();
    }
    fn logging_is_on(&self) -> bool {
        self.get_arbiter().logging_is_on()
    }
    fn set_logging_is_on(&mut self, is_on: bool) {
        self.get_arbiter().set_logging_is_on(is_on)
    }

    fn set_thread_indent(&mut self, thread_indent: &'static str) {
        self.get_arbiter().set_thread_indent(thread_indent)
    }

    fn log_call(&mut self, name: &str, param_vals: Option<String>) {
        self.get_arbiter().log_call(name, param_vals)
    }
    fn log_ret(&mut self, ret_val: Option<String>) {
        self.get_arbiter().log_ret(ret_val)
    }
    fn maybe_flush(&mut self) {
        self.get_arbiter().maybe_flush();
    }
    // NOTE: Reuses the trait's `fn flush(&mut self) {}` that does nothing.
    fn log_loopbody_start(&mut self) {
        self.get_arbiter().log_loopbody_start()
    }
    fn log_loopbody_end(&mut self) {
        self.get_arbiter().log_loopbody_end()
    }
}

// Global data shared by all the threads:
// TODO: Test with {file, socket, pipe} writer as an arg to `ThreadSharedWriter::new()`.
static mut THREAD_SHARED_WRITER: LazyLock<ThreadSharedWriterPtr> = LazyLock::new(|| {
    Arc::new(RefCell::new(ThreadSharedWriter::new(
        Some(crate::writer::FclWriter::Stdout),
        // None, 
        // Some(Box::new(std::io::stderr /*stdout*/())), /*None*/
    )))
});
static mut CALL_LOGGER_ARBITER: LazyLock<Arc<Mutex<CallLoggerArbiter>>> = LazyLock::new(|| {
    Arc::new(Mutex::new({
        let mut arbiter = unsafe { CallLoggerArbiter::new((*THREAD_SHARED_WRITER).clone()) };
        arbiter.set_std_output_sync();
        arbiter.set_panic_sync();
        arbiter
    }))
});

// TODO: COnsider removing `LazyLock<RefCell<>>`.
static mut ORIGINAL_PANIC_HOOK: LazyLock<RefCell<Option<Box<dyn Fn(&std::panic::PanicHookInfo<'_>)>>>> = 
    LazyLock::new(|| {
        RefCell::new(None)
    });

// Global data per thread. Each thread has its own copy of these data.
// These data are initialized first upon thread start, and destroyed last upon thread termination.
thread_local! {
    pub static THREAD_LOGGER: RefCell<Box<dyn CallLogger>> = {
        RefCell::new(Box::new(CallLoggerAdapter::new(
            {
                unsafe {
                    match (*CALL_LOGGER_ARBITER).lock() {
                        Ok(mut guard) => {
                            guard.add_thread_logger(Box::new(
                                CallLogInfra::new(Rc::new(RefCell::new(
                                    // fcl_decorators::TreeLikeDecorator::new(
                                    //     Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
                                    //     None, None, None))))))
                                    fcl_decorators::CodeLikeDecorator::new(
                                        Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
                                        None))))))
                        }
                        Err(e) => {
                            println!("CallLoggerAdapter creation panic: '{:?}'", e); // TODO: Remove after the freezing is clarified.
                            debug_assert!(false, "Unexpected mutex lock failure: '{:?}'", e);
                        } 
                    }
                    // if let Ok(mut guard) = (*CALL_LOGGER_ARBITER).lock() {
                    //     guard.add_thread_logger(Box::new(
                    //         CallLogInfra::new(Rc::new(RefCell::new(
                    //             // fcl_decorators::TreeLikeDecorator::new(
                    //             //     Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
                    //             //     None, None, None))))))
                    //             fcl_decorators::CodeLikeDecorator::new(
                    //                 Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
                    //                 None))))))
                    // } else {
                    //     println!("CallLoggerAdapter creation panic");
                    //     panic!("Unexpected mutex lock failure")
                    // }
                }
                let call_logger_arbiter;
                unsafe {
                    call_logger_arbiter = (*CALL_LOGGER_ARBITER).clone();
                }
                call_logger_arbiter
            })))
    };
}
