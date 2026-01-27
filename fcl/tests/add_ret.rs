use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::loggable;
#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::{THREAD_DECORATOR, THREAD_LOGGER};

// By the moment of `code_commons::call_graph::CallGraph::add_ret()` (being tested in this file)
// the call graph state (and the terminology) is:
// parent { // The call or the loop body that encloses the call of interest.
//     [...] // Brackets (`[]`) by default mean "optional".
//
//     [previous_sibling() {...} // If caching has started upon the call of interest (returning_sibling()) below then this is the "caching model node".
//      [// previous_sibling() repeats 99 time(s).  // Not yet flushed if caching is active.]] // NOTE: The returning function 
//                                                  // (returning_sibling() below)
//                                                  // can get removed and increment this repreat count, if caching is active.
//     || (or)
//     [{ // Loop body start. Caching is either not active (since the function call after a loop body cannot trigger caching) 
//                                                  // or is active and has started at the parent or earlier level.
//          child() {...}
//          [// child() repeats 10 time(s).]
//      } // Loop body end.
//      [// Loop body repeats 6 time(s). // Flushed (if caching is not active).]]
//
//     // The function of interest:
//     returning_sibling() { // Current node. call_stack: [..., parent (if it's a call), returning_sibling]. // Brackets (`[]`) in this line mean {array|vector|etc.}
//        [... // Nested calls (children).
//         [// last_child() repeats 9 time(s). // Not yet flushed. ]]
//     } // The `return` of interest, that's being handled in `add_ret()` under test.

// High-level logic to test (at the moment of the `return` of interest):
// ---------------
// A: If caching is not active {
// A:     Log the repeat count, if non-zero, of the last_child(), if present.
// A:     Log the return of the returning_sibling().
// A: } else { // (caching is active)
// D:     If there exists a previous_sibling(), then {
// D:         The call subtree of the returning_sibling is compared recursively
// D:         to the previous_sibling's call subtree.
// D:         If the call subtrees are equal {
// D:             the previous sibling's repeat count is incremented,
// D:             and the returning_sibling's call subtree is removed from the call graph.
// D:             If the previous sibling is the caching model node then
// D:                 caching is over, i.e. the caching model becomes `None`.
// C:             else (caching started at a parent level or above)
// C:                 do nothing.
// D:         } else { // The call subtrees are different.
// D:             (Caching is active, there is the previous_sibling)
// D:             The returning_sibling's and previous_sibling's subtrees differ
// [TODO:]        (either by name, if caching started at parent or earlier,
// D:             or by children, if the previous_sibling is the cahing model node).
// D:             If the previous_sibling is the cahing model node then {
// D:                 Log the previous_sibling's repeat count, if non-zero,
// D:                 Log the subtree of the returning_sibling,
// D:                 Stop caching.
// D:             }
// D:         }
// B:      } // else (no previous_sibling, the returning_sibling is the only child of parent) {
// B:          continue caching (do nothing). The caching end cannot be detected upon return from the only child.
// B:      }
// B: }

// Test cases:

