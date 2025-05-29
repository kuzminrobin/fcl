use std::{cell::RefCell, rc::Rc};

use fcl_traits::{CalleeName, CoderunNotifiable, RepeatCount};    
// TODO: Stop dependency on fcl (extract CalleeName, CoderunNotifiable to non-fcl-related file/package)

type Link = Rc<RefCell<CallNode>>;

struct CallNode {
    name: CalleeName,
    children: Vec<Link>,
    repeat_count: RepeatCount,
    has_returned: bool
}
impl CallNode {
    pub fn new(name: &CalleeName) -> Self {
        Self {
            name: name.clone(),
            children: Vec::new(),
            repeat_count: RepeatCount::new(),
            has_returned: false
        }
    }
}

#[rustfmt::skip]
struct CachingInfo {
    node        : Option<Link>, // TODO: Consider model_node.
    call_depth  : usize,
    next_sibling: Option<Link>,
}
impl CachingInfo {
    #[rustfmt::skip]
    fn new() -> Self {
        Self {
            node        : None, 
            call_depth  : 0, 
            next_sibling: None
        }
    }
    fn clear(&mut self)  {
        self.node = None
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
    current: Link,  // TODO: -> current_node

    // The last node that is not being cached and is used as a model for caching the subsequent sibling(s).
    // The node referred to by caching_model_node is never removed
    // (caching_model_node is None when the node is removed upon graph clearing).
    caching_info: CachingInfo,

    // An instance (e.g. a decorator) that gets notified about changes in the call graph.
    coderun_notifiable: Rc<RefCell<dyn CoderunNotifiable>>,
}

impl CallGraph {
    
    pub fn new(coderun_notifiable: Rc<RefCell<dyn CoderunNotifiable>>) -> Self {
        let pseudo_node = 
            Rc::new(RefCell::new(CallNode::new(&CalleeName::Function(String::new()))));
            // Rc::new(RefCell::new(CallNode::new(&CalleeName::Function(""))));
        Self {
            current: pseudo_node.clone(),
            call_stack: vec![pseudo_node],
            caching_info: CachingInfo::new(),
            coderun_notifiable,
        }
    }

    pub fn flush(&mut self) {
        // If caching is active:
        // * the caching model node can have a non-zero non-flushed repeat count
        // * and the subsequent sibling (with its children) is being added to the call graph (is cached).
        if let Some(caching_model_node) = self.caching_info.node.as_ref() {
            // TODO: Extract the code below into `flush_cache()` and call from 2 places.
            // Log the caching_model_node's repeat count, if non-zero,
            // Log the subtree of the caching_model_node's next sibling.
            // Stop caching (`caching_model_node = None`).

            // If the caching model node has a non-flushed repeat count
            if !caching_model_node.borrow().repeat_count.non_flushed_is_empty() {
                self.coderun_notifiable.borrow_mut().notify_repeat_count(
                    self.caching_info.call_depth,
                    &caching_model_node.borrow().name,
                    caching_model_node.borrow().repeat_count.non_flushed()
                );
                caching_model_node.borrow_mut().repeat_count.mark_flushed();
            }
            // Log the subtree of the (subsequent) cached sibling:
            if let Some(cached_sibling) = self.caching_info.next_sibling.take() {
                self.traverse_tree(&cached_sibling, self.caching_info.call_depth); 

            }

            // Stop caching.
            self.caching_info.clear();
        } else {
            // The latest sibling can have a non-zero non-flushed repeat count.
            // The `self.current` points to the parent or pseudo.
            if let Some(latest_sibling) = self.current.borrow().children.last()
                && !latest_sibling.borrow().repeat_count.non_flushed_is_empty()
            {
                self.coderun_notifiable.borrow_mut().notify_repeat_count(
                    self.caching_info.call_depth,
                    &latest_sibling.borrow().name,
                    latest_sibling.borrow().repeat_count.non_flushed()
                );
                latest_sibling.borrow_mut().repeat_count.mark_flushed();
            }
        }
        self.coderun_notifiable.borrow_mut().notify_flush();
    }

