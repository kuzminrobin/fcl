use std::{cell::RefCell, rc::Rc};

use crate::{CoderunNotifiable, ItemKind, RepeatCount};
//use crate::{CalleeName, CoderunNotifiable, RepeatCount};
// TODO: Stop dependency on fcl (extract CalleeName, CoderunNotifiable to non-fcl-related file/package)
// Option: Move {CalleeName, CoderunNotifiable, RepeatCount} and CallGraph to Code[run]Commons crate
// (to be reused for (dynamic handling) code profiling, code coverage, (static handling) translation from language to language).

type Link = Rc<RefCell<CallNode>>;

struct CallNode {
    // TODO: Consider -> ItemNode (function, closure, loop)
    kind: ItemKind,
    /// Value returned by a function or `loop` (`while` and `for` loops do not return a value).
    ret_val: Option<String>,
    children: Vec<Link>,
    repeat_count: RepeatCount,
    /// Flag that tells to flush the item return (like
    /// * "} // f().",
    /// * "} // closure()."
    /// * "} // Loop body end." )
    ///
    /// upon thread context switch or std output and panic sync.
    has_ended: bool, // Function has returned, loopbody (one iteration) has ended.
}
impl CallNode {
    fn new(kind: ItemKind) -> Self {
        Self {
            kind: kind,
            ret_val: None,
            children: Vec::new(),
            repeat_count: RepeatCount::new(),
            has_ended: false,
        }
    }
    fn set_ret_val(&mut self, output: Option<String>) {
        self.ret_val = output;
    }
    fn get_ret_val(&self) -> &Option<String> {
        &self.ret_val
    }
}

// enum CacheKind {
//     Call,
//     Loopbody {
//         initial: bool,
//     }
// }
#[rustfmt::skip]
struct CachingInfo {
    /// The node to compare the `node_being_cached` with.
    /// This is None for an _initial_ loopbody in `node_being_cached`.
    model_node  : Option<Link>, 
    node_being_cached: Option<Link>,
    call_depth  : usize,
}
impl CachingInfo {
    // #[rustfmt::skip]
    fn new() -> Self {
        Self {
            model_node: None,
            node_being_cached: None,
            call_depth: 0,
        }
    }
    fn clear(&mut self) {
        self.node_being_cached = None;
        self.model_node = None;
    }
    fn is_active(&self) -> bool {
        self.node_being_cached.is_some()
    }
}

pub struct CallGraph {
    // For returning to the parent at any moment.
    // The link to a pseudo-node always exists at the bottom of the call stack.
    // The pseudo-node is not to be logged.
    call_stack: Vec<Link>,

    // Repeats the self.call_stack.last() for quick access and brevity
    // (strictly speaking is not required).
    // The node that represents the currently running function.
    // The nested calls are added as children to this node.
    current_node: Link,

    // The last node that is not being cached and is used as a model for caching the subsequent sibling(s).
    // The node referred to by caching_model_node is never removed
    // (caching_model_node is None when the node is removed upon graph clearing).
    caching_info: CachingInfo,

    // An instance (e.g. a decorator) that gets notified about changes in the call graph.
    coderun_notifiable: Rc<RefCell<dyn CoderunNotifiable>>,
}

impl CallGraph {
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

    pub fn flush(&mut self) {
        // If call caching is active:
        // * the caching model (sibling) node can have a non-zero non-flushed repeat count
        // * and the subsequent sibling (with its children) is being added to the call graph (is being cached).
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
            // Log the subtree of the (subsequent) node bing cached:
            if let Some(cached_sibling) = self.caching_info.node_being_cached.take() {
                self.flush_tree(&cached_sibling, self.caching_info.call_depth);
            }
        } else if let Some(node_being_cached) = self.caching_info.node_being_cached.clone() {
            // The initial loopbody (with optional nested initial loopbodies) is being cached.
            self.flush_tree(&node_being_cached, self.caching_info.call_depth);
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
        self.coderun_notifiable.borrow_mut().notify_flush();

        // Stop caching.
        self.caching_info.clear();
    }

