// call_log_infra

use call_graph::CallGraph;
use fcl_decorators::CodeLikeDecorator;
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, LazyLock, Mutex, MutexGuard},
    thread,
};
// use fcl_decorators::TreeLikeDecorator;
use fcl_traits::{
    CallLogger, CalleeName, CoderunNotifiable, CoderunThreadSpecificNotifyable, ThreadSpecifics,
};

use crate::writer::{ThreadSharedWriter, ThreadSharedWriterPtr, WriterAdapter};

pub struct CallLogInfra {
    is_on: Vec<bool>, // Disabled by default (if empty). TODO: Consider renaming to `logging_is_on`.
    // code_run_decorator: Rc<RefCell<dyn CodeRunDecorator>>,
    thread_specifics: Rc<RefCell<dyn ThreadSpecifics>>,
    call_graph: CallGraph,
}

impl CallLogInfra {
    pub fn new(thread_spec_notifyable: Rc<RefCell<dyn CoderunThreadSpecificNotifyable>>) -> Self {
        // pub fn new(code_run_notifyable: Rc<RefCell<dyn CoderunNotifiable + CodeRunDecorator>>) -> Self {
        let coderun_notifiable: Rc<RefCell<dyn CoderunNotifiable>> = thread_spec_notifyable.clone(); // Rc::clone(&thread_spec_notifyable); // TODO: Make sure that his trick works. // NOTE: Curious trick.
        let thread_specifics: Rc<RefCell<dyn ThreadSpecifics>> = thread_spec_notifyable;
        Self {
            is_on: Vec::with_capacity(4),
            // code_run_decorator: Rc::clone(&code_run_notifyable),
            thread_specifics,
            call_graph: CallGraph::new(coderun_notifiable),
            // call_graph: CallGraph::new(Rc::clone(&coderun_notifiable)),
            // call_graph: CallGraph::new(Rc::clone(&thread_spec_notifyable)),
        }
    }
}

impl CallLogger for CallLogInfra {
    fn push_is_on(&mut self, is_on: bool) {
        self.is_on.push(is_on)
    }
    fn pop_is_on(&mut self) {
        self.is_on.pop();
    }
    fn is_on(&self) -> bool {
        *self.is_on.last().unwrap_or(&false)
    }
    fn set_is_on(&mut self, is_on: bool) {
        self.is_on.pop();
        self.is_on.push(is_on);
    }

    fn set_thread_indent(&mut self, thread_indent: &'static str) {
        self.thread_specifics
            .borrow_mut()
            .set_thread_indent(thread_indent);
    }

    fn log_call(&mut self, name: &CalleeName) {
        self.call_graph.add_call(name);
    }
    fn log_ret(&mut self) {
        self.call_graph.add_ret();
    }

    // TODO: Make this impl conditional, for multithreaded case only.
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
    pub fn add(&mut self, thread_logger: Box<dyn CallLogger>) {
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
    // TODO: remove -> {remove_thread_logger?}
    pub fn remove(&mut self /*, thread_logger: Box<dyn CallLogger>*/) {
        let current_thread_id = thread::current().id();
        self.get_logger(current_thread_id).flush(); // Flush the possible repeat count.

        if self
            .thread_loggers
            .remove(&current_thread_id)
            .is_none()
        {
            debug_assert!(
                false,
                "Internal error suspected: Unregistering non-registered thread"
            );
        }
        if self.last_output_thread == Some(current_thread_id) {
            self.last_output_thread = None; // Prevent subsequent flushing of the terminated thread.
        }
    }
    fn get_logger(&mut self, thread_id: thread::ThreadId) -> &mut Box<dyn CallLogger> {
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
            self.get_logger(last_output_thread).flush()
        }
    }
}

impl CallLogger for CallLoggerArbiter {
    fn push_is_on(&mut self, is_on: bool) {
        self.get_logger(thread::current().id()).push_is_on(is_on)
    }
    fn pop_is_on(&mut self) {
        self.get_logger(thread::current().id()).pop_is_on()
    }
    fn is_on(&self) -> bool {
        if let Some(logger) = self.thread_loggers.get(&thread::current().id()) {
            return logger.is_on();
        } else {
            panic!("Internal error: Logging by unregistered thread");
        }
    }
    fn set_is_on(&mut self, is_on: bool) {
        self.get_logger(thread::current().id()).set_is_on(is_on)
    }