#[test]
fn ret_from_cached_func() {
    // The instrumented functions that will generate the call log:
    #[loggable]
    fn child() {}
    #[loggable]
    fn sibling(third_call: bool) {
        child();
        child(); // Generates the repeat count in the call log.
        if third_call { // Generates the difference between the calls to `sibling()`.
            child();
        }
    }
    // TODO: Suppress the `log` param logging when the suppression is implemented.
    #[loggable]
    fn parent(third_child_call: bool, /*parent_number: usize,*/ log: Rc<RefCell<Vec<u8>>>) {
        sibling(false); // Original.

        sibling(false); // Repeats.
        // Assert: The log above, except the sibling's repeat count, is in the call log:
        #[rustfmt::skip]
        unsafe {
            let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
            assert_eq!(
                log_contents,
                if third_child_call {
                    concat!(
                        "parent(third_child_call: true, log: RefCell { value: [] }) {\n", // Original.
                        "  sibling(third_call: false) {\n",  // Original.
                        "    child() {}\n",
                        "    // child() repeats 1 time(s).\n",
                        "  } // sibling().\n",
                        // No repeat count.
                    ) 
                } else {
                    concat!(
                        "parent(third_child_call: false, log: RefCell { value: [] }) {\n", // Original.
                        "  sibling(third_call: false) {\n",  // Original.
                        "    child() {}\n",
                        "    // child() repeats 1 time(s).\n",
                        "  } // sibling().\n",
                        // No repeat count.
                    ) 
                }
            )
        };
        
        sibling(third_child_call); // Of interest. The return under test.
        if ! third_child_call { // All siblings are equal.
            // Assert: (If siblings above are equal then) no latest sibling and no previous sibling's repeat count in the call log:
            #[rustfmt::skip]
            unsafe {
                let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                assert_eq!(
                    log_contents,
                    concat!(
                        "parent(third_child_call: false, log: RefCell { value: [] }) {\n", // Original.
                        "  sibling(third_call: false) {\n",  // Original.
                        "    child() {}\n",
                        "    // child() repeats 1 time(s).\n",
                        "  } // sibling().\n",
                        // No repeat count.
                    ) 
                )
            };
        } else { // The latest sibling is different.
            // Assert: (the siblings above differ) the whole log above.
            #[rustfmt::skip]
            unsafe {
                let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                assert_eq!(
                    log_contents,
                    concat!(
                        "parent(third_child_call: true, log: RefCell { value: [] }) {\n",     // Original.
                        "  sibling(third_call: false) {\n",  // Original.
                        "    child() {}\n",
                        "    // child() repeats 1 time(s).\n",
                        "  } // sibling().\n",
                        "  // sibling() repeats 1 time(s).\n",                          // Repeats.
                        "  sibling(third_call: true) {\n",                              // Of interest. The return under test.
                        "    child() {}\n",
                        "    // child() repeats 2 time(s).\n", // The difference from the previous siblings.
                        "  } // sibling().\n",
                    ) 
                )
            };
        }
    }

    // Mock log writer creation and substitution of the default one:
    let log: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    // The call log generation and some of the checks:

    // All siblings are equal:
    parent(false, log.clone()); 
    // Assert: the whole log above:
    #[rustfmt::skip]
    unsafe {
        let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
        assert_eq!(
            log_contents,
            concat!(
                "parent(third_child_call: false, log: RefCell { value: [] }) {\n",
                "  sibling(third_call: false) {\n",  // Original.
                "    child() {}\n",
                "    // child() repeats 1 time(s).\n",
                "  } // sibling().\n",
                "  // sibling() repeats 2 time(s).\n", // All siblings are equal.
                "} // parent().\n",
            ) 
        )
    };

    // Prepare the log for one more test, for that:
    // Prevent the subsequent `parent()` caching by calling an unrelated function:
    #[loggable]
    fn dummy() {}
    dummy();
    // Clear the log:
    log.borrow_mut().clear();

    // The sibling of interest is different:
    parent(true, log.clone()); 
    // Assert: the whole log above:
    #[rustfmt::skip]
    unsafe {
        let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
        assert_eq!(
            log_contents,
            concat!(
                "parent(third_child_call: true, log: RefCell { value: [] }) {\n",
                "  sibling(third_call: false) {\n",  // Original.
                "    child() {}\n",
                "    // child() repeats 1 time(s).\n",
                "  } // sibling().\n",
                "  // sibling() repeats 1 time(s).\n",
                "  sibling(third_call: true) {\n",          // The sibling of interest is different.
                "    child() {}\n",
                "    // child() repeats 2 time(s).\n",      // The difference from the previous siblings.
                "  } // sibling().\n",
                "} // parent().\n",
            ) 
        )
    };
}

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

