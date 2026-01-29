use crate::CoderunNotifiable;
use std::{cell::RefCell, rc::Rc};

type Link = Rc<RefCell<CallNode>>;

/// The call tree node. Represents one of the following items
/// * function
/// * closure
/// * loop body.
struct CallNode {
    // TODO: Consider CallNode -> ItemNode or CallGraphItemNode.
    /// A call (a function or a closure, with name and optional parameter values), a loop body.
    kind: ItemKind, // TODO: Consider kind -> item_info or item_input_info or signature_info.
    /// String representation of a value returned by a function, closure, or `loop`
    /// (the `while` and `for` loops do not return a value).
    ret_val: Option<String>,
    /// Collection of nested calls (not to be confused with locally defined functions and closures).
    children: Vec<Link>,
    /// Counter that tells how many times the call (including all of its nested calls) repeats.
    repeat_count: RepeatCount,
    /// Flag that tells, during the cache flushing, that the function/closure has returned
    /// or the loopbody (loop iteration) has ended.
    ///
    /// During flush upon thread context switch or {std output and panic} sync
    /// tells whether to to log the item return, e.g.
    /// * `} // function().`
    /// * `} // closure().`
    /// * `} // Loop body end.`
    has_ended: bool,
}
impl CallNode {
    /// Creates a new call tree node.
    fn new(kind: ItemKind) -> Self {
        Self {
            kind: kind,
            ret_val: None,
            children: Vec::new(),
            repeat_count: RepeatCount::new(),
            has_ended: false,
        }
    }
    /// Sets the string representation of the value returned by the function, closure or `loop` iteration.
    fn set_ret_val(&mut self, output: Option<String>) {
        self.ret_val = output;
    }
    /// Provides access to the string representation of the value returned by the function, closure or `loop` iteration.
    fn get_ret_val(&self) -> &Option<String> {
        &self.ret_val
    }
}

/// This structure contains information about cached nodes. Caching avoids duplicate logging.
///
/// For example, if the same function is called multiple times, then the first call is added
/// to the call graph and logged immediately, while subsequent calls are cached in the graph
/// without logging. Each repeated call, upon return, increments the repeat count
/// of the first call and is then removed from the call graph.
///
/// While the sequence of repeated calls continues, the `model_node` field points to the first call,
/// and the `node_being_cached` field points to the current repeated call.
///
/// When the sequence of repeated calls ends, the repeat count is flushed to the log,
/// and caching stops.
struct CachingInfo {
    /// The node to compare the `node_being_cached` with.
    /// This is `None` for the _initial_ loopbody specified by the `node_being_cached`.
    model_node: Option<Link>,
    /// The node that is being cached. If `None` then caching is not active.
    node_being_cached: Option<Link>,
    /// The call depth common for both the `model_node` and `node_being_cached`.
    call_depth: usize,
}
impl CachingInfo {
    /// Initializes and returns the new `CachingInfo` instance.
    fn new() -> Self {
        Self {
            model_node: None,
            node_being_cached: None,
            call_depth: 0,
        }
    }
    /// Stops caching.
    fn clear(&mut self) {
        self.node_being_cached = None; // Stop caching.
        self.model_node = None; // For consistency. Not required.
    }
    /// Tells if cahing is active.
    fn is_active(&self) -> bool {
        self.node_being_cached.is_some()
    }
}

/// Repeat count data type for Function/closure call or loop body.
type RepeatCountType = usize;
/// The maximum value (saturation value) for the repeat count data type.
/// The function can be called in a loop endlessly, such that the function call repeat count can potentially overflow.
/// The algorithm increments that count to the maximum (saturation) value and then stops incrementing.
const REPEAT_COUNT_MAX: RepeatCountType = RepeatCountType::MAX;

/// The category of the repeat count.
///
/// The call tree item repeat count consists of 2 parts:
/// * `overall` - how many times the item repeats; participates in the subtree comparison;
/// * `flushed` - the last flushed value of the `overall`, affects the repeat count value shown
/// during the next flush as a difference `overall - flushed`.
/// The `flushed` must never be greater than `overall`.
///
/// Any of them can reach saturation - `REPEAT_COUNT_MAX` - that affects
/// the repeat count value shown during the next flush. This category of the repeat count
/// tells which part has reached the saturation.
//
// TODO: Consider RepeatCountCategory -> RepeatCountInfo (since also contains a value).
pub enum RepeatCountCategory {
    // For `CallNode::repeat_count::{overall,flushed}`.
    /// Neither `overall` nor `flushed` reached saturation. During next flush the repeat count value
    /// is to be shown as `overall - flushed`, e.g. `// Repeats 6 time(s).`.
    Exact(RepeatCountType),
    /// The `overall` has reached saturation (and stopped incremeting) but the `flushed` has not.
    /// During next flush the repeat count value is to be shown as _at least_ `overall - flushed`,
    /// e.g. `// Repeats 6+ time(s).`.
    AtLeast(RepeatCountType),
    /// Both `overall` and `flushed` have reached saturation. During next flush the repeat count value
    /// is to be shown as _unknown_, e.g. `// Repeats ? time(s).`.
    Unknown,
}
impl RepeatCountCategory {
    /// Converts `RepeatCountCategory` to `String`, e.g.,
    /// * `"5"`  for `RepeatCountCategory::Exact`  
    ///   (neither `overall` nor `flushed` reached saturation,
    ///   the difference between them tells _exactly_ how many times the call repeats since the last flush),
    /// * `"5+"` for `RepeatCountCategory::AtLeast`  
    ///   (`overall` reached saturation, the difference between
    ///   `overall` and `flushed` tells how many times _at least_ the call repeats since the last flush),
    /// * `"?"`  for `RepeatCountCategory::Unknown`  
    ///   (both reached saturation, the difference between
    ///   `overall` and `flushed` doesn't tell anything about how many times the call repeats since the last flush).
    pub fn to_string(&self) -> String {
        match self {
            RepeatCountCategory::Exact(exact) => exact.to_string(),
            RepeatCountCategory::AtLeast(at_least) => at_least.to_string() + "+",
            RepeatCountCategory::Unknown => "?".to_string(),
        }
    }
}

