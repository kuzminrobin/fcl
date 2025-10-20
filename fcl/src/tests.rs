use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::loggable;

use crate as fcl;
use crate::call_log_infra::instances::THREAD_DECORATOR;

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
            use super::fcl;
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
}
type Calls = Option<CallOptions>;

#[loggable]
fn parent(calls: Calls) {
    // // # Empty calls ("empty" means has no nested calls).
    // parent_?() {} // Empty call.
    let Some(call_options) = calls else { return };

    match call_options {
        // // # Empty calls.
        // // ## Calls with empty loop bodies and corresponding returns.
        // parent_?() {} // Call with single empty loop body ("single" means non-repeating).
        // parent_?() {} // Call with 1 loop with repreating empty body.
        // parent_?() {} // Call with 2 loops, each with single empty body.
        // parent_?() {} // Call with 2 loops, repeating and single body correspondingly.
        // parent_?() {} // Call with 2 loops, single and repeating body correspondingly.
        // parent_?() {} // Call with 2 loops with repeating body each.
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
        // parent_?() {
        //     sibling_a() {}  // Single empty sibling.
        // }
        // parent_?() {
        //     sibling_a() {}  // Single sibling with an empty loop body.
        // }
        // parent_?() {
        //     sibling_a() {}  // Single sibling with multiple loops with an empty body each.
        // }
        CallOptions::SingleEmptySibling {
            iter_count: (iter_count_a, iter_count_b),
        } => {
            // TODO: What about `sibling_a` mentioned in comments?
            for _ in 0..iter_count_a {}
            for _ in 0..iter_count_b {}
        }

        // // ## Single non-empty sibling with an empty child.
        // parent_?() {
        //     sibling_a() { // Single non-empty sibling.
        //         child_a() {} // Empty child.
        //     }
        // }
        // parent_?() {
        //     sibling_a() { // Single non-empty sibling.
        //         child_a() {} // Child with empty loop bodies.
        //     }
        // }
        // parent_?() {
        //     { // Single non-empty loop body (sibling is a loop body).
        //         child_a() {} // Empty child.
        //     }
        // }
        // parent_?() {
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
    }
}

#[test]
fn single_nonempty_sibling_with_empty_child() {
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    #[loggable]
    fn call_all() {
        // // ## Single non-empty sibling with an empty child.
        // parent_?() {
        //     sibling_a() { // Single non-empty sibling.
        //         child_a() {} // Empty child.
        //     }
        // }
        // parent_?() {
        //     sibling_a() { // Single non-empty sibling.
        //         child_a() {} // Child with empty loop bodies.
        //     }
        // }
        // parent_?() {
        //     { // Single non-empty loop body.
        //         child_a() {} // Empty child.
        //     }
        // }
        // parent_?() {
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
    }

    call_all();

    #[rustfmt::skip]
    unsafe {
        // Release the borrow for the hook of a possible subsequent panic upon assertion failure:
        let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

        assert_eq!(
            result_log,
            concat!(
                "call_all() {\n", 
                "  parent(calls: ?) {\n", 
                "    parent()::sibling(child_has_loopbody: false) {\n",
                "      parent()::child(child_has_loopbody: false) {}\n",
                "    } // parent()::sibling().\n",
                "  } // parent().\n",
                "  // parent() repeats 1 time(s).\n", 
                "  parent(calls: ?) {\n", 
                "    { // Loop body start.\n",
                "      parent()::child(child_has_loopbody: false) {}\n",
                "    } // Loop body end.\n",
                "  } // parent().\n",
                "  // parent() repeats 1 time(s).\n", 
                "} // call_all().\n"
            )
        )
    };
}

#[test]
fn single_empty_sibling() {
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    #[loggable]
    fn call_all() {
        // // ## Single empty sibling.
        // parent_?() {
        //     sibling_a() {}  // Single empty sibling.
        // }
        // parent_?() {
        //     sibling_a() {}  // Single sibling with an empty loop body.
        // }
        // parent_?() {
        //     sibling_a() {}  // Single sibling with multiple loops with an empty body each.
        // }
        parent(Some(CallOptions::SingleEmptySibling { iter_count: (0, 0) }));
        parent(Some(CallOptions::SingleEmptySibling { iter_count: (1, 0) }));
        parent(Some(CallOptions::SingleEmptySibling { iter_count: (3, 2) }));
    } // Flush the log.

    call_all();

    #[rustfmt::skip]
    unsafe {
        // Release the borrow for the subsequent panic upon assertion failure:
        let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

        assert_eq!(
            result_log,
            concat!(
                "call_all() {\n", 
                "  parent(calls: ?) {}\n", 
                "  // parent() repeats 2 time(s).\n", 
                "} // call_all().\n"
            )
        )
    };
}

