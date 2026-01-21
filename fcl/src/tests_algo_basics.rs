use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::loggable;

use crate as fcl;
use crate::call_log_infra::instances::{THREAD_DECORATOR, THREAD_LOGGER};
#[cfg(feature = "singlethreaded")]
use crate::CallLogger;

// By default the tests run in parallel in different threads. That is why they affect each other's log.
// In particular the log becomes multithreaded, i.e. some tests' log is thread-indented, some function calls
// and repeat counting become interrupted by the thread context switch, etc.
// The test run needs to be serialized.
// By default (`cargo test [-p fcl]`) the lines below are better than nothing:
// serial_test = "3.2.0"     # In Cargo.toml.
// use serial_test::serial;
// #[serial]
// But the test run serialization is still unstable for `serial_test = "3.2.0" .. serial_test = "3.3.1"`
// (the test runs make an impression that during some runs,
// surprizingly _at most one_, test still gets interrupted,
// i.e. its log is different from the expected one, but the call logic is still the same,
// and it runs in an extra thread, i.e. its log is thread-indented).
// The fight with `#[serial]` didn't help. Details are in the mdBook's "Testing" section.
// The default test run command `cargo test [-p fcl]` fails.
// Use `--test-threads=1` (`cargo test [-p fcl] -- --test-threads=1`)
// for reliable test run serialization.
mod singlethreaded {
    use super::*;