    // parent { // `current`: parent (call | loopbody). call_stack: {..., parent}. `current.children`: {[..., previous_sibling]}.
    //     ...
    //     [previous_sibling() {}
    //      [// Repeats 99 time(s).]]
    //     new_sibling() {    // The call being handled.
    pub fn add_call(&mut self, call_name: &str, param_vals: Option<String>) {
        // Create the new_sibling node:
        let new_sibling = Rc::new(RefCell::new(CallNode::new(ItemKind::Call {
            name: String::from(call_name),
            param_vals: param_vals.clone(),
        })));

        // While the updates have not been done, prepeare the indo for later use.
        let siblings_call_depth = self.call_depth();
        let parent = self.current_node.clone();
        let optional_previous_sibling =
            if let Some(previous_sibling) = parent.borrow().children.last() {
                Some(previous_sibling.clone())
            } else {
                None
            };

        // Add new_sibling to the call tree by adding
        // new_sibling node to the parent's list of children:
        parent
            // self.current_node // parent
            .borrow_mut()
            .children
            .push(new_sibling.clone());

        // But not yet make the new_sibling current.

        // Fork depending on whether the caching is active.
        match self.caching_info.node_being_cached.as_ref() {
            // TODO: -> if !self.caching_is_active() {
            None => {
                // Caching is not active (ancestry has no loopbodies being cached).
                // There potentially can be a previous sibling (call or non-initial loopbody)
                // with non-zero repeat count.
                // If there is a previous sibling
                //   If a call with the same name then
                //        begin caching starting with the new sibling;
                //   otherwise (a call with different name or a loopbody)
                //        log the previous sibling's repeat count, if non-zero.
                //        Log the call.
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
                        // Log the call.
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
            }
            Some(_node_being_cached) => {
                // Caching is active.
                // If the caching has started at the enclosing loopbody
                // (with optional intermediate enclosing loopbodies in between)
                // and that loopbody is initial then flush and stop caching.
                if self.caching_info.model_node.is_none()
                {
                    self.flush(); // It also stops caching.
                }
            }
        }
        // Add new_sibling to the call stack:
        self.call_stack.push(new_sibling.clone()); // [..., parent] -> [..., parent, new_sibling]

        // Mark that the subsequent calls will be added as children to the new_sibling:
        self.current_node = new_sibling.clone();
    }

