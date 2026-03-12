use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::{loggable, non_loggable};

#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::{THREAD_DECORATOR};

mod common;

// High-level logic to test:
//
// (Removing the childless loop bodies is already tested in "fcl\tests\add_loopbody_start.rs")
//
// If caching is inactive (stopped upon {child in initial loopbody} or flush) {
//      Log the last child's non-flushed repeat count.
//      Log the loop body end.
// }
// if loop body is identical
//      remove it and increment the repeat count of the previous one.
// else {
//      Log the previous_sibling's non-flushed repeat count.
//      Log the subtree of the ending loop body.
// }
//
// Test cases:
//
// parent() {                        // Instruments the loop bodies.
//   {                               // Initial loop body. Starts being cached.
//     child() {}                    // Stops caching for the initial loop body.
//     // child() repeats 2 time(s). // Not logged yet.
//                                   // Assert: The last child's repeat count is not logged yet.
//   }                               // The loop body end of interest.
//                                   // Assert: The last child's repeat count is logged.
//                                   // Assert: The loop body's end is logged.
//   // Loop body repeats 1 time(s). // Not logged yet. The identical loop body does not affect the log at this moment.
//                                   // Assert: The identical loop body is not logged yet.
//   {                               // Another (differing) loop body. Starts being cached.
//     child() {}                    // Caching continues for the non-initial loop body.
//     // child() repeats 3 time(s). // The difference (`3`) from the other iterations. Not logged yet. Caching continues.
//                                   // Assert: The whole iteration is being cached.
//   }                               // The loop body end of interest.
//                                   // Assert: The previous loop body's rep.count is logged.
//                                   // Assert: The ending loop body's subtree is logged.
// }
#[test]
fn loop_body_end() {
    // Does the assertions between the iterations (before the iteration whose number is passed as an arg):
    fn assert_bofore_iter(next_iter_count: usize, log: Rc<RefCell<Vec<u8>>>) -> bool {
        match next_iter_count {
            0 => (), // Nothing to assert before [0], continue the test execution.
            1 | 2 => {
                // Before iteration [1] and [2]:
                #[rustfmt::skip]
                test_assert!(log, concat!(
                    "loop_instrumenter(..) {\n",
                    "  { // Loop body start.\n",
                    "    child() {}\n",
                    "    // child() repeats 2 time(s).\n",  // Assert: The last child's repeat count is logged in [0].
                    "  } // Loop body end.\n",              // Assert: The loop body [0]'s end is logged.
                    // Assert: The identical loop body is not logged yet after [1].
                ));
            }
            3 => {
                #[rustfmt::skip]
                test_assert!(log, concat!(
                    "loop_instrumenter(..) {\n",
                    "  { // Loop body start.\n",
                    "    child() {}\n",
                    "    // child() repeats 2 time(s).\n",
                    "  } // Loop body end.\n",
                    "  // Loop body repeats 1 time(s).\n",  // Assert: The previous loop body's rep. count is logged.
                    "  { // Loop body start.\n",            // Assert: The ending loop body's subtree is logged.
                    "    child() {}\n",
                    "    // child() repeats 3 time(s).\n",
                    "  } // Loop body end.\n",
                ));
            }
            _ => return false, // Stop the test loop.
        }

        true // Continue the test loop.
    }

    #[loggable]
    fn child() {}

    // Is used to instrument the loops since the loops (expressions and statements) cannot be custom-attributed directly in Rust (yet).
    #[loggable(skip_params)]
    fn loop_instrumenter(log: Rc<RefCell<Vec<u8>>>) {
        let mut iter_count = 0;
        while assert_bofore_iter(iter_count, log.clone()) {
            // All iterations behavior:
            child();
            child();
            child();
            // The difference for a specific iteration:
            if iter_count == 2 {
                child();
            }

            // Assertions inside of the loop bodies:
            match iter_count {
                0 => {
                    #[rustfmt::skip]
                    test_assert!(log, concat!(
                        "loop_instrumenter(..) {\n",
                        "  { // Loop body start.\n",
                        "    child() {}\n",
                        // Assert: The last child's repeat count is not logged yet.
                    ));
                }
                // 1 => (), // Nothing to assert inside of [1].
                2 => {
                    #[rustfmt::skip]
                    test_assert!(log, concat!(
                        "loop_instrumenter(..) {\n",
                        "  { // Loop body start.\n",
                        "    child() {}\n",
                        "    // child() repeats 2 time(s).\n",
                        "  } // Loop body end.\n",
                        // Assert: The whole iteration is being cached 
                        // (and the rep.count of the previous one is not logged yet).
                    ));
                }

                _ => (),
            }
            iter_count += 1;
        }
    }

    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    loop_instrumenter(log.clone());
}