/// Call tree's item kind, one of the following:
/// * a call (function or closure),
/// * a loop body.
#[derive(Clone)]
pub enum ItemKind {
    /// The item is a function or a closure (with name and optional parameter values).
    Call {
        name: String,
        param_vals: Option<String>,
    },
    /// Item is a loop body.
    Loopbody { 
        /// Flag telling that the loop body ends the loop. In other words 
        /// the loop body is the last (non-childless) loop body of the loop.
        /// 
        /// This flag enables detecting that the two adjacent loop bodies 
        /// bolong to different adjacent loops,
        /// the first one is the last loop body of an earlier loop, 
        /// the second one is the first loop body of the later loop.
        ends_the_loop: bool 
    },
}
impl ItemKind {
    /// Tells if the call tree item is a call (a function or a closure) as opposed to the loop body.
    pub fn is_call(&self) -> bool {
        if let Self::Call { .. } = self {
            true
        } else {
            false
        }
    }
    // TODO: Seems redundant (just a negtion of `is_call()`), consider removing.
    pub fn is_loopbody(&self) -> bool {
        if let Self::Loopbody { .. } = self {
            true
        } else {
            false
        }
    }
}

/// The call tree item repeat count. Consists of the two parts.
/// * Actual repeat count. Stops incrementing upon reaching `REPEAT_COUNT_MAX` (saturates).
/// * The flushed part of the actual repeat count. Value less than or equal to the actual repeat count.
#[derive(Clone, Copy)]
pub struct RepeatCount {
    /// Tells how many times the call tree item repeats, participates in the subtree comparison.
    overall: RepeatCountType,
    /// Tells the last flushed value of the `overall`, affects the repeat count value shown
    /// during the next flush as a difference `overall - flushed`. Must never be greater than `overall`.
    flushed: RepeatCountType, // flushed <= overall
}
impl RepeatCount {
    /// Creates a new repeat count.
    pub fn new() -> Self {
        Self {
            overall: 0,
            flushed: 0,
        }
    }
    /// Returns the repeat count category and the value to be used during flush.
    pub fn non_flushed(&self) -> RepeatCountCategory {
        if self.overall < REPEAT_COUNT_MAX {
            return RepeatCountCategory::Exact(self.overall - self.flushed);
        } else if self.flushed < REPEAT_COUNT_MAX {
            return RepeatCountCategory::AtLeast(self.overall - self.flushed);
        }
        RepeatCountCategory::Unknown
    }
    /// Tells (if `true`) if there have definitely not been repeat count increments since the last flush
    /// (in other words the non-flushed value of the repeat count is zero).
    /// If both `overall` and `flushed` have reached saturation, then there potentially could be increments,
    /// `false` is returned.
    pub fn non_flushed_is_empty(&self) -> bool {
        // Equal but not both are saturated:
        self.overall == self.flushed && self.flushed < REPEAT_COUNT_MAX
    }
    /// Increments the `overall` repeat count part if it hasn't saturated.
    pub fn inc(&mut self) {
        if self.overall < REPEAT_COUNT_MAX {
            self.overall += 1
        }
    }
    /// Marks the current repeat count value as flushed.
    /// In other words updates the repeat count `flushed` part with the `overall` value.
    pub fn mark_flushed(&mut self) {
        self.flushed = self.overall
    }
}
impl core::cmp::PartialEq for RepeatCount {
    /// Tells if the two repeat counts are equal (during the subtree comparison).
    fn eq(&self, other: &Self) -> bool {
        self.overall.eq(&other.overall)
    }
}

/// The thread's call graph.
///
/// The per-thread instance of this type contains the full information about
/// the thread's function and closure calls, loop bodies, their order, and repeat counts.
///
/// Typically the call graph of a program or a thread is a tree
/// with the `main()` or a thread function being the root.
/// But if the logging starts later than the call to `main()`
/// (but before the return from `main()`), then the call graph
/// can be not a tree but a sequence of trees (e.g., a sequence of functions called by `main()`).
///
/// For eample, if `main()` calls `f()` and then `g()`,
/// but logging gets enabled after `main()` and before `f()`,
/// then `f()` will be the first-most call to be added to the call graph, and then `g()`.
/// The `f()` followed by `g()`, including their nested calls, will be a sequence of two call trees
/// added to the call graph.
///
/// To unify and simplify handling, a _pseudonode_ is always added to the call graph as a root,
/// which turns any call graph to a tree. Both `f()` and `g()` get added as children of the pseudonode.  
///
/// But if logging gets enabled before `main()` then `main()` gets added as a child to the pseudonode.
pub struct CallGraph {
    /// The call stack.
    ///
    /// In particular a stack of links to the call graph nodes.
    /// In other words a stack of links to the nodes on the path
    /// from the root to the currently active call node in the call graph.
    /// Is used for returning to the parent at any moment in a singly linked call tree.
    ///
    /// The link to a pseudonode always exists at the bottom of the call stack.
    /// The pseudonode is not to be logged. Its children have the call depth of 0.
    /// In other words the call depth of a call is `call_stack.len() - 1` when
    /// the call is not yet on the call stack.
    call_stack: Vec<Link>,

    /// The pointer to the current node in the call tree. That node represents
    /// the currently running function in the instrumented user code.
    /// The nested calls are added as children to the node pointed by this field.
    ///
    /// Repeats the `self.call_stack.last()` for quick access and brevity
    /// (strictly speaking is not required).
    current_node: Link,

    /// Contains the info necessary for caching the calls before logging.
    /// The cache is used for repeated call or empty loop body removal.
    /// The cache is flushed upon thread context switch or synchronization with the
    /// instrumented user code's own std output and panic hook output.
    caching_info: CachingInfo,

    /// A pointer to an instance implementing the `CoderunNotifiable` trait
    /// (for example a decorator) that gets notified
    /// about changes in the call graph.
    ///
    /// Those notifications can end up in the call logging.
    coderun_notifiable: Rc<RefCell<dyn CoderunNotifiable>>,
}

