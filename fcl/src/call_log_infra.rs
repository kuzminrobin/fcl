// call_log_infra

use call_graph::CallGraph;
// use fcl_decorators::TreeLikeDecorator;
use fcl_decorators::{CodeLikeDecorator, ThreadAwareWriter, ThreadAwareWriterType};
use parking_lot::ReentrantMutex;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, LazyLock},
};
// use fcl_decorators::TreeLikeDecorator;
use fcl_traits::{CalleeName, CoderunNotifiable, CoderunThreadSpecificNotifyable, ThreadSpecifics};

pub struct CallLogInfra {
    is_on: Vec<bool>, // Disabled by default (if empty). TODO: Consider renaming to `logging_is_on`.
    // code_run_decorator: Rc<RefCell<dyn CodeRunDecorator>>,
    thread_specifics: Rc<RefCell<dyn ThreadSpecifics>>,
    flusher: Rc<RefCell<dyn CoderunNotifiable>>,
    call_graph: CallGraph,
}

// impl Flushable for CallLogInfra {
//     fn flush(&mut self) {
        
//     }
// }

impl CallLogInfra {
    pub fn init_flushable(&mut self/*, thread_spec_notifyable: Rc<RefCell<dyn CoderunThreadSpecificNotifyable>> */) {
        self.flusher.borrow_mut().set_flushable(&self.call_graph);
        // let coderun_notifiable: Rc<RefCell<dyn CoderunNotifiable>> = thread_spec_notifyable;    //.clone(); // `Rc::clone(&thread_spec_notifyable)` fails. // NOTE: Curious trick (TODO: Document it).
        // coderun_notifiable.borrow_mut().set_flushable(&self.call_graph);
    }

    pub fn new(thread_spec_notifyable: Rc<RefCell<dyn CoderunThreadSpecificNotifyable>>) -> Self {
        // pub fn new(code_run_notifyable: Rc<RefCell<dyn CoderunNotifiable + CodeRunDecorator>>) -> Self {
        let coderun_notifiable: Rc<RefCell<dyn CoderunNotifiable>> = thread_spec_notifyable.clone(); // `Rc::clone(&thread_spec_notifyable)` fails. // NOTE: Curious trick (TODO: Document it).
        let thread_specifics: Rc<RefCell<dyn ThreadSpecifics>> = thread_spec_notifyable;
        // TODO: return just `Self { ... }`
        let instance = Self {
            is_on: Vec::with_capacity(4),
            // code_run_decorator: Rc::clone(&code_run_notifyable),
            thread_specifics,
            flusher: coderun_notifiable.clone(),
            call_graph: CallGraph::new(coderun_notifiable.clone()),
            // call_graph: CallGraph::new(Rc::clone(&coderun_notifiable)),
            // call_graph: CallGraph::new(Rc::clone(&thread_spec_notifyable)),
        };
        // coderun_notifiable.borrow_mut().set_flushable(&instance.call_graph);
        instance
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

static mut THREAD_AWARE_WRITER: LazyLock<ThreadAwareWriterType>
        /* : Arc<Mutex<ThreadAwareWriter>>*/ = LazyLock::new(||
        Arc::new(ReentrantMutex::new(RefCell::new(ThreadAwareWriter::new(None))))
    );

thread_local! {
    pub static CALL_LOG_INFRA: RefCell<CallLogInfra> = {
        let thread_aware_writer;
        unsafe {
            thread_aware_writer = (*THREAD_AWARE_WRITER).clone();
        };

        // let infra = RefCell::new(CallLogInfra::new(Rc::new(RefCell::new(TreeLikeDecorator::new(thread_aware_writer/* , None*/, None, None, None)))));
        let decorator = Rc::new(RefCell::new(CodeLikeDecorator::new(thread_aware_writer/* , None*/, None)));
        let infra = RefCell::new(CallLogInfra::new(decorator.clone()));
        // let infra = RefCell::new(CallLogInfra::new(Rc::new(RefCell::new(CodeLikeDecorator::new(thread_aware_writer/* , None*/, None)))));
        
        // infra.borrow_mut().init_flushable(decorator);
        infra
    };
}
