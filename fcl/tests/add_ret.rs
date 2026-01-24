use std::cell::RefCell;
use std::rc::Rc;

use fcl::call_log_infra::instances::THREAD_DECORATOR;
// use fcl::call_log_infra::instances::{THREAD_DECORATOR, THREAD_LOGGER};
use fcl_proc_macros::loggable;

// By the moment of `code_commons::call_graph::CallGraph::add_ret()` (being tested in this file)
// the call graph state (and the terminology) is:
// parent { // The call or the loop body.
//     [...] // Brackets (`[]`) by default mean "optional".
//
//     [previous_sibling() {...} // If caching is active then this is the "caching model node".
//      [// previous_sibling() repeats 99 time(s). // Not yet flushed (if caching is active).]] // NOTE: The returning function (below)
//                                                  // can get removed and increment this repreat count (if caching is active).
//     || (or)
//     [{ // Loop body start. Caching is not active (since the function call after a loop body cannot trigger caching).
//          child() {...}
//          [// child() repeats 10 time(s).]
//      } // Loop body end.
//      [// Loop body repeats 6 time(s). // Flushed (since caching is not active).]]
//
//     returning_sibling() { // Current node. call_stack: [..., parent (if it's a call), returning_sibling]. // Brackets (`[]`) in this line mean {array|vector|etc.}
//        [... // Nested calls (children).
//         [// last_child() repeats 9 time(s). // Not yet flushed. ]]
//     } // The `return` of interest, that's being handled in `add_ret()` under test.

// High-level logic to test (at the moment of the return of interest):
// ---------------
// A: If caching is not active {
// A:     Log the repeat count, if non-zero, of the last_child, if present.
// A:     Log the return of the returning_sibling.
// A: } else { // (caching is active)
//      If there exists a previous_sibling, then {
//          The call subtree of the returning_sibling is compared recursively
//          to the previous_sibling's call subtree.
//          If the call subtrees are equal {
//              the previous sibling's repeat count is incremented,
//              and the returning_sibling's call subtree is removed from the call graph.
//              If the previous sibling is the caching model node then
//                  caching is over, i.e. the caching model becomes `None`.
//              else (caching started at a parent level or above)
//                  do nothing.
//          } else { // The call subtrees are different.
//              (Caching is active, there is the previous_sibling)
//              The returning_sibling's and previous_sibling's subtrees differ
//              (either by name, if caching started at parent or earlier,
//              or by children, if the previous_sibling is the cahing model node).
//              If the previous_sibling is the cahing model node then {
//                  Log the previous_sibling's repeat count, if non-zero,
//                  Log the subtree of the returning_sibling,
//                  Stop caching.
//              }
//          }
// B:      } // else (no previous_sibling, the returning_sibling is the only child of parent) {
// B:          continue caching (do nothing). The caching end cannot be detected upon return from the only child.
// B:      }
// B: }

// Test cases:

// A: `no_caching_child_repeats`:
// [previous_sibling() {}] // Doesn't exist or has different name, i.e. caching is not active.
// returning_func() {
//     [child() {}   // The optional child
//      [// Repeats]]   // and optional non-zero repeat count.
// }
// assert_eq!(): The log above.
#[test]
fn no_caching_child_repeats() {
    #[loggable]
    fn child() {}
    #[loggable]
    fn returning_func(child_call_count: usize) {
        match child_call_count {
            0 => {}       // No child call.
            1 => child(), // One child call (no repeat count).
            _ => {
                // Child has a repeat count.
                child();
                child();
                child(); // child() repeats 2 time(s).
            }
        }
    }

    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    // Generate the log and check it:

    // No previous sibling.
    returning_func(0); // No child.
    unsafe {
        assert_eq!(
            std::str::from_utf8_unchecked(&*log.borrow()),
            concat!("returning_func(child_call_count: 0) {}\n",)
        )
    };
    // Clear the log for more checks.
    log.borrow_mut().clear();

    // No previous sibling.
    returning_func(1); // One child call.
    #[rustfmt::skip]
    unsafe {
        assert_eq!(
            std::str::from_utf8_unchecked(&*log.borrow()),
            concat!(
                "returning_func(child_call_count: 1) {\n",
                "  child() {}\n", // One child call, no repeat count.
                "} // returning_func().\n"
            )
        )
    };
    log.borrow_mut().clear();

    // No previous sibling.
    returning_func(3); // More than one child call. The repeat count to be logged.
    #[rustfmt::skip]
    unsafe {
        assert_eq!(
            std::str::from_utf8_unchecked(&*log.borrow()),
            concat!(
                "returning_func(child_call_count: 3) {\n",
                "  child() {}\n", 
                "  // child() repeats 2 time(s).\n",
                "} // returning_func().\n"
            )
        )
    };
    log.borrow_mut().clear();

    #[loggable]
    fn previous_sibling() {}

    previous_sibling(); // Previous sibling with a different name. No caching.
    returning_func(3); // More than one child call. The repeat count to be logged.
    #[rustfmt::skip]
    unsafe {
        assert_eq!(
            std::str::from_utf8_unchecked(&*log.borrow()),
            concat!(
                "previous_sibling() {}\n",
                "returning_func(child_call_count: 3) {\n",
                "  child() {}\n", 
                "  // child() repeats 2 time(s).\n",
                "} // returning_func().\n"
            )
        )
    };
}

