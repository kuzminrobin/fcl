// call_log_infra

use call_graph::CallGraph;
use fcl_decorators::CodeLikeDecorator;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, LazyLock},
};
// use fcl_decorators::TreeLikeDecorator;
use fcl_traits::{CalleeName, CoderunNotifiable, CoderunThreadSpecificNotifyable, ThreadSpecifics};

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
    pub fn push_is_on(&mut self, is_on: bool) {
        self.is_on.push(is_on)
    }
    pub fn pop_is_on(&mut self) {
        self.is_on.pop();
    }
    pub fn is_on(&self) -> bool {
        *self.is_on.last().unwrap_or(&false)
    }
    pub fn set_is_on(&mut self, is_on: bool) {
        self.is_on.pop();
        self.is_on.push(is_on);
    }

    pub fn set_thread_indent(&mut self, thread_indent: &'static str) {
        self.thread_specifics
            .borrow_mut()
            .set_thread_indent(thread_indent);
    }

    pub fn log_call(&mut self, name: &CalleeName) {
        self.call_graph.add_call(name);
    }
    pub fn log_ret(&mut self) {
        self.call_graph.add_ret();
    }
}

// TODO: Test with file, socket writer as an arg to `ThreadSharedWriter::new()`.
static mut THREAD_SHARED_WRITER: LazyLock<ThreadSharedWriterPtr> =
    LazyLock::new(|| Arc::new(RefCell::new(ThreadSharedWriter::new(None))));

thread_local! {
    // pub static WRITER_ADAPTER: RefCell<WriterAdapter> =
    //     RefCell::new(WriterAdapter::new(unsafe { (*THREAD_SHARED_WRITER).clone() }));

    // pub static CALL_LOG_DECORATOR: RefCell<dyn CoderunThreadSpecificNotifyable>
    pub static CALL_LOG_INFRA: RefCell<CallLogInfra> = {
        // let notifyable_decorator = Rc::new(RefCell::new(CodeLikeDecorator::new(None, None)));
        // RefCell::new(CallLogInfra::new(Rc::clone(&<notifyable_decorator as Rc<RefCell<dyn CoderunNotifiable>>>)))


        RefCell::new(CallLogInfra::new(Rc::new(RefCell::new(
            CodeLikeDecorator::new(
                Some(Box::new(WriterAdapter::new(unsafe { (*THREAD_SHARED_WRITER).clone() }))), 
                None)))))
        // // RefCell::new(CallLogInfra::new(Rc::new(RefCell::new(TreeLikeDecorator::new(None, None, None, None)))));
        // RefCell::new(CallLogInfra::new(Rc::new(RefCell::new(CodeLikeDecorator::new(None, None)))))
    };
}