#[test]
fn empty_calls() {
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    #[loggable]
    fn call_all() {
        // # Empty calls.
        parent(None); // parent_?() {} // Empty call (no nested calls).

        // ## Calls with empty loop bodies (empty: has no nested calls) and corresponding returns.
        // parent_?() {} // Call with single empty loop body (single: non-repeating).
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: true,
            iter_count: (1, 0),
        }));
        // parent_?() {} // Call with 1 loop of repreating empty body.
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: true,
            iter_count: (3, 0),
        }));
        // parent_?() {} // Call with 2 loops, each with single empty body.
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: false,
            iter_count: (1, 1),
        }));
        // parent_?() {} // Call with 2 loops, repeating and single body correspondingly.
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: false,
            iter_count: (3, 1),
        }));
        // parent_?() {} // Call with 2 loops, single and repeating body correspondingly.
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: false,
            iter_count: (1, 2),
        }));
        // parent_?() {} // Call with 2 loops with repeating body each.
        parent(Some(CallOptions::EmptyLoopBodies {
            one_loop: false,
            iter_count: (2, 3),
        }));
    } // Flush the log.

    call_all();

    #[rustfmt::skip]
    unsafe {
        // Release the borrow for the subsequent panic upon assertion failure:
        let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));

        assert_eq!(
            result_log,
            concat!(
                "call_all() {\n", 
                "  parent(calls: ?) {}\n", 
                "  // parent() repeats 6 time(s).\n", 
                "} // call_all().\n"
            )
        )
    };

    {
        /*
                // # Empty calls.
                parent_?() {} // Empty call (no nested calls).

                // ## Calls with empty loop bodies (empty: has no nested calls) and corresponding returns.
                parent_?() {} // Call with single empty loop body (single: non-repeating).
                parent_?() {} // Call with 1 loop of repreating empty body.
                parent_?() {} // Call with 2 loops, each with single empty body.
                parent_?() {} // Call with 2 loops, repeating and single body correspondingly.
                parent_?() {} // Call with 2 loops, single and repeating body correspondingly.
                parent_?() {} // Call with 2 loops with repeating body each.

                // # Non-empty calls.
                // ## Single empty sibling.
                parent_?() {
                    sibling_a() {}  // Single empty sibling.
                } // Return after a non-repeating empty sibling.
                parent_?() {
                    sibling_a() {}  // Single sibling with an empty loop body.
                }
                parent_?() {
                    sibling_a() {}  // Single sibling with multiple loops with an empty body each.
                }

                // ## Single non-empty sibling with an empty child.
                parent_?() {
                    sibling_a() { // Single non-empty sibling.
                        child_a() {} // Empty child.
                    }
                }
                parent_?() {
                    sibling_a() { // Single non-empty sibling.
                        child_a() {} // Child with empty loop bodies.
                    }
                }
                parent_?() {
                    { // Single non-empty loop body.
                        child_a() {} // Empty child.
                    }
                }
                parent_?() {
                    { // Single non-empty loop body.
                        child_a() {} // Child with empty loop bodies.
                    }
                }
        >
        >>
                // ## Single non-empty sibling with a non-empty child.
                parent_?() {
                    sibling_a() { // Single non-empty sibling.
                        child_a() {
                            grandc_a() {} // Single empty grandchild.
                        }
                    }
                }

                ## Repeating sibling.
                parent_?() {
                    sibling_a() {}  // Repeating empty sibling.
                    // repeats 2 time(s).
                }
                parent_?() {
                    sibling_a() {   // Repeating non-empty sibling.
                        child_a() {}
                    }
                    // repeats 2 time(s).
                }

                ## Loop body with a single sibling.
                parent_?() {
                    { // Single non-empty loop body.
                        sibling_a() {}
                    }
                }
                parent_?() {
                    { // Repeating loop body.
                        sibling_a() {}
                    }
                    // Loop body repeats 2 time(s).
                } // Return after a repeating non-empty loop body.

                ## Loop body with a repeating sibling.
                parent_?() {
                    { // Single non-empty loop body.
                        sibling_a() {}
                        // Repeats
                    }
                }
                parent_?() {
                    { // Repeating loop body.
                        sibling_a() {}
                        // Repeats
                    }
                    // Loop body repeats 2 time(s).
                }

                ## Single loop body with differing siblings.
                parent_?() {
                    {
                        sibling_a() {
                            child_a() {}
                        }
                        sibling_a() {
                            child_b() {}
                        }
                    }
                }
                parent_?() {
                    {
                        sibling_a() {
                            child_a() {}
                        }
                        // Repeats
                        sibling_a() {
                            child_b() {}
                        }
                        // Repeats
                    }
                }
                parent_?() {
                    {
                        sibling_a() {}
                        sibling_b() {}
                    }
                }
                parent_?() {
                    {
                        sibling_a() {}
                        // Repeats
                        sibling_b() {}
                        // Repeats
                    }
                }

                ## Repeating loop body with differing siblings.
                parent_?() {
                    {
                        sibling_a() {
                            child_a() {}
                        }
                        sibling_a() {
                            child_b() {}
                        }
                    }
                    // Repeats
                }
                parent_?() {
                    {
                        sibling_a() {
                            child_a() {}
                        }
                        // Repeats
                        sibling_a() {
                            child_b() {}
                        }
                        // Repeats
                    }
                    // Repeats
                }
                parent_?() {
                    {
                        sibling_a() {}
                        sibling_b() {}
                    }
                    // Repeats
                }
                parent_?() {
                    {
                        sibling_a() {}
                        // Repeats
                        sibling_b() {}
                        // Repeats
                    }
                    // Repeats
                }








                parent_?() {
                    { // Single loop body.
                        sibling_a() {}  // Empty sibling repeating in a loop.
                    }
                } // Return after a single non-empty loop body.
                parent_?() {
                    { // Repeating loop body.
                        sibling_a() {}  // Empty sibling repeating in a loop.
                    }
                    // Loop body repeats 2 time(s).
                } // Return after a repeating non-empty loop body.
                parent_?() {
                    { // Loop
                        sibling_a() {   // Repeating non-empty sibling.
                            child_a() {}
                        }
                    }
                    // repeats 2 time(s).
                } // Return after a repeating empty sibling.

                parent_a() {
                    sibling_a() {}
                    [// sibling_a() repeats 1 time(s).   // sibling_a() {}]
                    sibling_b() {}
                }
                parent_b() {
                    { // Loop body start.
                        ?
                    } // Loop body end.
                    [// Loop body repeats 1 time(s).]
                    sibling_a() {}
                }
                */
        log.borrow_mut().clear();
    }
}