    // TODO: child -> sibling
    // parent() { // current. call_stack: [..., parent]. children: [..., previous_child].
    //     ...
    //     [previous_child() {}
    //      [// Repeats 99 time(s).]]
    //     current_child() {    // The call being handled.
    pub fn add_call(&mut self, name: &CalleeName) {
        // Create the current_child node:
        let rc_current_child = Rc::new(RefCell::new(CallNode::new(&name)));

        // Try to detect the caching start:
        if !self.caching_is_active() {
            // Check if the current_child name repeats the previous_child name:
            let children = &self.current.borrow().children; // parent.children
            // If there is a previous_child
            if let Some(previous_child) = children.last() {
                // If current_child.name == previous_child.name:
                let previous_child_name = previous_child.borrow().name.clone();
                if previous_child_name == *name {
                    // Mark that the current_child (including its children) starts being cached,
                    // and previous_child becomes the caching_model (for comapring (upon return) current_child with
                    // and detecting the caching end).
                    self.caching_info = CachingInfo {
                        node: Some(previous_child.clone()),
                        call_depth: self.call_depth(), // The node is not yet on the call stack.
                        next_sibling: Some(rc_current_child.clone()),
                    }
                } else {
                    // Previous child has different name. Its repeat_count stops being cached.
                    // Log the previous_child.repeat_count, if non-zero.
                    if !previous_child.borrow().repeat_count.non_flushed_is_empty() {
                        self.coderun_notifiable.borrow_mut().notify_repeat_count(
                            self.call_depth(),  // The node is not yet on the call stack.
                            &previous_child_name,
                            previous_child.borrow().repeat_count.non_flushed()
                        );
                        previous_child.borrow_mut().repeat_count.mark_flushed();
                    } // else nothing.
                }
            } // else nothing.
        } // else nothing.

        // Add current_child to the call tree.
        // Add current_child node to the parent's list of children:
        self.current // parent
            .borrow_mut()
            .children
            .push(rc_current_child.clone());

        // Add current_child to the call stack:
        self.call_stack.push(rc_current_child.clone()); // [..., parent] -> [..., parent, current_child]

        // Mark that the subsequent calls will be added as children to the current_child:
        self.current = rc_current_child;

        // If not caching, log the call:
        if !self.caching_is_active() {
            self.coderun_notifiable.borrow_mut()
                .notify_call(
                    self.call_depth() - 1, // `- 1`: // The node is already on the call stack.
                    &name);
        }
    }

