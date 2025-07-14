#[cfg(not(feature = "minimal_writer"))]
use std::cell::LazyCell;
#[cfg(not(feature = "singlethreaded"))]
use std::sync::Arc;
use std::{cell::RefCell, collections::HashMap, io::Write, rc::Rc, sync::LazyLock, thread};

use crate::CallLogger;
use crate::decorators::{CoderunThreadSpecificNotifyable, ThreadSpecifics};
use code_commons::{CallGraph, CoderunNotifiable};

#[cfg(not(feature = "minimal_writer"))]
use crate::output_sync::StdOutputRedirector;
#[cfg(not(feature = "minimal_writer"))]
use writer::{THREAD_SHARED_WRITER, ThreadSharedWriterPtr, WriterAdapter, WriterKind};

#[cfg(not(feature = "minimal_writer"))]
mod writer;

macro_rules! NO_LOGGER_ERR_STR {
    () => {
        "Internal Error: Unexpected lack of logger"
    };
}

pub struct CallLogInfra {
    logging_is_on: Vec<bool>, // Enabled by default (if empty).
    thread_specifics: Rc<RefCell<dyn ThreadSpecifics>>,
    call_graph: CallGraph,
}

impl CallLogInfra {
    pub fn new(thread_spec_notifyable: Rc<RefCell<dyn CoderunThreadSpecificNotifyable>>) -> Self {
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

struct ThreadIndents {
    indents_taken: Vec<bool>,
    thread_indent_step: String,
}
impl ThreadIndents {
    fn new(thread_indent_step: Option<String>) -> Self {
        Self {
            indents_taken: vec![false, false, false, false],
            thread_indent_step: thread_indent_step
                .unwrap_or(String::from("                                ")), // 32 spaces
        }
    }
    fn idx_to_string(&self, index: usize) -> String {
        let mut ret_val = String::with_capacity(index * self.thread_indent_step.len());
        for _ in 0..index {
            ret_val.push_str(&self.thread_indent_step)
        }
        ret_val
    }
    fn check_out(&mut self) -> (usize, String) {
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
    fn check_in(&mut self, index: usize) {
        self.indents_taken[index] = false;
    }
}

#[cfg(not(feature = "minimal_writer"))]
struct OutputSync {
    thread_shared_writer: Option<ThreadSharedWriterPtr>,
    stderr_redirector: Option<StdOutputRedirector>,
    stdout_redirector: Option<StdOutputRedirector>,
    main_thread_id: thread::ThreadId,
}

pub struct CallLoggerArbiter {
    /// Collection of per-thread loggers and thread indent IDs used by the corresponding logger.
    thread_loggers: HashMap<thread::ThreadId, (Box<dyn CallLogger>, usize)>,
    last_fcl_update_thread: Option<thread::ThreadId>,
    thread_indents: ThreadIndents,

    #[cfg(not(feature = "minimal_writer"))]
    output_sync: OutputSync,
}

impl CallLoggerArbiter {
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
                main_thread_id: thread::current().id(),
            },
        };
    }
    pub fn add_thread_logger(&mut self, mut thread_logger: Box<dyn CallLogger>) {
        let (thread_indent_id, thread_indent) = self.thread_indents.check_out();
        thread_logger.set_thread_indent(thread_indent);
        if self
            .thread_loggers
            .insert(thread::current().id(), (thread_logger, thread_indent_id))
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
        if let Some((_logger, thread_indent_id)) = self.get_thread_logger(current_thread_id) {
            let thread_indent_id = *thread_indent_id; // Released the mutable borrow.

            // Flush the possible trailing repeat count and std output.
            #[cfg(not(feature = "minimal_writer"))]
            self.sync_fcl_and_std_output(true);
            #[cfg(feature = "minimal_writer")]
            if let Some((logger, ..)) = self.get_thread_logger(thread::current().id()) {
                logger.flush();
            }

            if self.thread_loggers.remove(&current_thread_id).is_none() {
                debug_assert!(false, "Internal Error: Unregistering failed");
            }
            self.thread_indents.check_in(thread_indent_id);
        } // else (no logger) the logger for the current thread is assumed removed in the panic handler.

        #[cfg(not(feature = "minimal_writer"))]
        if self.thread_loggers.is_empty() {
            // The last remaining main() thread has terminated, its thread_local data are being destroyed;
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
    #[cfg(not(feature = "minimal_writer"))]
    pub fn set_panic_sync(&mut self) {
        unsafe {
            *(*ORIGINAL_PANIC_HANDLER).borrow_mut() = Some(std::panic::take_hook());
        }
        std::panic::set_hook(Box::new(Self::panic_hook))
    }
    #[cfg(not(feature = "minimal_writer"))]
    pub fn set_std_output_sync(&mut self) {
        let Some(thread_shared_writer) = self.output_sync.thread_shared_writer.clone() else {
            return;
        };

        // Set stdout and stderr redirection to a corresponding buffer (set std output buffering):
        let writer_kind = thread_shared_writer.borrow().get_writer_kind();
        // let writer_kind = self.thread_shared_writer.borrow().get_writer_kind();
        self.output_sync.stderr_redirector =
            Self::set_stdx_sync(writer_kind, StdOutputRedirector::new_stderr());
        self.output_sync.stdout_redirector =
            Self::set_stdx_sync(writer_kind, StdOutputRedirector::new_stdout());

        // Recover {FCL's own logging directly to the original stdout (or stderr)}
        // while still buffering the program's {stdout and stderr} output.
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
            // Ok(file) => self.thread_shared_writer.borrow_mut().set_writer(file),
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
            }
        });
    }
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
                arbiter.sync_fcl_and_std_output(true);
                arbiter.remove_thread_logger();
                if thread::current().id() == arbiter.output_sync.main_thread_id {
                    // The main() thread is panicking.
                    // Stop buffering the std output.
                    arbiter.output_sync.stderr_redirector = None;
                    arbiter.output_sync.stdout_redirector = None;
                }
            }
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

        // Potentially a call (from a `FunctionLogger` destructor) during stack unwinding in the regular panic handler.
        // In that case suppress flushing and {logging the fake returns in the panicking thread}
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
        } // else (no logger) the stack unwinding of the current thread is in progress. Do nothing (suppress 
        // the fake return logging).
    }
    fn maybe_flush(&mut self) {
        #[cfg(not(feature = "minimal_writer"))]
        self.sync_fcl_and_std_output(false);
    }
    // NOTE: Reuses the trait's `fn flush(&mut self) {}` that does nothing.
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

        // Potentially a call (from a `LoopbodyLogger` destructor) during stack unwinding in the regular panic handler.
        // In that case suppress flushing and {logging the fake loopbody ends in the panicking thread}
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
        } // else (no logger) the stack unwinding of the current thread is in progress. Do nothing (suppress 
        // the fake loopbody end logging).
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
static mut ORIGINAL_PANIC_HANDLER: LazyCell<
    RefCell<Option<Box<dyn Fn(&std::panic::PanicHookInfo<'_>)>>>,
> = LazyCell::new(|| RefCell::new(None));

#[cfg(feature = "singlethreaded")]
pub mod instances {
    use super::*;
    thread_local! {
        pub static THREAD_LOGGER: RefCell<Rc<RefCell<CallLoggerArbiter>>> = unsafe {
            #[cfg(not(feature = "minimal_writer"))]
            let writer: Option<Box<dyn Write>> = Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone())));
            #[cfg(feature = "minimal_writer")]
            let writer: Option<Box<dyn Write>> = None;

            let logging_infra = Box::new(CallLogInfra::new(std::rc::Rc::new(std::cell::RefCell::new(
                // crate::decorators::TreeLikeDecorator::new(
                // // fcl_decorators::TreeLikeDecorator::new(
                //     writer,
                //     None, None, None))))))
                crate::decorators::CodeLikeDecorator::new(
                // fcl_decorators::CodeLikeDecorator::new(
                    writer,
                    None)))));
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
        pub static THREAD_LOGGER: RefCell<Box<dyn CallLogger>> = unsafe {
            let logging_infra = Box::new(/*fcl::call_log_infra::*/CallLogInfra::new(std::rc::Rc::new(std::cell::RefCell::new(
                // crate::decorators::TreeLikeDecorator::new(
                // fcl_decorators::TreeLikeDecorator::new(
                //     Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
                //     None, None, None))))))
                crate::decorators::CodeLikeDecorator::new(
                // fcl_decorators::CodeLikeDecorator::new(
                    Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
                    None)))));
            match (*THREAD_GATEKEEPER).lock() {
                Ok(mut gatekeeper) => gatekeeper.add_thread_logger(logging_infra),
                Err(e) => {
                    println!("(stdout) FCL Internal Error: Thread panicked while holding a mutex ({}).", e);
                    eprintln!("(stderr) FCL Internal Error: Thread panicked while holding a mutex ({}).", e);
                    debug_assert!(false, "FCL Internal Error: Thread panicked while holding a mutex ({}).", e);
                }
            }
            RefCell::new(Box::new(ThreadGateAdapter::new((*THREAD_GATEKEEPER).clone())))
        };
    }
}