impl CallGraph {
    /// Creates a new `CallGraph` instance.
    pub fn new(coderun_notifiable: Rc<RefCell<dyn CoderunNotifiable>>) -> Self {
        let pseudo_node = Rc::new(RefCell::new(CallNode::new(ItemKind::Call {
            name: String::from(""),
            param_vals: None,
        })));
        Self {
            current_node: pseudo_node.clone(),
            call_stack: vec![pseudo_node],
            caching_info: CachingInfo::new(/*CacheKind::Call*/),
            coderun_notifiable,
        }
    }

    /// Adds a call (a function or a closure) as a child to the current function in the call graph
    /// (a function on top of the call stack). This results in
    /// * an update to the call graph,
    /// * a potential caching start, if the added child call has the same name as the preceding sibling,
    /// * a potential child call logging, if caching is not active.
    //
    // By the moment of this call (`add_call()`):
    // Log State (logged or cached):
    // parent {
    //     ...
    //     [previous_sibling() { ... }
    //      [// Repeats 99 time(s).]]
    //     new_sibling() {    // The call being added (not yet logged).
    // Call Graph State:
    // `current`: parent (call | loopbody). `call_stack`: {..., parent}.
    // `current.children`: Optional( [..., previous_sibling] ).
    pub fn add_call(&mut self, call_name: &str, param_vals: Option<String>) {
        // Create the new_sibling node:
        let new_sibling = Rc::new(RefCell::new(CallNode::new(ItemKind::Call {
            name: String::from(call_name),
            param_vals: param_vals.clone(),
        })));

        // While the updates have not been done, prepeare the info for later use.
        let siblings_call_depth = self.call_depth();
        let parent = self.current_node.clone();
        let optional_previous_sibling = parent
            .borrow()
            .children
            .last()
            .map(|previous_sibling| previous_sibling.clone());

        // Add new_sibling to the call tree by adding
        // `new_sibling` node to the parent's list of children:
        parent
            // self.current_node // parent
            .borrow_mut()
            .children
            .push(new_sibling.clone());

        // But not yet make the `new_sibling` current.

        // Fork depending on whether the caching is active.
        if !self.caching_is_active() {
            // Caching is not active (ancestry has no loopbodies and calls being cached).
            // There potentially can be a previous sibling (call or non-initial loopbody)
            // with non-zero repeat count.
            // If there is a previous sibling
            //   If a call with the same name then
            //        begin caching starting with the new sibling;
            //   otherwise (a call with different name or a loopbody)
            //        log the previous sibling's repeat count, if non-zero.
            //        Log the call being added.
            // else
            //   log the call.

            // If there is a previous_sibling
            if let Some(previous_sibling) = optional_previous_sibling {
                // If two last siblings differ then
                // * log the previous sibling's repeat count, if non-zero;
                // * log the call.
                let previous_sibling_kind = previous_sibling.borrow().kind.clone();
                let previous_sibling_is_different_call = if let ItemKind::Call { name, .. } =
                    &previous_sibling_kind
                    && name != call_name
                {
                    true
                } else {
                    false
                };
                if previous_sibling_kind.is_loopbody() || previous_sibling_is_different_call {
                    // Log the previous sibling's repeat count, if non-zero.
                    if !previous_sibling
                        .borrow()
                        .repeat_count
                        .non_flushed_is_empty()
                    {
                        self.coderun_notifiable.borrow_mut().notify_repeat_count(
                            siblings_call_depth,
                            &previous_sibling.borrow().kind,
                            previous_sibling.borrow().repeat_count.non_flushed(),
                        );
                        previous_sibling.borrow_mut().repeat_count.mark_flushed();
                    }
                    // Log the call being added.
                    self.coderun_notifiable.borrow_mut().notify_call(
                        self.call_depth(),
                        &call_name,
                        &param_vals,
                    );
                } else {
                    // Otherwise (the previous and the new siblings both are calls with the same name)
                    // begin caching starting with the new sibling.
                    self.caching_info = CachingInfo {
                        model_node: Some(previous_sibling.clone()),
                        node_being_cached: Some(new_sibling.clone()),
                        call_depth: siblings_call_depth,
                    };
                }
            } else {
                // (no previous sibling) Log the call.
                self.coderun_notifiable.borrow_mut().notify_call(
                    self.call_depth(),
                    &call_name,
                    &param_vals,
                );
            }
        } else {
            // Caching is active.
            // If the caching has started at the enclosing loopbody
            // (with optional intermediate enclosing loopbodies in between)
            // and that loopbody is initial then flush
            // (without flushing the notifiable/decorator (TODO: why without?))
            // and stop caching.
            if self.caching_info.model_node.is_none() {
                self.flush(false); // It also stops caching.
            }
        }
        // Add new_sibling to the call stack:
        self.call_stack.push(new_sibling.clone()); // [..., parent] -> [..., parent, new_sibling]

        // Mark that the subsequent calls will be added as children to the new_sibling:
        self.current_node = new_sibling.clone();
    }

