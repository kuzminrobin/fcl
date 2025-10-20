// The code_commons crate is to be reused for various code-handling projects.
mod call_graph;
pub use call_graph::{CallGraph, ItemKind, RepeatCountCategory};

/// Trait to be implemented by the instances that need to be notified about the code run events
/// (such as function or closure calls, returns, etc.).
pub trait CoderunNotifiable {
    /// Non-cached call happened.
    fn notify_call(&mut self, _call_depth: usize, _name: &str, _param_vals: &Option<String>) {}
    /// Non-cached return happened.
    fn notify_return(
        &mut self,
        _call_depth: usize,
        _name: &str,
        _has_nested_calls: bool,
        _ret_val: &Option<String>,
    ) {
    }
    /// Repeat count has stopped being cached.
    fn notify_repeat_count(
        &mut self,
        _call_depth: usize,
        _kind: &ItemKind,
        _count: RepeatCountCategory,
    ) {
    }

    /// Flush needed (any output cached by this trait implementor needs to be flushed).
    fn notify_flush(&mut self) {}

    /// Loop body has stopped being cached.
    fn notify_loopbody_start(&mut self, _call_depth: usize);

    /// Loop body (iteration) has ended (but not necessarily the whole loop).
    fn notify_loopbody_end(&mut self, _call_depth: usize) {}
}

