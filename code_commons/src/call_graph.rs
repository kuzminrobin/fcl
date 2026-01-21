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
    Loopbody,
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
        let optional_previous_sibling =
            parent.borrow().children.last().map(|previous_sibling| previous_sibling.clone());

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
    // Either `parent() {` or `{ // (TODO: Enclosing?) Loop body start`.  // `current`. `call_stack`: [..., parent]. `current.children`: [..., {previous_sibling | [enclosing_?]loopbody}]. // TODO: Smth is wrong about `current.children`, should be just optional previous_sibling: `current.children`: [..., [previous_sibling] ]
    //     [...]
    //     [{ // Loop body start. // The body of the previous loop.
    //        . . .
    //        loop_nested_sibling() { .. }  // At least one mandatory function call (otherwise the loop would not be logged and would be removed from the call graph).
    //         [// loop_nested_sibling() repeats 2 time(s).]
    //        . . .
    //     } // Loop body end.
    //     // Loop body repeats 5 time(s). // Not yet flushed.]
    //     // || (or)
    //     [previous_sibling() { .. }
    //      [// previous_sibling() repeats 99 time(s). // Not yet flushed.]]
    //     // || (or)
    //     [{ // Loop body start. // Previous iteration(s) of the current loop.
    //        . . .
    //        previous_sibling() { .. } // At least one mandatory function call (otherwise the previous loop iterations would be removed).
    //         [// previous_sibling() repeats 7 time(s).]
    //        . . .
    //     } // Loop body end.
    //     // Loop body repeats 9 time(s). // Not yet flushed.]
    //     { // Loop body start that's being handled.
    pub fn add_loopbody_start(&mut self) {
        // Logic.
        // By this moment in the call graph there's
        //  * either no sibling-level node (just parent (who can also be an enclosing_loopbody) or pseudo)
        //  * or a sibling-level node (with potentially non-zero repeat count) of
        //      * either previous loop's last logged loopbody (with mandatory nested calls)
        //        (node.kind.ended_the_loop: true) // TODO: Is this line still applicable?
        //      * or previous_sibling() call (function or closure)
        //      * or previous iteration (with mandatory nested calls) of the current loop
        //        ( node.kind.ended_the_loop: false ).  // TODO: Is this line still applicable?
        //
        // If there's a sibling-level node, then memorize the info for flushing its repeat count
        // ([name,] repeat count, call depth, etc.).
        // Create the loopbody node, add it to the call graph, make it current.
        // If caching is not active {
        //      If the previous sibling node exists and is NOT the previous iteration of the current loop, then {
        //          Log the repeat count, if non-zero, of the previous sibling-level node.
        //      }
        //      Begin caching the newly-added loopbody node:
        //      If it is the initial loopbody (i.e. the first-most loopbody/iteration of the loop
        //      or the previous loopbodies/iterations had no nested calls and have been removed)
        //      {
        //          then the caching info
        //              * gets NO model_node (that's how the new loopbody node gets marked as initial),
        //              * gets the new loopbody node.
        //
        //          (For an initial loopbody the caching will continue until either the first-most nested call
        //          (directly in the loopbody or indirectly in the nested loopbodies) or until loopbody end.
        //          Upon the first-most nested function call the cache will be flushed (starting from the current loopbody,
        //          through the intermediate nested loopbodies, and ending after the first-most nested function call),
        //          caching will end, and execution will continue.
        //          Upon initial loopbody end,
        //          if the loopbody will have no children, then // TODO: Consider -> "if caching is still active (i.e. no logged nested function or closure call)".
        //              the loopbody will get removed and the subsequent loopbody, if any, of the current loop
        //              will be marked later as initial.
        //          Otherwise (the inital loopbody has (logged) children) the last child's repeat count
        //          (which will be the only non-flushed thing) will get flushed, if non-zero)
        //      } otherwise (it is non-initial loopbody) {
        //          caching info
        //            * gets the model node pointing to the previous loopbody/iteration of the
        //              current loop (thus the new loopbody node gets marked as non-initial)
        //            * gets new loopbody node.
        //
        //          (For a non-initial loopbody the caching will continue until loopbody end, where
        //          the loopbody, if will not have nested calls, will be removed, otherwise will be analized
        //          in a similar way as repeted function call,
        //          i.e. will be compared to the previous loopbody, and,
        //          if equal, will get removed incrementing the repeat count for the previous loopbody,
        //          otherwise (will differ) will cause previous loopbody's repeat count flush and one's own subtree flush)
        //      }
        // } Otherwise (caching is active) {
        //      (Caching has started at the parent or earlier because if the previous node is a loopbody
        //      and cahing started at it, then caching ended upon loopbody end)
        //
        //      Do nothing (after creating and adding the new loopbody to the graph, continue caching).
        // }

        // Implementation.
        // If there's a sibling-level node, then memorize the info for flushing its repeat count
        // ([name,] repeat count, call depth, etc.).
        let call_depth = self.call_depth();
        let mut previous_sibling_node_info = None;
        // TODO: Consider map().
        // let mut previous_sibling_node_info = self.current_node.borrow().children.last().map(|ref_last_child| ref_last_child.clone());
        if let Some(previous_sibling) = self.current_node.borrow().children.last().clone() {
            previous_sibling_node_info = Some(previous_sibling.clone());
        }

        // Create the loopbody node,
        let new_loopbody_node = Rc::new(RefCell::new(CallNode::new(ItemKind::Loopbody)));

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
            // If the previous sibling node exists and is NOT loopbody (NOT the previous iteration of the current loop,
            // and NOT the last iteration of the previous loop) then {
            if let Some(rc_previous_sibling) = previous_sibling_node_info.as_ref()
                && !rc_previous_sibling.borrow().kind.is_loopbody()
            {
                // Log the repeat count, if non-zero, of the previous sibling-level node.
                let mut previous_sibling = rc_previous_sibling.borrow_mut();
                if !previous_sibling.repeat_count.non_flushed_is_empty() {
                    self.coderun_notifiable.borrow_mut().notify_repeat_count(
                        call_depth,
                        &previous_sibling.kind,
                        previous_sibling.repeat_count.non_flushed(),
                    );
                    previous_sibling.repeat_count.mark_flushed();
                }
            }
            // Begin caching the newly-added loopbody node:
            // If it is the initial loopbody (i.e. the first-most loopbody/iteration of the loop
            // or the previous loopbodies/iterations had no nested function calls and have been removed)
            let previous_iteration_loopbody =
                previous_sibling_node_info.and_then(|previous_sibling| {
                    if previous_sibling.borrow().kind.is_loopbody() {
                        Some(previous_sibling)
                    } else {
                        None
                    }
                });
            // then the caching info
            //     * gets NO model_node (new loopbody node marked as initial),
            //     * gets the new loopbody node.
            // } otherwise (it is non-initial loopbody) {
            //   caching info
            //     * gets the model node pointing to the previous loopbody/iteration of the
            //       current loop (new loopbody node marked as non-initial)
            //     * gets new loopbody node.
            self.caching_info = CachingInfo {
                // kind: CacheKind::Loopbody {
                //     initial: previous_iteration_loopbody.is_none(),
                // },
                model_node: previous_iteration_loopbody,
                node_being_cached: Some(new_loopbody_node.clone()),
                call_depth,
            };
        }
        // Otherwise (caching is active)
        //   Do nothing (after creating and adding the new loopbody to the graph, continue caching).
    }

    /// Adds the loop body end to the call graph.
    // By the moment of this call (`add_loopbody_end()`) the call graph state is:
    // parent() {
    //      . . .
    //      [{ // Loop body start.   // Possible previous iterations of the current loop.
    //          . . .
    //          child() { ... } // At least one function call in the loopbody.
    //          [// child() repeats 7 time(s).]
    //      } // Loop body end.
    //      // Loop body repeats 6 time(s). // Not yet flushed? ]
    //      { // Loop body start.   // `current`. `call_stack`: [..., parent, loopbody]. `current.children`: loopbody's nested calls, can be empty.
    //          . . .
    //          [child() { ... }
    //          // child() repeats 3 time(s). // Not yet flushed]
    //      } // Loop body end.     // The end being handled.
    pub fn add_loopbody_end(&mut self) {
        // Logic. // TODO: Consider re-writing from scratch (or reviewing to be 100% correct) after writing the tests for each case.
        // If no nested function calls {
        //      If caching is active and this loopbody is a node being cached then stop caching.
        //      Remove this loopbody (from call {graph, stack}) and make parent a current node.
        // }
        // otherwise (there are nested calls) {
        //      (Here, caching could only start at a parent or earlier
        //      or the current node can be being cached if the node is not initial loopbody)
        //      If caching is not active
        //          Log the last child's repat count, if non-zero.
        //          Log the current loopbody's end.
        //      otherwise (caching is active)
        //          Do noithing here, move on.
        //      Mark loopbody (but not the whole loop) as ended (node.ended).
        //      If there is a previous loopbody (previous iteration of the current loop)
        //          Compare this loopbody to the previous loopbody.
        //          If equal
        //              Remove this loopbody from call graph (thus making parent a current node).
        //              Increment the previous loopbody overall repeat count.
        //              If caching
        //                  If the current loopbody is the node being cached // TODO: Consider doing this before removing the current loopbody?
        //                      Stop caching.
        //              // { (An update: Bug "Redundant repeat count logged for loop bodies")
        //              Else (not caching)
        //                  Increment the previous loopbody flushed repeat count.
        //              // } (An update: Bug "Redundant repeat count logged for loop bodies").
        // 
        //              //Return (in the current implementation there is no `return` here).
        //          Otherwise (differs)
        //              If caching and the current node is the one being cached
        //                  Flush and stop caching.
        //      Otherwise (no previous iteration of the current loop)
        //          Do nothing here, move on.
        //      Remove this loopbody from the call stack (thus making parent a current node). // Remove from stack, not from graph.
        // }

        // Implementation.
        // If no nested function calls {
        if self.current_node.borrow().children.is_empty() {
            // If caching is active and this loopbody was a node being cached
            if self.caching_is_active() {
                if let Some(node_being_cached) = self.caching_info.node_being_cached.as_ref()
                    && node_being_cached.as_ptr() == self.current_node.as_ptr()
                {
                    // then stop caching.
                    self.caching_info.clear();
                }
            }
            // Remove this loopbody (from call {graph, stack}) and make parent a current node.
            self.call_stack.pop();
            if let Some(parent) = self.call_stack.last() {
                parent.borrow_mut().children.pop();
                self.current_node = parent.clone();
            } else {
                debug_assert!(false, "FCL Internal Error: Unexpected bottom of call stack");
            }
        } else {
            // Otherwise (there are nested calls) {
            // (Here, caching could only start at a parent or earlier
            // or the current node can be being cached if the node is not initial loopbody)

            let child_call_depth = self.call_depth();
            match self.call_stack.pop() {
                // self.current_node still points to the ending_loopbody.
                // Popped the ending (current) loopbody's node from the call stack, but not from graph
                // (parent or pseudo stays on top of the call stack).
                None => debug_assert!(false, "FCL Internal Error: Unexpected bottom of call stack"), // TODO: Consider postponing the panic until the mutex release.
                Some(ending_loopbody) => {
                    debug_assert!( // TODO: Consider postponing the panic until the mutex release.
                        ending_loopbody.borrow().kind.is_loopbody(),
                        "Unexpected item kind in the call stack"
                    );

                    match self.call_stack.last() {
                        // self.current_node still points to the ending_loopbody.
                        None => {
                            debug_assert!(false, "FCL Internal Error: Unexpected call stack bottom") // TODO: Consider postponing the panic until the mutex release.
                        }
                        Some(parent_or_pseudo) => {
                            let parent_or_pseudo = parent_or_pseudo.clone();
                            // If caching is not active
                            if !self.caching_is_active() {
                                // Flush the last child's repat count, if non-zero.
                                if let Some(last_child) = self.current_node.borrow().children.last() // TODO: Consider self.current_node -> ending_loopbody.
                                    && !last_child.borrow().repeat_count.non_flushed_is_empty()
                                {
                                    self.coderun_notifiable.borrow_mut().notify_repeat_count(
                                        child_call_depth,
                                        &last_child.borrow().kind,
                                        last_child.borrow().repeat_count.non_flushed(),
                                    );
                                    last_child.borrow_mut().repeat_count.mark_flushed();
                                }
                                // Log the current loopbody's end.
                                self.coderun_notifiable
                                    .borrow_mut()
                                    .notify_loopbody_end(self.call_depth()); // parent_or_pseudo is on top of the call stack, so the call_depth corresponds to the ending_loopbody.
                            }
                            // otherwise (caching is active, starting with parent or earlier (TODO: What about current node being cached?))
                            //     Do noithing here, continue caching, move on.

                            // Mark loopbody (but not the whole loop) as ended.
                            self.current_node.borrow_mut().has_ended = true; // TODO: Consider self.current_node -> ending_loopbody.

                            // If there is a previous loopbody (previous iteration of the current loop
                            // or the last iteration of the previous loop ;-)
                            let siblings_count = parent_or_pseudo.borrow().children.len();
                            if siblings_count > 1 {
                                let previous_node =
                                    parent_or_pseudo.borrow().children[siblings_count - 2].clone();
                                if previous_node.borrow().kind.is_loopbody() {
                                    // Compare this loopbody to the previous loopbody.
                                    // If equal
                                    if Self::trees_are_equal(
                                        &previous_node,
                                        &ending_loopbody,
                                        false,
                                    ) {
                                        // Remove this loopbody (from call {graph, stack}) and make parent a current node.
                                        parent_or_pseudo.borrow_mut().children.pop();
                                        // (Removing from call stack is already done above in `match self.call_stack.pop()`)
                                        // Increment the previous loopbody repeat count.
                                        previous_node.borrow_mut().repeat_count.inc();

                                        if self.caching_is_active() {
                                            // If the current loopbody is the node being cached,
                                            if let Some(node_being_cached) =
                                                &self.caching_info.node_being_cached
                                                && node_being_cached.as_ptr()
                                                    == ending_loopbody.as_ptr()
                                            {
                                                // Stop caching.
                                                self.caching_info.clear();
                                            }
                                        } else {
                                            previous_node.borrow_mut().repeat_count.mark_flushed();
                                        }
                                    }
                                    // Otherwise (differs)
                                    // If caching, and the current node is the one being cached
                                    if let Some(node_being_cahed) =
                                        &self.caching_info.node_being_cached
                                        && ending_loopbody.as_ptr() == node_being_cahed.as_ptr()
                                    {
                                        // Flush (including the notifiable) and stop caching.
                                        self.flush(true); // It also stops caching.
                                    }
                                }
                                // TODO: Consider replacing
                                //   Otherwise (no previous iteration of the current loop, previous node is a call or a different loop.)
                                // with
                                //   Otherwise (no previous loop_body (of the current or previous loop), previous node is a call)
                                //     Do nothing here, move on.
                            }
                            // else (no previous sibling node)
                            //   Do nothing, move on.

                            // Remove this loopbody from call stack (is already done above in `match self.call_stack.pop()`)
                            // and make parent a current node.
                            self.current_node = parent_or_pseudo.clone();
                        } // Some()
                    } // match
                } // Some()
            } // match
        }
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

    /// Returns the call depth (`0` when the pseudonode only is in)
    /// in the call graph for the currently running function's children.
    /// E.g. before adding `main()` the call depth is `0` (the call depth of pseudonode's children).
    /// After adding `main()` the call depth is `1` (the call depth of `main()`'s children).
    //
    // TODO: Consider call_depth -> children_call_depth.
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