    #[test]
    fn basics() {
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));
        {
            #[loggable]
            fn f() {}

            f();
            unsafe { assert_eq!(std::str::from_utf8_unchecked(&*log.borrow()), "f() {}\n") };
        }
        {
            #[loggable]
            fn f() {}

            #[loggable]
            fn g() {
                for _ in 0..3 {
                    f();
                }
            }

            log.borrow_mut().clear();
            g();
            unsafe {
                assert_eq!(
                    std::str::from_utf8_unchecked(&*log.borrow()),
                    concat!(
                        "g() {\n",
                        "  { // Loop body start.\n",
                        "    f() {}\n",
                        "  } // Loop body end.\n",
                        "  // Loop body repeats 2 time(s).\n",
                        "} // g().\n",
                    )
                )
            };
        }
        {
            #[loggable]
            mod t {
                // NOTE: 
                // The `use super::fcl` below is added since the tests are inside of `fcl` iteself.
                // The user does not have to add "use super::fcl" below. TODO (make sure).
                use super::fcl; // `#[loggable]` adds items referring `fcl`.
                fn f(p1: usize, p2: bool) -> f32 {
                    -1.01
                }
                pub fn g() {
                    let _ = f(0, true);
                    let _ = f(1, true);
                    let _ = f(2, false);
                }
            }

            log.borrow_mut().clear();
            t::g();
            unsafe {
                assert_eq!(
                    std::str::from_utf8_unchecked(&*log.borrow()),
                    concat!(
                        "t::g() {\n",
                        "  t::f(p1: 0, p2: true) {} -> -1.01\n",
                        "  // t::f() repeats 2 time(s).\n",
                        "} // t::g().\n",
                    )
                )
            };
        }
    }

    enum CallOptions {
        EmptyLoopBodies {
            one_loop: bool,
            iter_count: (usize, usize),
        },
        SingleEmptySibling {
            iter_count: (usize, usize),
        },
        SingleNonEmptySiblingWithEmptyChild {
            sibling_is_call: bool,
            child_has_loopbody: bool,
        },
        SingleNonEmptySiblingWithNonEmptyChild,
        RepeatingSibling {
            call_child: bool,
        },
        LoopBodyWithSingleChild {
            invocation_count: usize,
            child_repeats: bool,
        },
        LoopBodyWithDiffChildren {
            same_child: bool,
            child_repeats: bool,
            loop_body_count: usize,
        },
    }
    type Calls = Option<CallOptions>;

    /// Instrumented function that, upon invocation, generates the function call log.
    #[loggable]
    fn parent(calls: Calls) {
        // // # Empty calls ("empty" means has no nested calls).
        // parent() {} // Empty call.
        let Some(call_options) = calls else { return };

        match call_options {
            // // # Empty calls.
            // // ## Calls with empty loop bodies and corresponding returns.
            // parent() {} // Call with single empty loop body ("single" means non-repeating).
            // parent() {} // Call with 1 loop with repeating empty body.
            // parent() {} // Call with 2 loops, each with single empty body.
            // parent() {} // Call with 2 loops, repeating and single body correspondingly.
            // parent() {} // Call with 2 loops, single and repeating body correspondingly.
            // parent() {} // Call with 2 loops with repeating body each.
            CallOptions::EmptyLoopBodies {
                one_loop,
                iter_count: (iter_count_a, iter_count_b),
            } => {
                for _ in 0..iter_count_a {}
                if !one_loop {
                    for _ in 0..iter_count_b {}
                }
            }
            // // # Non-empty calls.
            // // ## Single empty sibling.
            // parent() {
            //     sibling_a() {}  // Single empty sibling.
            // }
            // parent() {
            //     sibling_a() {}  // Single sibling with an empty loop body.
            // }
            // parent() {
            //     sibling_a() {}  // Single sibling with multiple loops with an empty body each.
            // }
            CallOptions::SingleEmptySibling {
                iter_count: (iter_count_a, iter_count_b),
            } => {
                fn sibling_a(iter_count_a: usize, iter_count_b: usize) {
                    for _ in 0..iter_count_a {}
                    for _ in 0..iter_count_b {}
                }
                sibling_a(iter_count_a, iter_count_b)
            }

            // // ## Single non-empty sibling with an empty child.
            // parent() {
            //     sibling_a() { // Single non-empty sibling.
            //         child_a() {} // Empty child.
            //     }
            // }
            // parent() {
            //     sibling_a() { // Single non-empty sibling.
            //         child_a() {} // Child with empty loop bodies.
            //     }
            // }
            // parent() {
            //     { // Single non-empty loop body (sibling is a loop body).
            //         child_a() {} // Empty child.
            //     }
            // }
            // parent() {
            //     { // Single non-empty loop body (sibling is a loop body).
            //         child_a() {} // Child with empty loop bodies.
            //     }
            // }
            CallOptions::SingleNonEmptySiblingWithEmptyChild {
                sibling_is_call,
                child_has_loopbody,
            } => {
                fn child(child_has_loopbody: bool) {
                    if child_has_loopbody {
                        for _ in 0..2 {}
                    }
                }

                if sibling_is_call {
                    fn sibling(child_has_loopbody: bool) {
                        child(child_has_loopbody);
                    }

                    sibling(child_has_loopbody)
                } else {
                    // Sibling is a loop body.
                    for _ in 0..1 {
                        child(child_has_loopbody);
                    }
                }
            }

            // // ## Single non-empty sibling with a non-empty child.
            // parent() {
            //     sibling_a() { // Single non-empty sibling.
            //         child_a() {
            //             grandchild_a() {} // Single empty grandchild.
            //         }
            //     }
            // }
            CallOptions::SingleNonEmptySiblingWithNonEmptyChild => {
                fn grandchild() {}
                fn child() {
                    grandchild();
                }
                fn sibling() {
                    child();
                }

                sibling();
            }
            // ## Repeating sibling.
            // parent() {
            //     sibling_a() {}  // Repeating empty sibling.
            //     // repeats 2 time(s).
            // }
            // parent() {
            //     sibling_a() {   // Repeating non-empty sibling.
            //         child_a() {}
            //     }
            //     // repeats 2 time(s).
            // }
            CallOptions::RepeatingSibling { call_child } => {
                fn child() {}
                fn sibling(call_child: bool) {
                    if call_child {
                        child();
                    }
                }

                sibling(call_child);
                sibling(call_child);
                sibling(call_child);
            }

            // ## Loop body with a single child.
            // parent() {
            //     { // Single non-empty loop body.
            //         child() {}
            //     }
            // }
            // parent() {
            //     { // Repeating loop body.
            //         child() {}
            //     }
            //     // Loop body repeats 2 time(s).
            // } // Return after a repeating non-empty loop body.
            CallOptions::LoopBodyWithSingleChild {
                invocation_count,
                child_repeats,
            } => {
                fn child() {}

                for _ in 0..invocation_count {
                    child();
                    if child_repeats {
                        child()
                    }
                }
            }
            CallOptions::LoopBodyWithDiffChildren {
                same_child,
                child_repeats,
                loop_body_count,
            } => {
                // ## Single or repeating loop body with differing children or their internals.
                // parent() {
                //   {
                //     child_a() {
                //       grandchild_a() {}
                //     }
                //     [// Repeats]
                //     child_a() {
                //       grandchild_b() {}
                //     }
                //     [// Repeats]
                //   }
                //   [// Repeats]
                // }
                // parent() {
                //   {
                //     child_a() { ... }
                //     [// Repeats]
                //     child_b() {}
                //     [// Repeats]
                //   }
                //   [// Repeats]
                // }
                fn grandchild_a() {}
                fn grandchild_b() {}
                fn child_a(default_grandchild: bool) {
                    if default_grandchild {
                        grandchild_a()
                    } else {
                        grandchild_b()
                    }
                }
                fn child_b() {}

                // Single or repeating loop body:
                for _ in 0..loop_body_count {
                    child_a(true);
                    if child_repeats {
                        child_a(true);
                    }

                    if same_child {
                        child_a(false) // Same child but different internals.
                    } else {
                        child_b() // Different child.
                    }
                    if child_repeats {
                        if same_child {
                            child_a(false)
                        } else {
                            child_b()
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn empty_calls() {
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        // # Empty calls.
        parent(None); // parent() {} // Empty call (no nested calls).

        // ## Calls with empty loop bodies (empty: has no nested calls) and corresponding returns.
        // parent() {} // Call with single empty loop body (single: non-repeating).
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: true,
            iter_count: (1, 0),
        }));
        // parent() {} // Call with 1 loop of repreating empty body.
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: true,
            iter_count: (3, 0),
        }));
        // parent() {} // Call with 2 loops, each with single empty body.
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: false,
            iter_count: (1, 1),
        }));
        // parent() {} // Call with 2 loops, repeating and single body correspondingly.
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: false,
            iter_count: (3, 1),
        }));
        // parent() {} // Call with 2 loops, single and repeating body correspondingly.
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: false,
            iter_count: (1, 2),
        }));
        // parent() {} // Call with 2 loops with repeating body each.
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: false,
            iter_count: (2, 3),
        }));

        // Flush the log:
        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "singlethreaded")]
            let logger = logger.borrow_mut();

            // The test output ends with the repeat count that needs to be flushed.
            logger.borrow_mut().flush();
        });

        #[rustfmt::skip]
        unsafe {
            // Release the borrow for the subsequent panic upon assertion failure
            // (in case the test fails):
            let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

            assert_eq!(
                result_log,
                concat!(
                    "parent(calls: ?) {}\n", 
                    "// parent() repeats 6 time(s).\n", 
                )
            )
        };
    }

    #[test]
    fn single_empty_sibling() {
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        // // ## Single empty sibling.
        // parent() {
        //     sibling_a() {}  // Single empty sibling.
        // }
        // parent() {
        //     sibling_a() {}  // Single sibling with an empty loop body.
        // }
        // parent() {
        //     sibling_a() {}  // Single sibling with multiple loops with an empty body each.
        // }
        parent(Some(CallOptions::SingleEmptySibling { iter_count: (0, 0) }));
        parent(Some(CallOptions::SingleEmptySibling { iter_count: (1, 0) }));
        parent(Some(CallOptions::SingleEmptySibling { iter_count: (3, 2) }));

        // Flush the log:
        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "singlethreaded")]
            let logger = logger.borrow_mut();

            // The test output ends with the repeat count that needs to be flushed.
            logger.borrow_mut().flush();
        });

        #[rustfmt::skip]
        unsafe {
            // Release the borrow for the subsequent panic upon assertion failure
            // (in case the test fails):
            let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

            assert_eq!(
                result_log,
                concat!(
                    "parent(calls: ?) {\n",
                    "  parent()::sibling_a(iter_count_a: 0, iter_count_b: 0) {}\n", 
                    "} // parent().\n",
                    "// parent() repeats 2 time(s).\n"
                )
            )
        };
    }

    #[test]
    fn single_nonempty_sibling_with_empty_child() {
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        // // ## Single non-empty sibling with an empty child.
        // parent() {
        //     sibling_a() { // Single non-empty sibling.
        //         child_a() {} // Empty child.
        //     }
        // }
        // parent() {
        //     sibling_a() { // Single non-empty sibling.
        //         child_a() {} // Child with empty loop bodies.
        //     }
        // }
        // parent() {
        //     { // Single non-empty loop body.
        //         child_a() {} // Empty child.
        //     }
        // }
        // parent() {
        //     { // Single non-empty loop body.
        //         child_a() {} // Child with empty loop bodies.
        //     }
        // }
        parent(Some(CallOptions::SingleNonEmptySiblingWithEmptyChild {
            sibling_is_call: true,
            child_has_loopbody: false,
        }));
        parent(Some(CallOptions::SingleNonEmptySiblingWithEmptyChild {
            sibling_is_call: true,
            child_has_loopbody: true,
        }));
        parent(Some(CallOptions::SingleNonEmptySiblingWithEmptyChild {
            sibling_is_call: false,
            child_has_loopbody: false,
        }));
        parent(Some(CallOptions::SingleNonEmptySiblingWithEmptyChild {
            sibling_is_call: false,
            child_has_loopbody: true,
        }));

        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "singlethreaded")]
            let logger = logger.borrow_mut();

            // The test output ends with the repeat count that needs to be flushed.
            logger.borrow_mut().flush();
        });

        #[rustfmt::skip]
        unsafe {
            // Release the borrow for the hook of a possible subsequent panic upon assertion failure
            // (in case the test fails):
            let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

            assert_eq!(
                result_log,
                concat!(
                    "parent(calls: ?) {\n", 
                    "  parent()::sibling(child_has_loopbody: false) {\n",
                    "    parent()::child(child_has_loopbody: false) {}\n",
                    "  } // parent()::sibling().\n",
                    "} // parent().\n",
                    "// parent() repeats 1 time(s).\n", 
                    "parent(calls: ?) {\n", 
                    "  { // Loop body start.\n",
                    "    parent()::child(child_has_loopbody: false) {}\n",
                    "  } // Loop body end.\n",
                    "} // parent().\n",
                    "// parent() repeats 1 time(s).\n", 
                )
            )
        };
    }

    #[test]
    fn single_nonempty_sibling_with_nonempty_child() {
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        parent(Some(CallOptions::SingleNonEmptySiblingWithNonEmptyChild));
        // // ## Single non-empty sibling with a non-empty child.
        // parent() {
        //     sibling() { // Single non-empty sibling.
        //         child() {
        //             grandchild() {} // Single empty grandchild.
        //         }
        //     }
        // }

        #[rustfmt::skip]
        unsafe {
            // Release the borrow for the hook of a possible subsequent panic upon assertion failure
            // (in case the test fails):
            let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

            assert_eq!(
                result_log,
                concat!(
                    "parent(calls: ?) {\n", 
                    "  parent()::sibling() {\n",
                    "    parent()::child() {\n",
                    "      parent()::grandchild() {}\n",
                    "    } // parent()::child().\n",
                    "  } // parent()::sibling().\n",
                    "} // parent().\n",
                )
            )
        };
    }

    #[test]
    fn repeating_sibling() {
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        parent(Some(CallOptions::RepeatingSibling { call_child: false }));
        parent(Some(CallOptions::RepeatingSibling { call_child: true }));
        // ## Repeating sibling.
        // parent() {
        //     sibling_a() {}  // Repeating empty sibling.
        //     // repeats 2 time(s).
        // }
        // parent() {
        //     sibling_a() {   // Repeating non-empty sibling.
        //         child_a() {}
        //     }
        //     // repeats 2 time(s).
        // }

        #[rustfmt::skip]
        unsafe {
            // Release the borrow for the hook of a possible subsequent panic upon assertion failure
            // (in case the test fails):
            let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

            assert_eq!(
                result_log,
                concat!(
                    "parent(calls: ?) {\n", 
                    "  parent()::sibling(call_child: false) {}\n", 
                    "  // parent()::sibling() repeats 2 time(s).\n", 
                    "} // parent().\n", 
                    "parent(calls: ?) {\n", 
                    "  parent()::sibling(call_child: true) {\n", 
                    "    parent()::child() {}\n", 
                    "  } // parent()::sibling().\n", 
                    "  // parent()::sibling() repeats 2 time(s).\n", 
                    "} // parent().\n", 
                )
            )
        };
    }

    #[test]
    fn loop_body_with_single_child() {
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        parent(Some(CallOptions::LoopBodyWithSingleChild {
            invocation_count: 1,
            child_repeats: false,
        }));
        parent(Some(CallOptions::LoopBodyWithSingleChild {
            invocation_count: 3,
            child_repeats: false,
        }));
        // ## Loop body with a single child.
        // parent() {
        //     { // Single non-empty loop body.
        //         child() {}
        //     }
        // }
        // parent() {
        //     { // Repeating loop body.
        //         child() {}
        //     }
        //     // Loop body repeats 2 time(s).
        // } // Return after a repeating non-empty loop body.

        #[rustfmt::skip]
        unsafe {
            // Release the borrow for the hook of a possible subsequent panic upon assertion failure
            // (in case the test fails):
            let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

            assert_eq!(
                result_log,
                concat!(
                    "parent(calls: ?) {\n",
                    "  { // Loop body start.\n",
                    "    parent()::child() {}\n",
                    "  } // Loop body end.\n",
                    "} // parent().\n",
                    "parent(calls: ?) {\n",
                    "  { // Loop body start.\n",
                    "    parent()::child() {}\n",
                    "  } // Loop body end.\n",
                    "  // Loop body repeats 2 time(s).\n",
                    "} // parent().\n",
                )
            )
        };
    }

    #[test]
    fn loop_body_with_repeating_child() {
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        parent(Some(CallOptions::LoopBodyWithSingleChild {
            invocation_count: 1,
            child_repeats: true,
        }));
        parent(Some(CallOptions::LoopBodyWithSingleChild {
            invocation_count: 3,
            child_repeats: true,
        }));
        // ## Loop body with a repeating child.
        // parent() {
        //     { // Single non-empty loop body.
        //         child() {}
        //         // Repeats
        //     }
        // }
        // parent() {
        //     { // Repeating loop body.
        //         child() {}
        //         // Repeats
        //     }
        //     // Loop body repeats 2 time(s).
        // }

        #[rustfmt::skip]
        unsafe {
            // Release the borrow for the hook of a possible subsequent panic upon assertion failure
            // (in case the test fails):
            let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

            assert_eq!(
                result_log,
                concat!(
                    "parent(calls: ?) {\n",
                    "  { // Loop body start.\n",
                    "    parent()::child() {}\n",
                    "    // parent()::child() repeats 1 time(s).\n",
                    "  } // Loop body end.\n",
                    "} // parent().\n",
                    "parent(calls: ?) {\n",
                    "  { // Loop body start.\n",
                    "    parent()::child() {}\n",
                    "    // parent()::child() repeats 1 time(s).\n",
                    "  } // Loop body end.\n",
                    "  // Loop body repeats 2 time(s).\n",
                    "} // parent().\n",
                )
            )
        };
    }

    fn loop_body_with_diff_children(loop_body_count: usize) {
        parent(Some(CallOptions::LoopBodyWithDiffChildren {
            same_child: true,
            child_repeats: false,
            loop_body_count,
        }));
        // parent() {
        //   {
        //     child_a() {
        //       grandchild_a() {}
        //     }
        //     child_a() {
        //       grandchild_b() {}
        //     }
        //   }
        //   [ // Repeats ]
        // }

        parent(Some(CallOptions::LoopBodyWithDiffChildren {
            same_child: true,
            child_repeats: true,
            loop_body_count,
        }));
        // parent() {
        //   { // SingleLoopBody
        //     child_a() {
        //       grandchild_a() {}
        //     }
        //     // Repeats
        //     child_a() {
        //       grandchild_b() {}
        //     }
        //     // Repeats
        //   }
        //   [ // Repeats ]
        // }

        parent(Some(CallOptions::LoopBodyWithDiffChildren {
            same_child: false,
            child_repeats: false,
            loop_body_count,
        }));
        // parent() {
        //   { // SingleLoopBody
        //     child_a() {}
        //     child_b() {}
        //   }
        //   [ // Repeats ]
        // }

        parent(Some(CallOptions::LoopBodyWithDiffChildren {
            same_child: false,
            child_repeats: true,
            loop_body_count,
        }));
        // parent() {
        //   {
        //     child_a() {}
        //     // Repeats
        //     child_b() {}
        //     // Repeats
        //   }
        //   [ // Repeats ]
        // }
    }

    #[test]
    fn single_loop_body_with_diff_children() {
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        // ## Single loop body with differing children or their internals.
        loop_body_with_diff_children(1);

        #[rustfmt::skip]
        unsafe {
            // Release the borrow for the hook of a possible subsequent panic upon assertion failure
            // (in case the test fails):
            let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

            assert_eq!(
                result_log,
                concat!(
                    // ## Single loop body with differing children or their internals.
                    "parent(calls: ?) {\n", 
                    "  { // Loop body start.\n",
                    "    parent()::child_a(default_grandchild: true) {\n",
                    "      parent()::grandchild_a() {}\n",
                    "    } // parent()::child_a().\n",
                    "    parent()::child_a(default_grandchild: false) {\n",
                    "      parent()::grandchild_b() {}\n",
                    "    } // parent()::child_a().\n",
                    "  } // Loop body end.\n",
                    "} // parent().\n",
                    "parent(calls: ?) {\n",
                    "  { // Loop body start.\n",
                    "    parent()::child_a(default_grandchild: true) {\n",
                    "      parent()::grandchild_a() {}\n",
                    "    } // parent()::child_a().\n",
                    "    // parent()::child_a() repeats 1 time(s).\n",
                    "    parent()::child_a(default_grandchild: false) {\n",
                    "      parent()::grandchild_b() {}\n",
                    "    } // parent()::child_a().\n",
                    "    // parent()::child_a() repeats 1 time(s).\n",
                    "  } // Loop body end.\n",
                    "} // parent().\n",
                    "parent(calls: ?) {\n",
                    "  { // Loop body start.\n",
                    "    parent()::child_a(default_grandchild: true) {\n",
                    "      parent()::grandchild_a() {}\n",
                    "    } // parent()::child_a().\n",
                    "    parent()::child_b() {}\n",
                    "  } // Loop body end.\n",
                    "} // parent().\n",
                    "parent(calls: ?) {\n",
                    "  { // Loop body start.\n",
                    "    parent()::child_a(default_grandchild: true) {\n",
                    "      parent()::grandchild_a() {}\n",
                    "    } // parent()::child_a().\n",
                    "    // parent()::child_a() repeats 1 time(s).\n",
                    "    parent()::child_b() {}\n",
                    "    // parent()::child_b() repeats 1 time(s).\n",
                    "  } // Loop body end.\n",
                    "} // parent().\n"
                )
            )
        };
    }

    #[test]
    fn repeating_loop_body_with_diff_children() {
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        // ## Repeating loop body with differing children or their internals.
        loop_body_with_diff_children(3);

        #[rustfmt::skip]
        unsafe {
            // Release the borrow for the hook of a possible subsequent panic upon assertion failure
            // (in case the test fails):
            let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

            assert_eq!(
                result_log,
                concat!(
                    // ## Single loop body with differing children or their internals.
                    "parent(calls: ?) {\n", 
                    "  { // Loop body start.\n",
                    "    parent()::child_a(default_grandchild: true) {\n",
                    "      parent()::grandchild_a() {}\n",
                    "    } // parent()::child_a().\n",
                    "    parent()::child_a(default_grandchild: false) {\n",
                    "      parent()::grandchild_b() {}\n",
                    "    } // parent()::child_a().\n",
                    "  } // Loop body end.\n",
                    "  // Loop body repeats 2 time(s).\n",
                    "} // parent().\n",
                    "parent(calls: ?) {\n",
                    "  { // Loop body start.\n",
                    "    parent()::child_a(default_grandchild: true) {\n",
                    "      parent()::grandchild_a() {}\n",
                    "    } // parent()::child_a().\n",
                    "    // parent()::child_a() repeats 1 time(s).\n",
                    "    parent()::child_a(default_grandchild: false) {\n",
                    "      parent()::grandchild_b() {}\n",
                    "    } // parent()::child_a().\n",
                    "    // parent()::child_a() repeats 1 time(s).\n",
                    "  } // Loop body end.\n",
                    "  // Loop body repeats 2 time(s).\n",
                    "} // parent().\n",
                    "parent(calls: ?) {\n",
                    "  { // Loop body start.\n",
                    "    parent()::child_a(default_grandchild: true) {\n",
                    "      parent()::grandchild_a() {}\n",
                    "    } // parent()::child_a().\n",
                    "    parent()::child_b() {}\n",
                    "  } // Loop body end.\n",
                    "  // Loop body repeats 2 time(s).\n",
                    "} // parent().\n",
                    "parent(calls: ?) {\n",
                    "  { // Loop body start.\n",
                    "    parent()::child_a(default_grandchild: true) {\n",
                    "      parent()::grandchild_a() {}\n",
                    "    } // parent()::child_a().\n",
                    "    // parent()::child_a() repeats 1 time(s).\n",
                    "    parent()::child_b() {}\n",
                    "    // parent()::child_b() repeats 1 time(s).\n",
                    "  } // Loop body end.\n",
                    "  // Loop body repeats 2 time(s).\n",
                    "} // parent().\n"
                )
            )
        };
    }

}