    /// Adds a return to the current function in the call graph
    /// (a function on top of the call stack). This results in
    /// * an update in the call graph,
    /// * a potential flush of the returning function's latest child's repeat count, if caching is not active,
    /// * if the returning function's subtree repeats that of the previous sibling,
    ///   then the removal of the returning function's subtree from
    ///   the call graph, increment of the previous sibling's repeat count, caching stop
    ///   (if caching started during the call to this returning function).
    //
    // By the moment of this call (`add_ret()`) the call graph state is:
    // parent { // The call or the loopbody.
    //     [...]
    //
    //     [previous_sibling() {...}
    //      [// previous_sibling() repeats 99 time(s). // Not yet flushed.]] // NOTE: The returning function can get removed and increment this repreat count.
    //     || (or)
    //     [{ // Loop body start.
    //          child() {...}
    //          [// child() repeats 10 time(s).]
    //      } // Loop body end.
    //      [// Loop body repeats 6 time(s). // Flushed.]]
    //
    //     returning_sibling() {        // current. call_stack: [..., parent, returning_sibling].
    //        [... // Nested calls (children).
    //         [// last_child() repeats 9 time(s). // Not yet flushed. ]]
    //     } // The return being handled.
    pub fn add_ret(&mut self, ret_val: Option<String>) {
        // If caching is not active {
        //     Log the repeat count, if non-zero, of the last_child, if present.
        //     Log the return of the returning_sibling.
        // }
        if !self.caching_is_active() {
            let children_call_depth = self.call_depth();
            let returning_sibling = self.call_stack.last().unwrap(); // TODO: Consider `self.call_stack.last().unwrap()` -> `self.current_node` (to get rid of the `unwrap()`).
            returning_sibling.borrow_mut().set_ret_val(ret_val);
            // Log the repeat count, if non-zero, of the last_child, if present:
            if let Some(last_child) = returning_sibling.borrow().children.last()
                && !last_child.borrow().repeat_count.non_flushed_is_empty()
            {
                self.coderun_notifiable.borrow_mut().notify_repeat_count(
                    children_call_depth, // While the returning_sibling is still on the call stack, the call depth reflects the last_child's call_depth.
                    &last_child.borrow().kind,
                    last_child.borrow().repeat_count.non_flushed(),
                );
                last_child.borrow_mut().repeat_count.mark_flushed();
            }
            // Log the return of the returning_sibling:
            match &self.current_node.borrow().kind {
                // TODO: Consider `self.current_node` -> `returning_sibling`.
                ItemKind::Loopbody { .. } => {
                    debug_assert!(
                        false,
                        "FCL Internal Error: Unexpected node in the call tree"
                    )
                }
                ItemKind::Call { name, .. } => {
                    let has_nested_calls = !self.current_node.borrow().children.is_empty();
                    self.coderun_notifiable.borrow_mut().notify_return(
                        children_call_depth - 1, // `- 1`: // The returning_sibling is still on the call stack. The call_depth reflects the children's indent.
                        &name,
                        has_nested_calls,
                        self.current_node.borrow().get_ret_val(),
                    );
                }
            }

            self.current_node.borrow_mut().has_ended = true;

            // Handle the return in the call graph:
            self.call_stack.pop(); // [..., parent, returning_sibling] -> [..., parent].
            self.current_node = self.call_stack.last().unwrap().clone();
        } else {
            // Otherwise (caching is active) {
            // TODO: Consider `self.call_stack.pop().unwrap()` -> `{self.call_stack.pop(); self.current_node}` (to get rid of the `unwrap()`).
            let returning_sibling = self.call_stack.pop().unwrap();
            returning_sibling.borrow_mut().has_ended = true;
            let parent_or_pseudo = self.call_stack.last().unwrap();
            let returning_sibling_call_depth = self.call_depth(); // The returning sibling is not on the call stack already.
            self.current_node = parent_or_pseudo.clone();
            // If there exists a previous_sibling, then {
            if parent_or_pseudo.borrow().children.len() > 1 {
                // The call subtree of the returning_sibling is compared recursively
                // to the previous_sibling's call subtree.
                let previous_sibling_index = parent_or_pseudo.borrow().children.len() - 2;
                let previous_sibling =
                    parent_or_pseudo.borrow().children[previous_sibling_index].clone();
                // If the call subtrees are equal
                if Self::trees_are_equal(
                    &previous_sibling,
                    &returning_sibling,
                    false, // Do not compare the repeat count for the previous_sibling and returning_sibling
                           // (because the returning_sibling's repeat count is always 0 at this stage,
                           // but the previous_sibling's repeat count can be >0),
                           // but compare for the nested calls.
                ) {
                    // the previous sibling's repeat count is incremented,
                    previous_sibling.borrow_mut().repeat_count.inc();
                    // and the returning_sibling's call subtree is removed from the call graph.
                    parent_or_pseudo.borrow_mut().children.pop();
                    // If the previous sibling is the caching model node then caching is over,
                    // i.e. the caching model becomes `None`.
                    if let Some(model_node) = self.caching_info.model_node.as_ref()
                        && model_node.as_ptr() == previous_sibling.as_ptr()
                    {
                        self.caching_info.clear();
                    } // else (caching started at a parent level or above) do nothing.
                } else {
                    // (Caching is active, there is the previous_sibling)
                    // The returning_sibling's and previous_sibling's subtrees differ
                    // (either by name, if caching started at parent or earlier,
                    // or by children, if the previous_sibling is the cahing model node).
                    // If the previous_sibling is the cahing model node then {
                    //     Log the previous_sibling's repeat count, if non-zero,
                    //     Log the subtree of the returning_sibling,
                    //     Stop caching.
                    // }
                    // If the previous_sibling is the caсhing model node then
                    if let Some(model_node) = self.caching_info.model_node.as_ref()
                        && model_node.as_ptr() == previous_sibling.as_ptr()
                    {
                        // Log the previous_sibling's repeat count, if non-zero,
                        if !previous_sibling
                            .borrow()
                            .repeat_count
                            .non_flushed_is_empty()
                        {
                            self.coderun_notifiable.borrow_mut().notify_repeat_count(
                                returning_sibling_call_depth, // Same call depth for the returning and previous siblings.
                                &previous_sibling.borrow().kind,
                                previous_sibling.borrow().repeat_count.non_flushed(),
                            );
                            previous_sibling.borrow_mut().repeat_count.mark_flushed();
                        }
                        // Log the subtree of the returning_sibling.
                        self.flush_tree(&returning_sibling, returning_sibling_call_depth);
                        // Stop caching.
                        self.caching_info.clear();
                    } // else (caching has started at a parent level or above) do nothing, continue caching.
                }
            } // else (no previous_sibling, the returning_sibling is the only child) continue caching, 
            // do nothing. The caching end cannot be detected upon return from the only child.
        }
    }