// High-level logic to test:
//
// Induce (by means of a function invoked between the iterations (during condition checking in `while()` loop))
// the flush in between the iterations.
// Make sure the iteration after the flush is logged fully (even if it's identical to the prevuous one)
// in addition to being removed and incrementing the repeat count of the one preceding the flush.
// Make sure the two identical calls (see `f()` below) compare equally
// after the iterations in one of them have been interrupted by a flush:
// e() {
//     f() {
//         { // Loop body start.
//             g() {}
//         } // Loop body end.
//         // Loop body repeats 2 time(s).     // The iterations haven't been interrupted by a flush.
//     } // f().
//     f() {                       // Same-name call. Starts being cached.
//         { // Loop body start.
//             g() {}
//         } // Loop body end.
//                                 // Assert: Repeated `f()` is being cached.
//         // Flush.               // The interruption between the iterations. It should affect the log, but not the call graph.
//                                 // Assert: Repeated `f()` has been flushed.
//         { // Loop body start.   // Despite of the flush, must start being cached (because can potentially be removed if childless).
//             g() {}              // Despite of the call `g()` the loop body must still continue being cached
//                                 // Assert: The loop body is being cached.
//         } // Loop body end.     
//                                 // Assert: The loop body is logged in full (instead of `// Repeats 1 time(s)`) 
//                                 // because of the `flush()` earlier.
//                                 // Despite of having been logged in full, must still get removed from
//                                 // the call graph and inc the repeat count of the prev. iteration.
//         // Loop body repeats 1 time(s).
//     } // f().                   // Despite of having been logged in full, 
//                                 // must still be removed from the call graph (since it's identical to the previous `f()`) 
//                                 // and inc the repeat count of the prev. `f()`.
// } // e().
// // e() repeats 1 time(s).       // The identical repeated call to `e()` needs to happen _uninterrupted_,
//                                 // after which it must compeare equal to the previous `e()`,
//                                 // thus making sure that the {previous `e()` and its internals} are consistent in the call graph,
//                                 // and (repeated `e()`) must get removed from the call graph, inc the repeat count.
//                                 // Assert: "e() repeats 1 time(s)".
#[test]
fn flush_between_iters() {
    #[loggable]
    fn g() {}
    #[loggable(skip_params)]
    fn f(flush: bool, log: Rc<RefCell<Vec<u8>>>) {
        #[non_loggable]
        fn go_on_and_maybe_flush(flush: bool, next_iter_count: usize, log: Rc<RefCell<Vec<u8>>>) -> bool {
            if flush {
                if next_iter_count == 1 {
                    #[rustfmt::skip]
                    test_assert!(log, concat!(
                        "e(..) {\n",
                        "  f(..) {\n",
                        "    { // Loop body start.\n",
                        "      g() {}\n",
                        "    } // Loop body end.\n",
                        "    // Loop body repeats 2 time(s).\n",
                        "  } // f().\n",
                        // Assert: Repeated `f()` is being cached.
                    ));

                    common::flush_log();

                    #[rustfmt::skip]
                    test_assert!(log, concat!(
                        "e(..) {\n",
                        "  f(..) {\n",
                        "    { // Loop body start.\n",
                        "      g() {}\n",
                        "    } // Loop body end.\n",
                        "    // Loop body repeats 2 time(s).\n",
                        "  } // f().\n",
                        "  f(..) {\n",                          // Assert: Repeated `f()` has been flushed.
                        "    { // Loop body start.\n",
                        "      g() {}\n",
                        "    } // Loop body end.\n",
                    ));
                } else if next_iter_count == 2 {
                    #[rustfmt::skip]
                    test_assert!(log, concat!(
                        "e(..) {\n",
                        "  f(..) {\n",
                        "    { // Loop body start.\n",
                        "      g() {}\n",
                        "    } // Loop body end.\n",
                        "    // Loop body repeats 2 time(s).\n",
                        "  } // f().\n",
                        "  f(..) {\n",
                        "    { // Loop body start.\n",
                        "      g() {}\n",
                        "    } // Loop body end.\n",
                        // Flush
                        "    { // Loop body start.\n",  // Assert: The loop body is logged in full (instead of `// Repeats 1 time(s)`).
                        "      g() {}\n",
                        "    } // Loop body end.\n",
                    ));
                }
            }

            next_iter_count < 3 // If >= 3 stop the test loop.
        }

        let mut iter_count = 0;
        while go_on_and_maybe_flush(flush, iter_count, log.clone()) {
            g();
            if flush && (iter_count == 1) {
                #[rustfmt::skip]
                test_assert!(log, concat!(
                    "e(..) {\n",
                    "  f(..) {\n",
                    "    { // Loop body start.\n",
                    "      g() {}\n",
                    "    } // Loop body end.\n",
                    "    // Loop body repeats 2 time(s).\n",
                    "  } // f().\n",
                    "  f(..) {\n",
                    "    { // Loop body start.\n",
                    "      g() {}\n",
                    "    } // Loop body end.\n",
                    // Assert: The loop body is being cached.
                ));
            }
            iter_count += 1;
        }
    }
    #[loggable(skip_params)]
    fn e(e_count: usize, log: Rc<RefCell<Vec<u8>>>) {
        f(false, log.clone()); // Do not flush.

        if e_count == 0 {
            f(true, log.clone()); // Flush during the first call to `e()`.
        } else {
            f(false, log.clone()); // Do not flush otherwise.
        }
    }

    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    e(0, log.clone());
    e(1, log.clone());
    common::flush_log();    // Flush "// e() repeats 1 time(s).\n".

    #[rustfmt::skip]
    test_assert!(log, concat!(
        "e(..) {\n",
        "  f(..) {\n",
        "    { // Loop body start.\n",
        "      g() {}\n",
        "    } // Loop body end.\n",
        "    // Loop body repeats 2 time(s).\n",
        "  } // f().\n",
        "  f(..) {\n",
        "    { // Loop body start.\n",
        "      g() {}\n",
        "    } // Loop body end.\n",
        "    { // Loop body start.\n",
        "      g() {}\n",
        "    } // Loop body end.\n",
        "    // Loop body repeats 1 time(s).\n",
        "  } // f().\n",
        "} // e().\n",
        "// e() repeats 1 time(s).\n",  // Assert: "e() repeats 1 time(s)".
    ));
}

// TODO: Double-check the scenario when upon thread context switch the initial loopbody gets flushed,
// subsequently it has no nested calls, but if its beginning is flushed then its end must be flushed too,
// after which the childless loopbody must be removed from the call graph.