// C: `repeated_parent()`
// D: `ret_from_cached_func()`
// C:    [parent() { .. } // Optional previous parent(), for the cases when caching starts at the repeated parent() below.
// C:    [// Repeats n time(s)]]
// C: D: parent() {
// C: D:      [sibling() { .. } // Optional sibling for the cases when caching starts upon the call of interest below.
// C: D:      [// Repeats n time(s)]]
// C: D:      // assert_eq!(): 
// C:             If caching started at parent then there is no current (latest) parent in the call log.
//    D:          otherwise the log above, except the sibling's repeat count, is in the call log.
// C: D:      sibling() { // The call of interest.
// C: D:          [..
// C: D:           [// Repeats]]
// C: D:      } // The return under test.
// C: D:      // assert_eq!(): 
// C:             If caching started at parent then there is no current (latest) parent in the call log.
//    D:          otherwise
//    D:              if there is a previous sibling then
//    D:                  if siblings above are equal then no latest sibling and no previous sibling's repeat count in the call log.
//    D:                  otherwise (the siblings above differ) the whole log above.
//    -               otherwise (no previous sibling)
//    -                   Tested in test B. Do nothing in this test.
// C: D: }
// C:    // Flush the log (flush the `parent()` repeat count to the call log).
// C: D: // assert_eq!(): 
// C:         If caching started at parent then 
// [TODO:]        if parents differ then the whole log above,
// C:             otherwise 
// C:                 parent() { .. }
// C:                 // Repeats n+1 time(s).     // The key fragment: `n+1`. 
//    D:      otherwise (caching didn't start at parent)
//    D:          the whole log above.
#[test]
fn repeated_parent() {
    // The instrumented functions that will generate the call log:
    #[loggable]
    fn child() {}
    #[loggable]
    fn sibling() {
        child();
        child(); // Repeats (generates the repeat count in the call log). 
    }
    // TODO: Suppress the `log` param logging when the suppression is implemented.
    #[loggable]
    fn parent(parent_number: usize, log: Rc<RefCell<Vec<u8>>>) {
        sibling(); // Original.

        sibling(); // Repeats.
        if parent_number == 2 {
            // Caching started at parent, there is no current (latest) parent in the call log.
            #[rustfmt::skip]
            unsafe {
                let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                assert_eq!(
                    log_contents,
                    concat!(
                        "parent(parent_number: 0, log: RefCell { value: [] }) {\n", // Original.
                        "  sibling() {\n",
                        "    child() {}\n",
                        "    // child() repeats 1 time(s).\n",
                        "  } // sibling().\n",
                        "  // sibling() repeats 2 time(s).\n",
                        "} // parent().\n",
                        // There is no previous parent's repeat count in the call log.
                        // There is no current (latest) parent in the call log.
                    ) 
                )
            };
        }

        sibling(); // Of interest. The return under test.
        if parent_number == 2 {
            // Caching started at parent, there is no current (latest) parent in the call log.
            #[rustfmt::skip]
            unsafe {
                let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                assert_eq!(
                    log_contents,
                    concat!(
                        "parent(parent_number: 0, log: RefCell { value: [] }) {\n", // Original.
                        "  sibling() {\n",
                        "    child() {}\n",
                        "    // child() repeats 1 time(s).\n",
                        "  } // sibling().\n",
                        "  // sibling() repeats 2 time(s).\n",
                        "} // parent().\n",
                        // There is no previous parent's repeat count in the call log.
                        // There is no current (latest) parent in the call log.
                    ) 
                )
            };    
        }
    }

    // Mock log writer creation and substitution of the default one:
    let log: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    // The call log generation and some of the checks:
    //   (The argument `log.clone()` below is only applocable to the third call to `parent()`.
    //   For the other calls the arg `Rc::new_uninit()` woul be more efficient.
    //   But to avoid the copy-paste errors, enable the experimenting with the tests and the future extension
    //   the arg `log.clone()` has been used for all the calls)
    parent(0, log.clone());   // Original.
    parent(1, log.clone());   // Repeats.
    parent(2, log.clone());   // Of interest.

    // Flush the log (flush the `parent()` repeat count to the call log).
    THREAD_LOGGER.with(|logger| {
        #[cfg(feature = "singlethreaded")]
        let logger = logger.borrow_mut();

        logger.borrow_mut().flush();
    });

    // Assert:
    //      parent() { .. }
    //      // Repeats n+1 time(s).     // The key fragment: `n+1`. 
    #[rustfmt::skip]
    unsafe {
        let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
        assert_eq!(
            log_contents,
            concat!(
                "parent(parent_number: 0, log: RefCell { value: [] }) {\n", // Original.
                "  sibling() {\n",
                "    child() {}\n",
                "    // child() repeats 1 time(s).\n",
                "  } // sibling().\n",
                "  // sibling() repeats 2 time(s).\n",
                "} // parent().\n",
                "// parent() repeats 2 time(s).\n",      // Repeats `n+1` times.
            ) 
        )
    };
}