    /// Adds the loop body start to the call graph.
    // By the moment of this call (`add_loopbody_start()`) the call graph state is:
    // Either `parent() {` or `{ // (Enclosing) Loop body start`.  // `current` node. `call_stack`: [..., parent].
    //     [...]
    //
    //     [{ // Loop body start. // The body of the previous loop.
    //        . . .
    //        previous_loop_child() { .. }  // At least one mandatory function or closure call
    //                                      // (otherwise the loop without calls would not be logged and would be removed from the call graph).
    //         [// previous_loop_child() repeats 2 time(s).]
    //        . . .
    //     } // Loop body end.
    //     // Loop body repeats 5 time(s). // Not yet flushed.]
    //
    //     // || (or)
    //
    //     [previous_sibling() { .. }
    //      [// previous_sibling() repeats 99 time(s). // Not yet flushed.]]
    //
    //     // || (or)
    //
    //     [{ // Loop body start. // Previous iteration(s) of the current loop.
    //        . . .
    //        // TODO: Rename `previous_sibling()` (below) to `current_loop_child()`.
    //        previous_sibling() { .. } // At least one mandatory function or closure call
    //                                  // (otherwise the previous iterations of the current loop would be removed).
    //         [// previous_sibling() repeats 7 time(s).]
    //        . . .
    //     } // Loop body end.
    //     // Loop body repeats 9 time(s). // Not yet flushed.]
    //
    //     // `current.children`: [..., {previous_sibling | loopbody (of the previous or current loop)}].
    //
    //     { // Loop body start that's being handled.
    pub fn add_loopbody_start(&mut self) {
        // Logic.
        // By this moment in the call graph there's
        //  * either no sibling-level node (just parent (who can also be an enclosing_loopbody) or pseudo)
        //  * or a sibling-level node (with potentially non-zero repeat count) of
        //      * either previous loop's last logged loopbody (with a mandatory child call)
        //        (node.kind.ends_the_loop: true)
        //      * or call (function or closure)
        //      * or previous iteration (with a mandatory child call) of the current loop
        //        (node.kind.ends_the_loop: false).
        //
        // If there's a sibling-level node, then memorize the info for flushing its repeat count
        //      ([func name,] repeat count, call depth, etc.).
        // Create the loopbody node, add it to the call graph, make it current.
        // If caching is not active {
        //      Begin caching the newly-added loopbody node
        //      (if it ends up having no (direct or indirect) nested function or closure calls then it will be removed).
        //      If it is the initial loopbody (i.e. the first-most loopbody (iteration) of the loop
        //      or the previous loopbodies (iterations) of the current loop had no nested calls and have been removed)
        //      {
        //          The caching info
        //              * gets NO model_node (that's how the new loopbody node gets marked as initial),
        //              * gets the (current) new loopbody node (to the `node_being_cached`).
        //
        //          // For an initial loopbody the caching will continue until
        //          //     * either the first-most nested function or closure call (directly in this loopbody
        //          //       or indirectly in the nested (initial) loopbodies)
        //          //     * or loopbody end
        //          //     * or flush.
        //          // Upon the first-most nested function or closure call the cache will be flushed 
        //          // (beginning with the current loopbody start,
        //          // through the nested loopbodies' starts, and ending with the first-most nested function or closure call/start),
        //          // caching will end, and execution will continue.
        //          // Upon initial loopbody end,
        //          // if the loopbody ends up having no (direct or indirect) nested function or closure calls 
        //          //          (and caching didn't end upon flush), then
        //          //     the childless loopbody will get removed and the subsequent loopbody, if any, of the current loop
        //          //     will be marked later as initial.
        //          // Otherwise (the inital loopbody has (logged) children) the last child's repeat count
        //          // (which will be the only non-flushed thing) will get flushed, if non-zero. Plus the loop body end.
        //      } otherwise (it is non-initial loopbody) {
        //          caching info
        //            * gets the model_node pointing to the previous loopbody of the current loop
        //              (thus the new loopbody node gets marked as non-initial)
        //            * gets the (current) new loopbody node (to the `node_being_cached`). // NOTE: This line
        //                  // is duplicated in `if` and `else` part of the current comment, but not in the code.
        //
        //          // For a non-initial loopbody the caching will continue until {loopbody end or `flush()`}, where the loopbody,
        //          //     * if will not have nested calls, will be removed,
        //          //     * otherwise will be analized in a similar way as repeted function call,
        //          //       i.e. will be compared to the previous iteration's loopbody, and,
        //          //         * if equal, will get removed, incrementing the repeat count for the previous loopbody,
        //          //         * otherwise (will differ) will cause previous loopbody's repeat count flush and one's own subtree flush.
        //      }
        // } Otherwise (caching is active) {
        //      (Caching has started at the parent or earlier because if the previous node is a loopbody
        //      and cahing started at it, then caching ended upon its end)
        //
        //      Do nothing (after creating and adding the new loopbody to the graph, continue caching).
        // }

        // Implementation.
        // If there's a sibling-level node, then memorize the info for flushing its repeat count
        // ([function or closure name,] repeat count, call depth, etc.).
        let siblings_call_depth = self.call_depth(); // The parent's children call depth.
        let previous_sibling_node_info = self
            .current_node // parent
            .borrow()
            .children
            .last()
            .map(|ref_last_child| ref_last_child.clone());

        // Create the loopbody node,
        let new_loopbody_node = Rc::new(RefCell::new(CallNode::new(ItemKind::Loopbody {
            ends_the_loop: false,
        })));

        // add it to the call graph (by adding to the parent's list of children),
        self.current_node // parent
            .borrow_mut()
            .children
            .push(new_loopbody_node.clone());
        // and call stack,
        self.call_stack.push(new_loopbody_node.clone()); // [..., parent] -> [..., parent, new_loopbody_node]

        // make it current (the subsequent calls will be added as children to the new node).
        self.current_node = new_loopbody_node.clone();

        // If caching is not active {
        if !self.caching_is_active() {
            // Begin caching the newly-added loopbody node:
            // If it is the initial loopbody {
            // (i.e.
            //  * no previous sibling (PS)
            //  * or PS is a call
            //  * or PS is a loop body of a previous loop)
            let initial = match &previous_sibling_node_info {
                None => true,
                Some(previous_sibling) => match previous_sibling.borrow().kind {
                    ItemKind::Call { .. } => true,
                    ItemKind::Loopbody { ends_the_loop } => ends_the_loop,
                },
            };
            // then the caching info
            //     * gets NO model_node (thus the new loopbody node is marked as initial),
            //     * gets the new loopbody node.
            // } otherwise (it is non-initial loopbody) {
            //   caching info
            //     * gets the model node pointing to the previous loopbody/iteration of the
            //       current loop (thus the new loopbody node is marked as non-initial)
            //     * gets new loopbody node.
            // }
            self.caching_info = CachingInfo {
                model_node: if initial {
                    None
                } else {
                    previous_sibling_node_info
                },
                node_being_cached: Some(new_loopbody_node.clone()),
                call_depth: siblings_call_depth,
            };
        }
        // Otherwise (caching is active)
        //   Do nothing (after creating and adding the new loopbody to the graph, continue caching).
    }