// B: `caching_continues_after_the_only_sibling`
// repeating_parent() { /* No children */ } // The original call.
// assert_eq!(): The log above.
// repeating_parent() {      // The repeated call (call with the same name). Triggers caching.
//     assert_eq!(): No second repeating_parent() in the log. Caching is active.
//     returning_sibling() {} // The `return` of interest.
//     assert_eq!(): No second repeating_parent() (and returning_sibling()) in the log. Caching continues.
// } // Caching stops since the same-name calls differ with internals. The second repeating_parent() gets flushed.
// assert_eq!(): The whole log above.
#[test]
fn caching_continues_after_the_only_sibling() {

    // The functions that will generate the call log.
    #[loggable]
    fn returning_sibling() {}
    #[loggable]
    fn repeating_parent(calls_child: bool, log: Rc<RefCell<Vec<u8>>>) {
        if calls_child {
            // assert_eq!(): No second repeating_parent() in the log. Caching is active.
            unsafe {
                let call_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                assert_eq!(
                    call_log,
                    concat!("repeating_parent(calls_child: false, log: RefCell { value: [] }) {}\n")
                )
            };

            returning_sibling();
            // assert_eq!(): No second repeating_parent() (and returning_sibling()) in the log. Caching continues.
            unsafe {
                let call_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                assert_eq!(
                    call_log,
                    concat!("repeating_parent(calls_child: false, log: RefCell { value: [] }) {}\n")
                )
            };
        }
    }

    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    // Generate the log and check it:

    repeating_parent(false, log.clone()); // The original call.
    // assert_eq!(): The log above.
    unsafe {
        let call_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
        assert_eq!(
            call_log,
            concat!("repeating_parent(calls_child: false, log: RefCell { value: [] }) {}\n")
        )
    };

    repeating_parent(true, log.clone()); // The repeated call (call with the same name). Triggers caching.
    // assert_eq!(): The whole log above.
    #[rustfmt::skip]
    unsafe {
        let call_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
        assert_eq!(
            call_log,
            concat!(
                "repeating_parent(calls_child: false, log: RefCell { value: [] }) {}\n",
                "repeating_parent(calls_child: true, log: RefCell { value: [",
                        "114, 101, 112, 101, 97, 116, 105, 110, 103, 95, 112, 97, 114, 101, 110, 116, ",
                        "40, 99, 97, 108, 108, 115, 95, 99, 104, 105, 108, 100, 58, 32, 102, 97, 108, ",
                        "115, 101, 44, 32, 108, 111, 103, 58, 32, 82, 101, 102, 67, 101, 108, 108, 32, ",
                        "123, 32, 118, 97, 108, 117, 101, 58, 32, 91, 93, 32, 125, 41, 32, 123, 125, 10] }) {\n",
                "  returning_sibling() {}\n", // Upon this return the caching continues. 
                "} // repeating_parent().\n", // (Not the subject of this test) Upon this return the second repeating_parent() gets flushed. 
            )
        )
    };
}