    fn set_thread_indent(&mut self, thread_indent: &'static str) {
        self.get_logger(thread::current().id())
            .set_thread_indent(thread_indent)
    }

    fn log_call(&mut self, name: &CalleeName) {
        self.flush_earlier_thread_output();

        let current_thread_id = thread::current().id();
        self.get_logger(current_thread_id).log_call(name);
        self.last_output_thread = Some(current_thread_id);
    }
    fn log_ret(&mut self) {
        self.flush_earlier_thread_output();

        let current_thread_id = thread::current().id();
        self.get_logger(current_thread_id).log_ret();
        self.last_output_thread = Some(current_thread_id);
    }
    // fn flush(&mut self) {}
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
        self.get_arbiter().remove();
    }
}
impl CallLogger for CallLoggerAdapter {
    fn push_is_on(&mut self, is_on: bool) {
        self.get_arbiter().push_is_on(is_on);
    }
    fn pop_is_on(&mut self) {
        self.get_arbiter().pop_is_on();
    }
    fn is_on(&self) -> bool {
        self.get_arbiter().is_on()
    }
    fn set_is_on(&mut self, is_on: bool) {
        self.get_arbiter().set_is_on(is_on)
    }

    fn set_thread_indent(&mut self, thread_indent: &'static str) {
        self.get_arbiter()
            // self.get_logger(thread::current().id())
            .set_thread_indent(thread_indent)
    }

    fn log_call(&mut self, name: &CalleeName) {
        self.get_arbiter().log_call(name)
    }
    fn log_ret(&mut self) {
        self.get_arbiter().log_ret()
    }
    // fn flush(&mut self) {}
}

// TODO: Test with file, socket writer as an arg to `ThreadSharedWriter::new()`.
static mut THREAD_SHARED_WRITER: LazyLock<ThreadSharedWriterPtr> =
    LazyLock::new(|| Arc::new(RefCell::new(ThreadSharedWriter::new(None))));
static mut CALL_LOGGER_ARBITER: LazyLock<Arc<Mutex<CallLoggerArbiter>>> =
    LazyLock::new(|| Arc::new(Mutex::new(CallLoggerArbiter::new())));

thread_local! {
    // pub static WRITER_ADAPTER: RefCell<WriterAdapter> =
    //     RefCell::new(WriterAdapter::new(unsafe { (*THREAD_SHARED_WRITER).clone() }));

    pub static THREAD_LOGGER: RefCell<Box<dyn CallLogger>> = {
        RefCell::new(Box::new(CallLoggerAdapter::new(
            {
                unsafe {
                    if let Ok(mut guard) = (*CALL_LOGGER_ARBITER).lock() {
                        guard.add(Box::new(
                            CallLogInfra::new(Rc::new(RefCell::new(
                                CodeLikeDecorator::new(
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
            // CALL_LOGGER_ARBITER.
            // CallLogInfra::new(Rc::new(RefCell::new(
            // CodeLikeDecorator::new(
            //     Some(Box::new(WriterAdapter::new(unsafe { (*THREAD_SHARED_WRITER).clone() }))),
            //     None))))))
    };

    // pub static CALL_LOG_INFRA: RefCell<CallLogInfra> = {
    // // pub static CALL_LOG_DECORATOR: RefCell<dyn CoderunThreadSpecificNotifyable>
    //     // let notifyable_decorator = Rc::new(RefCell::new(CodeLikeDecorator::new(None, None)));
    //     // RefCell::new(CallLogInfra::new(Rc::clone(&<notifyable_decorator as Rc<RefCell<dyn CoderunNotifiable>>>)))


    //     RefCell::new(CallLogInfra::new(Rc::new(RefCell::new(
    //         CodeLikeDecorator::new(
    //             Some(Box::new(WriterAdapter::new(unsafe { (*THREAD_SHARED_WRITER).clone() }))),
    //             None)))))
    //     // // RefCell::new(CallLogInfra::new(Rc::new(RefCell::new(TreeLikeDecorator::new(None, None, None, None)))));
    //     // RefCell::new(CallLogInfra::new(Rc::new(RefCell::new(CodeLikeDecorator::new(None, None)))))
    // };
}