    /// Adds the loop body end to the call graph.
    // By the moment of this call (`add_loopbody_end()`) the call graph state is:
    // parent() {
    //      . . .
    //      [{ // Loop body start.  // Optional previous loop.
    //          . . .
    //          child() { ... } // At least one function or closure call in the loopbody.
    //      } // Loop body end.
    //      [// Repeats. // Flushed]]
    //      [{ // Loop body start.   // Optional previous iterations of the current loop.
    //          . . .
    //          child() { ... } // At least one function or closure call in the loopbody.
    //          [// child() repeats 7 time(s).]
    //      } // Loop body end.
    //      // Loop body repeats 6 time(s). // Not yet flushed. ]
    //      { // Loop body start.   // `current`. `call_stack`: [..., parent, loopbody]. `current.children`: loopbody's nested calls, can be empty.
    //          . . .
    //          [child() { ... }
    //          // child() repeats 3 time(s). // Not yet flushed]
    //      } // Loop body end.     // The end being handled.
    pub fn add_loopbody_end(&mut self) {
        // Logic V2 (with loop_end notification).
        //
        // If the ending loop body has no children {
        //      If caching is inactive { // The loop body's start has been flushed upon thread context switch.
        //          Log the loop body's end.
        //      } else (caching is active) if the loop body is the `node_being_cached` {
        //          Stop caching.
        //      }
        //      Remove the ending childless loop body from the call graph (from the parent's list of children,
        //      pop from the call stack, redirect `current` to parent; // NOTE: In the impl this line is not duplicated.
        //      and it is not in cache (since caching is either inactive, or stopped, or started at parent or earlier)).
        //
        //      return;
        // }
        //
        // // Has child(ren).
        // If caching is inactive (stopped upon child or flush) {
        //      Log the last child's repeat count, if non-zero.
        //      Log the loop body end.
        // }
        //
        //  Mark the loop body as `ended`.  // The thread context switch can happen immediately before or after the (loop body's) end.
        //                                  // Upon `flush()`,
        //                                  //  * if the loop body is not marked as `ended`, then the loop body's end is not logged;
        //                                  //  * if marked as `ended`, then is logged.
        //
        // // If there is a (current loop's) previous iteration's loop body (with optionally non-zero repeat count)
        // If the previous sibling exists && is a loop body && doesn't end the loop {
        //     Compare the ending loop body's subtree with the previous iteration's loop body's subtree.
        //     If equal {
        //         Remove the ending loop body from the call graph (now from the parent's list of children,
        //             later - from the call stack, current;
        //             and it is already not in the cache (since the caching either stopped upon child
        //             or started at parent or earlier)).
        //         Increment the repeat count of the previous iteration's loop body.
        //         Stop caching (unless caching has started at parent or earlier).
        //     }
        //     // Otherwise (differs from the previous iteration's loop body's subtree)
        //     // Do nothing. The ending loop body is already logged (if caching is inactive)
        //     // and stays in the call graph (in the parent's list of children).
        // }
        // // Otherwise { // No previous sibling || it is a call || ends the (previous) loop. // No previous iteration's loop body.
        // //  Do nothing. The ending loop body is already logged (if caching is inactive) and stays in the call graph (in the parent's list of children).
        //
        // Pop (the ending loop body) from the call stack, redirect `current` to parent. // NOTE: In the impl this line is not duplicated.

        // Implementation.

        let ending_loopbody = self.current_node.clone();
        let children_call_depth = self.call_depth();

        // Go back to parent:
        self.call_stack.pop(); // [.., parent_or_pseudo, ending_loopbody] -> [.., parent_or_pseudo].
        let parent_or_pseudo = match self.call_stack.last() {
            None => panic!("FCL Internal Error: Unexpected bottom of the call stack"), // Must never get here. The program state
            // is unexpected, panicking ASAP. TODO: Consider logging the error and returning (to continue)
            // instead of panicking the thread.
            Some(parent_or_pseudo) => parent_or_pseudo.clone(),
        };
        self.current_node = parent_or_pseudo.clone();

        let returning_loopbody_call_depth = self.call_depth();

        // If the ending loop body has no children {
        if ending_loopbody.borrow().children.is_empty() {
            // If caching is inactive { // The childless loop body's start has been flushed upon thread context switch.
            if !self.caching_is_active() {
                // Log the loop body's end.
                self.coderun_notifiable
                    .borrow_mut()
                    .notify_loopbody_end(returning_loopbody_call_depth);
            }
            // } else if the loop body is the `node_being_cached` {
            else if let Some(node_being_cached) = &self.caching_info.node_being_cached
                && node_being_cached.as_ptr() == ending_loopbody.as_ptr()
            {
                // Stop caching.
                self.caching_info.clear();
            }
            // Remove the ending childless loop body from the call graph (from the parent's list of children,
            parent_or_pseudo.borrow_mut().children.pop();

            // pop from the call stack, redirect `current` to parent;
            // Is already done in the beginning of the function.

            // and it is not in cache (since caching is either inactive, or stopped, or started at parent or earlier)).

            return;
        }

        // // Has child(ren). Caching is either inactive (stopped upon child) or started at parent level or earlier.
        // If caching is inactive (stopped upon child or flush) {
        if !self.caching_is_active() {
            // Log the last child's repeat count, if non-zero.
            if let Some(last_child) = ending_loopbody.borrow().children.last()
                && !last_child.borrow().repeat_count.non_flushed_is_empty()
            {
                self.coderun_notifiable.borrow_mut().notify_repeat_count(
                    children_call_depth,
                    &last_child.borrow().kind,
                    last_child.borrow().repeat_count.non_flushed(),
                );
                last_child.borrow_mut().repeat_count.mark_flushed();
            }
            // Log the loop body end.
            self.coderun_notifiable
                .borrow_mut()
                .notify_loopbody_end(returning_loopbody_call_depth);
        }
        // Otherwise nothing, go on.

        //  Mark the loop body as `ended`.  // The thread context switch can happen immediately before or after the (loop body's) end.
        //                                  // Upon `flush()`,
        //                                  //  * if the loop body is not marked as `ended`, then the loop body's end is not logged;
        //                                  //  * if marked as `ended`, then is logged.
        ending_loopbody.borrow_mut().has_ended = true;

        // // If there is a (current loop's) previous iteration's loop body (with optionally non-zero repeat count)
        // If the previous sibling exists...
        let sibling_count = parent_or_pseudo.borrow().children.len();
        if sibling_count >= 2 {
            let previous_sibling = parent_or_pseudo.borrow().children[sibling_count - 2].clone();

            // && is a loop body && doesn't end the loop {
            let previous_sibling_kind = previous_sibling.borrow().kind.clone(); // NOTE: Cloning to enable 
            // the subsequent `previous_sibling.borrow_mut()`.
            if let ItemKind::Loopbody { ends_the_loop } = previous_sibling_kind
                && !ends_the_loop
            {
                // Compare the ending loop body's subtree with the previous iteration's loop body's subtree.
                // If equal {
                if Self::trees_are_equal(&ending_loopbody, &previous_sibling, false) {
                    // Remove the ending loop body from the call graph (from the parent's list of children,
                    parent_or_pseudo.borrow_mut().children.pop();
                    // and from {the call stack, current} - already done in the beginning of the function;
                    // and it is already not in the cache (since the caching either stopped upon child
                    // or started at parent or earlier)).

                    // Increment the repeat count of the previous iteration's loop body.
                    previous_sibling.borrow_mut().repeat_count.inc();

                    // Stop caching (unless caching has started at parent or earlier).
                    if let Some(node_being_cached) = &self.caching_info.node_being_cached
                        && node_being_cached.as_ptr() == ending_loopbody.as_ptr()
                    {
                        self.caching_info.clear()
                    }
                }
                // Otherwise (differs from the previous iteration's loop body's subtree)
                // Do nothing. The ending loop body is already logged (if caching is inactive)
                // and stays in the call graph (in the parent's list of children).
            } // else nothing.
        }
        // Otherwise {      // No previous sibling || it is a call || ends the (previous) loop. // No previous iteration's loop body.
        // Do nothing. The ending loop body is already logged (if caching is inactive)
        // and stays in the call graph (in the parent's list of children).

        // Pop (the ending loop body) from the call stack, redirect `current` to parent.
        // Is already done in the beginning of the function.
    }

