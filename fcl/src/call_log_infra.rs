use code_commons::{CallGraph, CoderunNotifiable};
// use code_commons::{CallGraph, CalleeName, CoderunNotifiable};
use fcl_traits::{CallLogger, CoderunThreadSpecificNotifyable, ThreadSpecifics};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, LazyLock, Mutex, MutexGuard},
    thread,
};

use crate::writer::{ThreadSharedWriter, ThreadSharedWriterPtr, WriterAdapter};

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
    // fn log_call(&mut self, name: &CalleeName) {
        self.call_graph.add_call(name, param_vals);
    }
    fn log_ret(&mut self, output: Option<String>) {
        self.call_graph.add_ret(output);
    }

    // TODO: Consider making this impl conditional, for multithreaded case only.
    fn flush(&mut self) {
        self.call_graph.flush()
    }
}

pub struct CallLoggerArbiter {
    thread_loggers: HashMap<thread::ThreadId, Box<dyn CallLogger>>,
    last_output_thread: Option<thread::ThreadId>,
}

impl CallLoggerArbiter {
    pub fn new() -> Self {
        Self {
            thread_loggers: HashMap::new(),
            last_output_thread: None,
        }
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
        self.get_thread_logger(current_thread_id).flush(); // Flush the possible trailing repeat count.

        if self.thread_loggers.remove(&current_thread_id).is_none() {
            debug_assert!(
                false,
                "Internal error suspected: Unregistering non-registered thread"
            );
        }
        if self.last_output_thread == Some(current_thread_id) {
            self.last_output_thread = None; // Prevent subsequent flushing of the terminated thread.
        }
    }
    fn get_thread_logger(&mut self, thread_id: thread::ThreadId) -> &mut Box<dyn CallLogger> {
        if let Some(logger) = self.thread_loggers.get_mut(&thread_id) {
            return logger;
        } else {
            panic!("Internal error: Logging by unregistered thread");
        }
    }
    fn flush_earlier_thread_output(&mut self) {
        if let Some(last_output_thread) = self.last_output_thread
            && thread::current().id() != last_output_thread
        {
            self.get_thread_logger(last_output_thread).flush()
        }
    }
}

impl CallLogger for CallLoggerArbiter {
    fn push_logging_is_on(&mut self, is_on: bool) {
        self.get_thread_logger(thread::current().id())
            .push_logging_is_on(is_on)
    }
    fn pop_logging_is_on(&mut self) {
        self.get_thread_logger(thread::current().id())
            .pop_logging_is_on()
    }
    fn logging_is_on(&self) -> bool {
        if let Some(logger) = self.thread_loggers.get(&thread::current().id()) {
            return logger.logging_is_on();
        } else {
            panic!("Internal error: Logging by unregistered thread");
        }
    }
    fn set_logging_is_on(&mut self, is_on: bool) {
        self.get_thread_logger(thread::current().id())
            .set_logging_is_on(is_on)
    }

    fn set_thread_indent(&mut self, thread_indent: &'static str) {
        self.get_thread_logger(thread::current().id())
            .set_thread_indent(thread_indent)
    }

    fn log_call(&mut self, name: &str, param_vals: Option<String>) {
    // fn log_call(&mut self, name: &CalleeName) {
        self.flush_earlier_thread_output();

        let current_thread_id = thread::current().id();
        self.get_thread_logger(current_thread_id).log_call(name, param_vals);
        self.last_output_thread = Some(current_thread_id);
    }
    fn log_ret(&mut self, output: Option<String>) {
        self.flush_earlier_thread_output();

        let current_thread_id = thread::current().id();
        self.get_thread_logger(current_thread_id).log_ret(output);
        self.last_output_thread = Some(current_thread_id);
    }
    // NOTE: Reuses the trait's `fn flush(&mut self) {}` that does nothing.
}

struct CallLoggerAdapter {
    arbiter: Arc<Mutex<CallLoggerArbiter>>,
}
impl CallLoggerAdapter {
    fn new(arbiter: Arc<Mutex<CallLoggerArbiter>>) -> Self {
        Self { arbiter }
    }
    fn get_arbiter(&self) -> MutexGuard<'_, CallLoggerArbiter> {
        if let Ok(guard) = self.arbiter.lock() {
            return guard;
        } else {
            panic!("Unexpected mutex lock failure")
        }
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
    // fn log_call(&mut self, name: &CalleeName) {
        self.get_arbiter().log_call(name, param_vals)
    }
    fn log_ret(&mut self, output: Option<String>) {
        self.get_arbiter().log_ret(output)
    }
    // NOTE: Reuses the trait's `fn flush(&mut self) {}` that does nothing.
}

// TODO: Test with {file, socket, pipe} writer as an arg to `ThreadSharedWriter::new()`.
static mut THREAD_SHARED_WRITER: LazyLock<ThreadSharedWriterPtr> =
    LazyLock::new(|| Arc::new(RefCell::new(ThreadSharedWriter::new(None))));
static mut CALL_LOGGER_ARBITER: LazyLock<Arc<Mutex<CallLoggerArbiter>>> =
    LazyLock::new(|| Arc::new(Mutex::new(CallLoggerArbiter::new())));

thread_local! {
    pub static THREAD_LOGGER: RefCell<Box<dyn CallLogger>> = {
        RefCell::new(Box::new(CallLoggerAdapter::new(
            {
                unsafe {
                    if let Ok(mut guard) = (*CALL_LOGGER_ARBITER).lock() {
                        guard.add_thread_logger(Box::new(
                            CallLogInfra::new(Rc::new(RefCell::new(
                                // fcl_decorators::TreeLikeDecorator::new(
                                //     Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
                                //     None, None, None))))))
                                fcl_decorators::CodeLikeDecorator::new(
                                    Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
                                    None))))))
                    } else {
                        panic!("Unexpected mutex lock failure")
                    }
                }
                let call_logger_arbiter;
                unsafe {
                    call_logger_arbiter = (*CALL_LOGGER_ARBITER).clone();
                }
                call_logger_arbiter
            })))
    };
}
