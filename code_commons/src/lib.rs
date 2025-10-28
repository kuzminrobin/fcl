// The code_commons crate is to be reused for various code-handling projects.
mod call_graph;
pub use call_graph::{CallGraph, ItemKind, RepeatCountCategory};

/// A trait to be implemented by the instances that need to be notified about the code run events
/// (such as function or closure calls, returns, etc.).
pub trait CoderunNotifiable {
    /// Notifies about a function or a closure call.
    /// # Parameters
    /// * The call depth.
    /// * The call name.
    /// * The optional string representation of the parameter names and values.
    fn notify_call(&mut self, _call_depth: usize, _name: &str, _param_vals: &Option<String>) {}

    /// Notifies about a function or a closure return.
    /// # Parameters
    /// * The call depth.
    /// * The call name.
    /// * Flag telling if the call has nested calls.
    /// * The optional string representation of the return value.
    fn notify_return(
        &mut self,
        _call_depth: usize,
        _name: &str,
        _has_nested_calls: bool,
        _ret_val: &Option<String>,
    ) {
    }
    /// Notifies about a repeat count.
    /// # Parameters
    /// * The call depth.
    /// * Call tree item info (function/closure or loop body, name, etc.).
    /// * Call tree item repeat count info.
    fn notify_repeat_count(
        &mut self,
        _call_depth: usize,
        _kind: &ItemKind,
        _count: RepeatCountCategory,
    ) {
    }

    /// Notifies about a flush.
    /// Any output cached by this trait implementor needs to be flushed.
    fn notify_flush(&mut self) {}

    /// Notifies about a loop body start.
    /// # Parameters
    /// * The call depth.
    fn notify_loopbody_start(&mut self, _call_depth: usize);

    /// Notifies about a loop body end.
    /// # Parameters
    /// * The call depth.
    fn notify_loopbody_end(&mut self, _call_depth: usize) {}
}