    pub fn add_loop_end(&mut self) {
        // When the loop ends
        // If the current node (parent) has children and the last child is a loop body { // Previous sibling is a loop body.
        //     If the last loop body is not marked as `ends_the_loop: true` (it is the last survived loop body of the ending loop) {
        //         Flush its repeat count, if non-zero.
        //         Mark it as `ends_the_loop: true`. // Based on this the subsequent loop body, if any,
        //                                           // will be interpreted as the first loop body of the next loop.
        //     }
        //     otherwise (the last loopbody is already marked as `ends_the_loop: true`) {
        //         // The last loopbody belongs to the previous loop that has already ended.
        //         // The currently ending loop has no survived loop bodies (they are all empty and have been removed).
        //         Do nothing.
        //     }
        // }
        // Otherwise (the current node (parent) has no children or the last child is not a loop body) {
        //     // I.e. the ending loop has no calls and all of its loop bodies are removed, and {no previous sibling or it's not a loop body}.
        //     Do nothing.
        // }

        // The logic below is partially reversed compared to the comment above.

        // If the current node (parent) has no children or the last child is not a loop body
        // then return.
        let Some(last_child) = self
            .current_node // parent
            .borrow()
            .children
            .last()
            .map(|ref_rc| ref_rc.clone())
        else {
            return;
        };
        let mut last_child_borrow_mut = last_child.borrow_mut();
        let last_child_kind = &mut last_child_borrow_mut.kind;
        if last_child_kind.is_call() {
            return;
        }

        // The last child is a loop body.
        // If it doesn't end the loop then:
        if let ItemKind::Loopbody { ends_the_loop } = last_child_kind
            && !*ends_the_loop
        {
            // Mark it as `ends_the_loop: true`.
            *ends_the_loop = true; // NOTE: Is placed first to release this `&mut` ASAP, 
            // such that the `last_child_kind` stays the only `&mut`.

            let last_child_kind = last_child_kind.clone();

            // Flush its repeat count, if non-zero.
            if !last_child_borrow_mut.repeat_count.non_flushed_is_empty() {

                self.coderun_notifiable.borrow_mut().notify_repeat_count(
                    self.call_depth(), // The parent's children call depth.
                    &last_child_kind,
                    last_child_borrow_mut.repeat_count.non_flushed(),
                );
                last_child_borrow_mut.repeat_count.mark_flushed();
            }
        }
        // else (the last loop body ends the loop)
        //  Do nothing.
    }