    // parent() {
    //     ...
    //     [previous_sibling() {}
    //      [// previous_sibling() repeats 99 time(s).]]
    //     returning_sibling() {        // current. call_stack: [..., parent, returning_sibling].
    //        [... // Nested calls (children).
    //         [// last_child() repeats 9 time(s). // (Not yet logged) ]]
    //     } // The return being handled.
    pub fn add_ret(&mut self) {
        // If caching is not active {
        //     Log the repeat count, if non-zero, of the last_child, if present.
        //     Log the return of the returning_sibling.
        // }
        if !self.caching_is_active() {
            let call_depth = self.call_depth();
            // Log the repeat count, if non-zero, of the last_child, if present:
            let returning_sibling = self.call_stack.last().unwrap();
            if let Some(last_child) = returning_sibling.borrow().children.last()
                && !last_child.borrow().repeat_count.non_flushed_is_empty()
            {
                self.coderun_notifiable.borrow_mut().notify_repeat_count(
                    call_depth, // While the returning node is still on the call stack, the call depth reflects the last_child's call_depth.
                    &last_child.borrow().name,
                    last_child.borrow().repeat_count.non_flushed(),
                );
                last_child.borrow_mut().repeat_count.mark_flushed();
            }
            // Log the return of the returning_sibling:
            let name = self.current.borrow().name.clone();
            let has_nested_calls = !self.current.borrow().children.is_empty();
            self.coderun_notifiable.borrow_mut()
                .notify_return(
                    call_depth - 1, // `- 1`: // The node is still on the call stack.
                    &name, has_nested_calls);

            self.current.borrow_mut().has_returned = true;

            // Handle the return in the call graph:
            self.call_stack.pop();
            self.current = self.call_stack.last().unwrap().clone();
        } else {
            // Otherwise (caching is active) {
            let returning_func = self.call_stack.pop().unwrap();
            returning_func.borrow_mut().has_returned = true;
            let parent_or_pseudo = self.call_stack.last().unwrap();
            let returning_func_call_depth = self.call_depth();  // The returning func is not on the call stack.
            self.current = parent_or_pseudo.clone();
            //     If there exists a previous sibling of the returning function, then {
            if parent_or_pseudo.borrow().children.len() > 1 {
                //   The call subtree of the returning function is compared recursively
                //   to the previous sibling call subtree.
                //   If equal {
                let previous_sibling_index = parent_or_pseudo.borrow().children.len() - 2;
                let previous_sibling =
                    parent_or_pseudo.borrow().children[previous_sibling_index].clone();
                if Self::trees_are_equal(
                    &previous_sibling, 
                    &returning_func, 
                    false) // Do not compare repeat count for previous_sibling and returning_func, but compare for the nested calls.
                {
                    //     the previous sibling's repeat count is incremented
                    previous_sibling.borrow_mut().repeat_count.inc();

                    //     and the currently returning function's call subtree is removed from the call graph.
                    parent_or_pseudo.borrow_mut().children.pop();
                    //     If the previous sibling is the caching_model then caching is over,
                    //     i.e. the `caching_model` becomes `None`.
                    // }
                    if self.caching_info.node.as_ref().unwrap().as_ptr()
                        == previous_sibling.as_ptr()
                    {
                        self.caching_info.clear();
                    } // else (caching started at a parent level or above) do nothing.
                } else {
                    // Otherwise (returning_sibling and previous_sibling differ) {
                    //     If the previous_sibling is the cahing_model then {
                    //         Log the previous_sibling's repeat count, if non-zero,
                    //         Log the subtree of the returning_sibling.
                    //         Stop caching. `caching_model = None`.
                    //     }
                    // If the previous_sibling is the cahing_model then
                    if self.caching_info.node.as_ref().unwrap().as_ptr()
                        == previous_sibling.as_ptr() {
                        //         Log the previous_sibling's repeat count, if non-zero,
                        if !previous_sibling.borrow().repeat_count.non_flushed_is_empty() {
                            self.coderun_notifiable.borrow_mut().notify_repeat_count(
                                returning_func_call_depth, // Same call depth for the returning and previous siblings.
                                &previous_sibling.borrow().name,
                                previous_sibling.borrow().repeat_count.non_flushed()
                            );
                            previous_sibling.borrow_mut().repeat_count.mark_flushed();
                        }
                        //         Log the subtree of the returning_sibling.
                        self.traverse_tree(&returning_func, returning_func_call_depth);
                        //         Stop caching. `caching_model = None`.
                        self.caching_info.clear();
                    }
                    //     otherwise (caching has been detected at a parent level or above) {
                    //         // Do nothing, continue caching.
                    //     }
                    // }
                }
            }
            // Otherwise (the returning_sibling is the only child) {
            //     // Continue caching, do nothing. The caching end cannot be detected upon return from the only child.
            // }
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
        self.caching_info.node.is_some()
    }

    fn trees_are_equal(a: &Link, b: &Link, compare_root_repeat_count: bool) -> bool {
        let a = a.borrow();
        let b = b.borrow();
        if a.name != b.name {
            return false;
        }

        if a.children.len() != b.children.len() {
            return false;
        }
        for index in 0..a.children.len() {
            if !Self::trees_are_equal(&a.children[index], &b.children[index], true) {
                return false;
            }
        }
        if compare_root_repeat_count && a.repeat_count != b.repeat_count {
            return false
        }

        true
    }

    /// Traverses the call subtree recursively and calls the notification callbacks,
    /// thus notifying that the subtree has stopped being cached.
    fn traverse_tree(&mut self, current_node: &Link, call_depth: usize) {
        let mut current_node = current_node.borrow_mut();
        let name = &current_node.name;
        let func_children = &current_node.children;

        // The call:
        self.coderun_notifiable.borrow_mut().notify_call(call_depth, name);

        // Traverse children recursively:
        for child in func_children {
            self.traverse_tree(child, call_depth + 1);
        }

        // If the call has returned:
        if current_node.has_returned {
            // The return:
            let has_nested_calls = !func_children.is_empty();
            self.coderun_notifiable.borrow_mut()
                .notify_return(call_depth, name, has_nested_calls);

            // The repeat count:
            if !current_node.repeat_count.non_flushed_is_empty() {
                self.coderun_notifiable.borrow_mut()
                    .notify_repeat_count(call_depth, name, current_node.repeat_count.non_flushed());
                current_node.repeat_count.mark_flushed();
            } // else (no non-flushed repeat count) do nothing.
        } // else (hasn't returned) do nothing.
    }
}
