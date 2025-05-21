// call_log_infra

use std::cell::RefCell;
use call_graph::CallGraph;
use fcl_decorators::CodeLikeDecorator;
// use fcl_decorators::TreeLikeDecorator;
use fcl_traits::{CalleeName, CoderunNotifiable};

pub struct CallLogInfra {
    is_on: Vec<bool>, // Disabled by default (if empty). TODO: Consider renaming to `logging_is_on`.
    call_graph: CallGraph,
}

impl CallLogInfra {
    pub fn new(code_run_decorator: Box<dyn CoderunNotifiable>) -> Self {
        Self {
            is_on: Vec::with_capacity(4),
            call_graph: CallGraph::new(code_run_decorator),
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

    pub fn log_call(&mut self, name: &CalleeName) {
        self.call_graph.add_call(name);
    }
    pub fn log_ret(&mut self) {
        self.call_graph.add_ret();
    }
}

thread_local! {
    pub static CALL_LOG_INFRA: RefCell<CallLogInfra> =
        // RefCell::new(CallLogInfra::new(Box::new(TreeLikeDecorator::new(None, None, None, None))));
        RefCell::new(CallLogInfra::new(Box::new(CodeLikeDecorator::new(None, None))));
        // RefCell::new(CallLogInfra::new(Box::new(CodeLikeDecorator::new(&"  ", None))));
        // RefCell::new(CallLogInfra::new(Box::new(OldCodeRunDecorator::new(&"  ", None))));
}