    /// Flushes the data cached in the call graph.
    /// Flushing is done upon
    /// * thread context switch,
    /// * log synchronization with the instrumented user code's own stdoutput (`stdout` and `stderr`)
    /// and panic output.
    ///
    /// **Parameters**
    /// * `flush_notifiable`: Flush the notifiable. For example, if the notifiable is the call-like decorator,
    /// and the flushed log ends with `f() {` then the decorator needs to output `"\n"` before the other entity
    /// (the other thread, stdoutput, panic handler) starts logging.
    // TODO: Reformat the {param doc-comment} according to standards.
    pub fn flush(&mut self, flush_notifiable: bool) {
        // TODO: Consider flush_notifiable -> flush_the_notifiable
        // If call caching is active:
        // * the caching model_node - the previous sibling node (for non-loopbody caching case) - can have a non-zero non-flushed repeat count
        // * and the subsequent (current) sibling (with its children) is being added to the call graph (is being cached).
        if let Some(caching_model_node) = self.caching_info.model_node.as_ref() {
            // Log the caching_model_node's repeat count, if non-zero,
            // Log the subtree of the node bing cached.
            // Stop caching (`caching_model_node = None`).

            // If the caching model node has a non-flushed repeat count
            if !caching_model_node
                .borrow()
                .repeat_count
                .non_flushed_is_empty()
            {
                // then flush the repeat count.
                self.coderun_notifiable.borrow_mut().notify_repeat_count(
                    self.caching_info.call_depth,
                    &caching_model_node.borrow().kind, //name,
                    caching_model_node.borrow().repeat_count.non_flushed(),
                );
                caching_model_node.borrow_mut().repeat_count.mark_flushed();
            }
            // Log the subtree of the subsequent (current) node being cached:
            if let Some(cached_sibling) = self.caching_info.node_being_cached.take() {
                self.flush_tree(&cached_sibling, self.caching_info.call_depth);
            }
        } else if let Some(node_being_cached) = self.caching_info.node_being_cached.clone() {
            // The initial loopbody (with optional nested initial loopbodies) is being cached.
            self.flush_tree(&node_being_cached, self.caching_info.call_depth);
            // TODO: Double-check the scenario when upon thread context switch the initial loopbody gets flushed,
            // subsequently it has no nested calls, but if its beginning is flushed then its end must be flushed too,
            // after which the empty loopbody must be removed from the call graph.
        } else {
            // Caching is inactive.
            // The latest sibling can have a non-zero non-flushed repeat count.
            // The `self.current` points to the parent or pseudo.
            if let Some(latest_sibling) = self.current_node.borrow().children.last()
                && !latest_sibling.borrow().repeat_count.non_flushed_is_empty()
            {
                self.coderun_notifiable.borrow_mut().notify_repeat_count(
                    self.caching_info.call_depth,
                    &latest_sibling.borrow().kind, //.name,
                    latest_sibling.borrow().repeat_count.non_flushed(),
                );
                latest_sibling.borrow_mut().repeat_count.mark_flushed();
            }
        }
        if flush_notifiable {
            self.coderun_notifiable.borrow_mut().notify_flush();
        }

        // Stop caching.
        self.caching_info.clear();
    }

    /// Returns one less than the length of the call stack, `0` when the mandatory pseudonode is the only one on the call stack.
    ///
    /// E.g. before adding `main()` returns `0`. After adding `main()` returns `1`.
    //
    // TODO: Consider call_depth -> children_call_depth.or
    // current_children_call_depth (the call depth should be calculated either for a specified node or be "current").
    pub fn call_depth(&self) -> usize {
        let call_depth = self.call_stack.len();
        debug_assert!(call_depth >= 1); // Pseudo-node.
        call_depth - 1
    }

    /// Tells if cahing is active.
    pub fn caching_is_active(&self) -> bool {
        self.caching_info.is_active()
    }

    /// Tells if the two subtrees of the call tree are equal recursively,
    /// including the nested calls (children), their names, order, repeat counts,
    /// but not including the parameters and return values.
    /// #### Parameters
    /// * `a` specifies the first subtree to compare, typically the caching model node
    ///   with potentially non-zero repeat count.
    /// * `b` specifies the second subtree to compare, typically the node being cached,
    ///   with always-zero repeat count.
    /// * `compare_root_repeat_count` tells to compare the repeat count for the subtree roots.
    ///   Expected to be `false` for the caching model node and the node being cached, but `true`
    ///   for their nested calls (children).
    // TODO: Reformat the params doc comments according to the standards
    // (ideally such that the param names are not mentioned in the doc comments (no dup),
    // otherwise renaming gets complicated).
    fn trees_are_equal(a: &Link, b: &Link, compare_root_repeat_count: bool) -> bool {
        let a = a.borrow();
        let b = b.borrow();
        match &a.kind {
            ItemKind::Call { name: a_name, .. } => {
                match &b.kind {
                    ItemKind::Call { name: b_name, .. } => {
                        if a_name != b_name {
                            return false; // Calls with differnt names.
                        }
                    }
                    ItemKind::Loopbody { .. } => return false, // a: Call, b: Loopbody.
                }
            }
            ItemKind::Loopbody { .. } => match &b.kind {
                ItemKind::Call { .. } => return false, // a: Loopbody, b: Call.
                ItemKind::Loopbody { .. } => {}        // Both are Loopbody, continue comparing.
            },
        }

        if a.children.len() != b.children.len() {
            return false;
        }
        if compare_root_repeat_count && a.repeat_count != b.repeat_count {
            return false;
        }
        for index in 0..a.children.len() {
            if !Self::trees_are_equal(&a.children[index], &b.children[index], true) {
                return false;
            }
        }
        true
    }

    /// Flushes to the log the call subtree rooted at the node passed as an argument.
    ///
    /// Traverses the call subtree recursively and calls the notification callbacks,
    /// thus loggign the subtree.
    fn flush_tree(&mut self, current_node: &Link, call_depth: usize) {
        let mut current_node = current_node.borrow_mut();
        let item_children = &current_node.children;

        match &current_node.kind {
            ItemKind::Call { name, param_vals } => {
                self.coderun_notifiable
                    .borrow_mut()
                    .notify_call(call_depth, name, param_vals);
            }
            ItemKind::Loopbody { .. } => self
                .coderun_notifiable
                .borrow_mut()
                .notify_loopbody_start(call_depth),
        }

        // Traverse children recursively:
        for child in item_children {
            self.flush_tree(child, call_depth + 1);
        }

        // If the item has returned/ended:
        if current_node.has_ended {
            // The return/end:
            match &current_node.kind {
                ItemKind::Call { name, .. } => {
                    let has_nested_calls = !item_children.is_empty();
                    self.coderun_notifiable.borrow_mut().notify_return(
                        call_depth,
                        name,
                        has_nested_calls,
                        current_node.get_ret_val(),
                    );
                }
                ItemKind::Loopbody { .. } => self
                    .coderun_notifiable
                    .borrow_mut()
                    .notify_loopbody_end(call_depth),
            }

            // The repeat count:
            if !current_node.repeat_count.non_flushed_is_empty() {
                self.coderun_notifiable.borrow_mut().notify_repeat_count(
                    call_depth,
                    &current_node.kind, //name,
                    current_node.repeat_count.non_flushed(),
                );
                current_node.repeat_count.mark_flushed();
            } // else (no non-flushed repeat count) do nothing.
        } // else (hasn't returned) do nothing.
    }
}