    // parent() {
    //      . . .
    //      [{ // Loop body start.   // Possible previous iterations of the current loop.
    //          . . .
    //          child() { ... } // At least one function call in the loopbody.
    //          [// child() repeats 7 time(s).]
    //      } // Loop body end.
    //      // Loop body repeats 6 time(s).]
    //      { // Loop body start.   // `current`. `call_stack`: [..., parent, loopbody]. `current.children`: loopbody's nested calls, can be empty.
    //          . . .
    //          [child() { ... }
    //          // child() repeats 3 time(s). // Not yet flushed]
    //      } // Loop body end.     // The end being handled.
    pub fn add_loopbody_end(&mut self) {
        // Logic.
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
        //              Remove this loopbody from call graph.
        //              If caching
        //                  Increment the previous loopbody repeat count.
        //                  If the current loopbody is the node being cached
        //                      Stop caching.
        //              //Return.
        //          Otherwise (differs)
        //              If caching and the current node is the one being cached
        //                  Flush and stop caching.
        //      Otherwise (no previous iteration of the current loop)
        //          Do nothing here, move on.
        //      Remove this loopbody from call stack and make parent a current node.
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
                debug_assert!(false, "Unexpected bottom of call stack");
            }
        } else {
            // Otherwise (there are nested calls) {
            // (Here, caching could only start at a parent or earlier
            // or the current node can be being cached if the node is not initial loopbody)

            let child_call_depth = self.call_depth();
            match self.call_stack.pop() {
                // Popped the ending loopbody's node from the call stack (parent or pseudo stays on top).
                None => debug_assert!(false, "Unexpected bottom of call stack"),
                Some(ending_loopbody) => {
                    debug_assert!(
                        ending_loopbody.borrow().kind.is_loopbody(),
                        "Unexpected item kind in the call stack"
                    );

                    match self.call_stack.last() {
                        None => debug_assert!(false, "Unexpected bottom of call stack"), // TODO: -> call stack bottom.
                        Some(parent_or_pseudo) => {
                            let parent_or_pseudo = parent_or_pseudo.clone();
                            // If caching is not active
                            if !self.caching_is_active() {
                                // Flush the last child's repat count, if non-zero.
                                if let Some(last_child) = self.current_node.borrow().children.last()
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
                                    .notify_loopbody_end(self.call_depth());
                            }
                            // otherwise (caching is active, starting with parent or earlier)
                            //     Do noithing here, continue caching, move on.

                            // Mark loopbody (but not the whole loop) as ended.
                            self.current_node.borrow_mut().has_ended = true;

                            // If there is a previous loopbody (previous iteration of the current loop
                            // or the last iteration of the previous loop ;-)
                            let siblings_count = parent_or_pseudo.borrow().children.len();
                            if siblings_count > 1 {
                                let previous_node =
                                    parent_or_pseudo.borrow().children[siblings_count - 2].clone();
                                if previous_node.borrow().kind.is_loopbody()
                                // let previous_node_kind = previous_node.borrow().kind.clone();
                                // if let ItemKind::Loopbody { ended_the_loop } =
                                //     previous_node_kind
                                //     && !ended_the_loop
                                {
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
                                        if self.caching_is_active() {
                                            // Increment the previous loopbody repeat count.
                                            previous_node.borrow_mut().repeat_count.inc();
                                            // If the current loopbody is the node being cached,
                                            if let Some(node_being_cached) = &self.caching_info.node_being_cached 
                                                && node_being_cached.as_ptr() == ending_loopbody.as_ptr() 
                                            {
                                                // Stop caching.
                                                self.caching_info.clear();
                                            }
                                        }
                                        // // Return.
                                        // return;
                                    }
                                    // Otherwise (differs)
                                    // If caching and the current node is the one being cached
                                    if let Some(node_being_cahed) = &self.caching_info.node_being_cached
                                        && ending_loopbody.as_ptr() == node_being_cahed.as_ptr()
                                    {
                                        // Flush and stop caching.
                                        self.flush(); // It also stops caching.
                                    }
                                }
                                // Otherwise (no previous iteration of the current loop, previous node is a call or a different loop)
                                //     Do nothing here, move on.
                            }
                            // else (no previous node)
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
    // // parent() { // current: parent (call or loopbody). `call_stack`: [..., parent]. `current.children`: siblings.
    // //      . . .
    // //      [{ // Loop body start.   // Previous iterations of the current loop.
    // //          . . .
    // //          child() { ... } // At least one function call in the loopbody.
    // //          [// child() repeats 7 time(s).]
    // //      } // Loop body end.
    // //      // Loop body repeats 6 time(s).]
    // pub fn add_loop_end(&mut self) {
    //     if let Some(last_sibling) = self.current_node.borrow_mut().children.last() {
    //         match &mut last_sibling.borrow_mut().kind {
    //             ItemKind::Loopbody { ended_the_loop } => *ended_the_loop = true,
    //             ItemKind::Call { .. } => debug_assert!(false, "Unexpected state"),
    //         }
    //     }
    // }

    // < `parent() {` | `{ // Loop body start` > // `current`. `call_stack`: [..., parent]. `current.children`: [..., {previous_sibling | loopbody}].
    //     [...]
    //     [{ // Loop body start. // The body of the previous loop.
    //        . . .
    //        loop_nested_sibling() { .. }  // At least one mandatory function call (otherwise the loop would not be logged).
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
        //  * either no sibling-level node (just parent (who can also be a loopbody) or pseudo)
        //  * or a sibling-level node (with potentially non-zero repeat count) of
        //      * either previous loop's last logged loopbody (with mandatory nested calls)
        //        (node.kind.ended_the_loop: true)
        //      * or previous_sibling() call (function or closure)
        //      * or previous iteration (with mandatory nested calls) of the current loop
        //        ( node.kind.ended_the_loop: false ).
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
        //              * gets NO model_node (new loopbody node marked as initial),
        //              * gets the new loopbody node.
        //
        //          (For an initial loopbody the caching will continue until either the first-most nested call
        //          (directly in the loopbody or indirectly in the nested loopbodies) or until loopbody end.
        //          Upon the first-most nested function call the cache will be flushed (starting from the current loopbody,
        //          through the intermediate nested loopbodies, and ending after the first-most nested function call),
        //          caching will end, and execution will continue.
        //          Upon initial loopbody end,
        //          if the loopbody will have no children, then
        //              the loopbody will get removed and the subsequent loopbody, if any, of the current loop
        //              will be marked later as initial.
        //          Otherwise (the inital loopbody will have children) the last child's repeat count
        //          (which will be the only non-flushed thing) will get flushed)
        //      } otherwise (it is non-initial loopbody) {
        //          caching info
        //            * gets the model node pointing to the previous loopbody/iteration of the
        //              current loop (new loopbody node marked as non-initial)
        //            * gets new loopbody node.
        //
        //          (For a non-initial loopbody the caching will continue until loopbody end, where
        //          the loopbody, if will not have nested calls, will be removed, otherwise will be analized
        //          in a similar way as repeted function call,
        //          i.e. will be compared to the previous loopbody, and,
        //          if equal, will get removed incrementing the repeat count for the previous loopbody,
        //          otherwise (will differ) will cause previous loopbody's repeat count flush and one's own flush)
        //      }
        // } Otherwise (caching is active) {
        //      (Caching has started at the parent or earlier.
        //      (If the previous node is a loopbody and cahing started at it, then caching ended upon loopbody end,
        //      and the execution wouldn't be here))
        //
        //      Do nothing (after creating and adding the new loopbody to the graph, continue caching).
        // }

        // Implementation.
        // If there's a sibling-level node, then memorize the info for flushing its repeat count
        // ([name,] repeat count, call depth, etc.).
        let call_depth = self.call_depth();
        let mut previous_sibling_node_info = None;
        if let Some(previous_sibling) = self.current_node.borrow().children.last().clone()
        {
            previous_sibling_node_info = Some(previous_sibling.clone());
        }

        // Create the loopbody node,
        let new_loopbody_node = Rc::new(RefCell::new(CallNode::new(ItemKind::Loopbody 
            // {
            //     ended_the_loop: false,
            // }
        )));

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
            // and NOT the last iteration of the previous loop ;-) then {
            if let Some(rc_previous_sibling) = previous_sibling_node_info.as_ref()
                && !rc_previous_sibling.borrow().kind.is_loopbody()
                // && match rc_previous_sibling.borrow().kind {
                //     ItemKind::Call { .. } => true, // Function or closure.
                //     ItemKind::Loopbody { ended_the_loop } => ended_the_loop, // Previous (different) loop.
                // }
            {
                // Log the repeat count, if non-zero, of the previous sibling-level node.
                let mut previous_sibling = rc_previous_sibling.borrow_mut();
                // TODO: `if` below -> separate call.
                if !previous_sibling.repeat_count.non_flushed_is_empty() {
                    self.coderun_notifiable.borrow_mut().notify_repeat_count(
                        call_depth,
                        &previous_sibling.kind, //&previous_sibling_name,
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
                    // let kind = previous_sibling.borrow().kind.clone();
                    // match kind {
                    //     ItemKind::Call { .. } => None, // Function or closure.
                    //     ItemKind::Loopbody { ended_the_loop } if ended_the_loop => None, // Previous loop.
                    //     ItemKind::Loopbody { .. } => Some(previous_sibling), // Previous iteration of the current loop.
                    // }
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

    // parent { // call | loopbody
    //     [...]
    //     [previous_sibling() {...}
    //      [// previous_sibling() repeats 99 time(s). // Not yet flushed.]]
    //     || (or)
    //     [{ // Loop body start.
    //          child() {...}
    //          [// child() repeats 10 time(s).]
    //      } // Loop body end.
    //      [// Loop body repeats 6 time(s). // Flushed.]]
    //     returning_sibling() {        // current. call_stack: [..., parent, returning_sibling].
    //        [... // Nested calls (children).
    //         [// last_child() repeats 9 time(s). // (Not yet logged) ]]
    //     } // The return being handled.
    pub fn add_ret(&mut self, ret_val: Option<String>) {
        // If caching is not active {
        //     Log the repeat count, if non-zero, of the last_child, if present.
        //     Log the return of the returning_sibling.
        // }
        if !self.caching_is_active() {
            let call_depth = self.call_depth();
            // Log the repeat count, if non-zero, of the last_child, if present:
            let returning_sibling = self.call_stack.last().unwrap();
            returning_sibling.borrow_mut().set_ret_val(ret_val);
            if let Some(last_child) = returning_sibling.borrow().children.last()
                && !last_child.borrow().repeat_count.non_flushed_is_empty()
            {
                self.coderun_notifiable.borrow_mut().notify_repeat_count(
                    call_depth, // While the returning_sibling is still on the call stack, the call depth reflects the last_child's call_depth.
                    &last_child.borrow().kind, //.name,
                    last_child.borrow().repeat_count.non_flushed(),
                );
                last_child.borrow_mut().repeat_count.mark_flushed();
            }
            // Log the return of the returning_sibling:
            match &self.current_node.borrow().kind {
                ItemKind::Loopbody { .. } => {
                    debug_assert!(false, "Unexpected node in the call tree")
                }
                ItemKind::Call { name, .. } => {
                    let has_nested_calls = !self.current_node.borrow().children.is_empty();
                    self.coderun_notifiable.borrow_mut().notify_return(
                        call_depth - 1, // `- 1`: // The returning_sibling is still on the call stack. The call_depth reflects the children's indent.
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
                if Self::trees_are_equal(&previous_sibling, &returning_sibling, false)
                // Do not compare the repeat count for the previous_sibling and returning_sibling
                // (because the returning_sibling's repeat count is always 0 at this stage,
                // but the previous_sibling's repeat count can be >0),
                // but compare for the nested calls.
                {
                    // the previous sibling's repeat count is incremented,
                    previous_sibling.borrow_mut().repeat_count.inc();
                    // and the currently returning sibling's call subtree is removed from the call graph.
                    parent_or_pseudo.borrow_mut().children.pop();
                    // If the previous sibling is the caching model node then caching is over,
                    // i.e. the caching model becomes `None`.
                    if let Some(model_node) = self.caching_info.model_node.as_ref()
                        && model_node.as_ptr() == previous_sibling.as_ptr()
                    {
                        self.caching_info.clear();
                    } // else (caching started at a parent level or above) do nothing.
                } else {
                    // The returning_sibling's and previous_sibling's subtrees differ.
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
                                &previous_sibling.borrow().kind, //.name,
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

    /// Returns the call depth, starting with 0 when pseudo-node is in,
    /// in the call graph for the currently running function's children.
    /// When a node is added to the call stack, the call depth starts reflecting the indent of its children.
    /// E.g. before adding main() the call depth is 0 (indent for printing main() who is a child of pseudonode).
    /// After adding main() the call depth is 1 (indent for printing main()'s children).
    pub fn call_depth(&self) -> usize {
        let call_depth = self.call_stack.len();
        debug_assert!(call_depth >= 1); // Pseudo-node.
        call_depth - 1
    }

    pub fn caching_is_active(&self) -> bool {
        self.caching_info.is_active()
    }

    fn trees_are_equal(a: &Link, b: &Link, compare_root_repeat_count: bool) -> bool {
        let a = a.borrow();
        let b = b.borrow();
        match &a.kind {
            ItemKind::Call { name, .. } => {
                let a_name = name;
                match &b.kind {
                    ItemKind::Call { name, .. } => {
                        if a_name != name {
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

    /// Traverses the call subtree recursively and calls the notification callbacks,
    /// thus notifying that the subtree has stopped being cached.
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
                ItemKind::Loopbody { .. } => {
                    self.coderun_notifiable
                        .borrow_mut()
                        .notify_loopbody_end(call_depth) // TODO: call_depth
                }
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
