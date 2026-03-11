use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::loggable;

#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::{THREAD_DECORATOR, THREAD_LOGGER};

// NOTE: Extracting this to a macro rather than a function
// in order to preserve the line numbers (inside of the tests) in the assertion failure reports.
macro_rules! test_assert {
    ($log:ident, $expected_str:expr $(,)?) => {
        unsafe {
            let log_contents = String::from(std::str::from_utf8_unchecked(&*$log.borrow()));
            assert_eq!(log_contents, $expected_str)
        }
    };
}

fn flush_log() {
    // Flush the log:
    THREAD_LOGGER.with(|logger| {
        #[cfg(feature = "singlethreaded")]
        let logger = logger.borrow_mut();

        logger.borrow_mut().flush();
    });
}

// High-level logic to test:
// D:  [parent // Previous parent to activate caching.]
//     parent {  // It's a call or an enclosing loop body. Caching is {inactive, active}.
// C:        Either a previous loop [with [non-]identical children];
// B:        or a call: {function, closure};
// A:        or a previous iteration of the current loop [A: with [non-]identical children]
//
// A,B,C,D:  { // Loop body start that's being tested.

// Test cases:

// D: `loop_in_cached_func()`:
//  parent() {} // Previous parent to activate caching.
//  parent() {  // Repeated call. Caching is active.
//      { // Loop body start that's being tested.
//          f() {}
//      }
//      Assert: The second parent is being cached.
//  }
//  Assert: The identical call increments the repeat count.
//  Assert: The differing call is logged as is.
#[test]
fn loop_in_cached_func() {
    #[loggable]
    fn f() {}
    #[loggable(skip_params)]
    fn loop_instrumenter(log: Rc<RefCell<Vec<u8>>>, call_count: usize) {
        let mut loop_count = 0;
        loop {
            match loop_count {
                1 => {
                    if call_count != 2 {
                        // Call 2 differs - doesn't call `f()` (other calls call `f()` in iteration [1]).
                        f()
                    }
                }
                _ => (),
            }

            loop_count += 1;
            if loop_count > 4 {
                break;
            }
        }

        // Assert: The non-first parent is being cached.
        match call_count {
            0 => {
                #[rustfmt::skip]
                test_assert!(log, concat!(
                    "loop_instrumenter(..) {\n", // The first call is being logged.
                    "  { // Loop body start.\n",
                    "    f() {}\n",
                    "  } // Loop body end.\n",
                ));
            }
            1 => {
                #[rustfmt::skip]
                test_assert!(log, concat!(
                    "loop_instrumenter(..) {\n", // The first call is logged as is.
                    "  { // Loop body start.\n",
                    "    f() {}\n",
                    "  } // Loop body end.\n",
                    "} // loop_instrumenter().\n",
                    // The second (identical) call is beging cached.
                ));
            }
            2 => {
                #[rustfmt::skip]
                test_assert!(log, concat!(
                    "loop_instrumenter(..) {\n", // The first call is logged as is.
                    "  { // Loop body start.\n",
                    "    f() {}\n",
                    "  } // Loop body end.\n",
                    "} // loop_instrumenter().\n",
                    // The second (identical) call has incremented the repeat count. Not yet flushed.
                    // The third (differing) call is being cached.
                ));
            }
            _ => (),
        }
    }

    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    loop_instrumenter(log.clone(), 0);
    loop_instrumenter(log.clone(), 1);
    loop_instrumenter(log.clone(), 2);

    //  Assert: The identical call increments the repeat count.
    //  Assert: The differing call is logged as is.
    #[rustfmt::skip]
    test_assert!(log, concat!(
        "loop_instrumenter(..) {\n", // The first call is logged as is.
        "  { // Loop body start.\n",
        "    f() {}\n",
        "  } // Loop body end.\n",
        "} // loop_instrumenter().\n",
        "// loop_instrumenter() repeats 1 time(s).\n",  // The second (identical) call has incremented the repeat count.
        "loop_instrumenter(..) {}\n",   // The third (differing) call is logged as is (all loop bodies are removed for being childless).
    ));
}

//
// C: `loop_after_loop()`:
// while() {    // Previous loop.
//      f() {}
// }
// // Assert: Previous loop is fully logged and flushed.
// while() {    // Identical or different loop.
//      // Assert: Loop body is cached.
//      f() {
//          // Assert: Caching stops.
//      }
// }
#[test]
fn loop_after_loop() {
    #[loggable(skip_params)]
    fn f(log: Rc<RefCell<Vec<u8>>>, assert: bool) {
        if assert {
            #[rustfmt::skip]
            // Assert: Caching stops.
            test_assert!(log, concat!(
                "  { // Loop body start.\n", 
                "    f(..) {",
            ));
        }
    }
    #[loggable(skip_params)]
    fn loop_instrumenter(log: Rc<RefCell<Vec<u8>>>) {
        // Previous loop:
        let mut while_count = 0;
        while while_count < 5 {
            if while_count == 1 || while_count == 3 {
                f(log.clone(), false); // Do not assert if caching stops.
            }
            while_count += 1;
        }
        #[rustfmt::skip]
        // Assert: Previous loop is fully logged and flushed.
        test_assert!(log, concat!(
            "loop_instrumenter(..) {\n",
            "  { // Loop body start.\n",
            "    f(..) {}\n",
            "  } // Loop body end.\n",
            "  // Loop body repeats 1 time(s).\n",
            // Iterations [0, 2, 4] are removed.
        ));

        log.borrow_mut().clear(); // Clear the log. The call graph is not affected.

        // The loop of interest:
        while_count = 0;
        while while_count < 6 {
            if while_count < 3 {
                // Assert: Loop body is being cached.
                test_assert!(log, "");
            } else if while_count == 3 {
                f(log.clone(), true); // Assert if caching stops.
            } else {
                f(log.clone(), false); // Increment the repeat count.
            }
            while_count += 1;
        }
        #[rustfmt::skip]
        // Assert: Second loop overall log is as expected.
        test_assert!(log, concat!(
            "  { // Loop body start.\n",
            "    f(..) {}\n",
            "  } // Loop body end.\n",
            "  // Loop body repeats 2 time(s).\n",
        ));
    }

    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    // Generate the log and check it step by step:
    loop_instrumenter(log.clone());
}

// B: `loopbody_after_call()`:
// call()
// while() {
//      Assert:
//          Loop body start is being cached (before and after those with child(ren)).
//          A few childless iterations are removed (before and after those with child(ren)).
//      [loop { // Optional nested loop.
//          Assert:
//              Loop body start is being cached (before and after those with child(ren)).
//              A few childless iterations are removed (before and after those with child(ren)).
//          // After the first few childless iterations:
//          f() {
//              Assert: Caching stopped.
//          }
//      }]
//      // After the first few childless iterations:
//      f() {
//          Assert: Caching stopped.
//      }
// }
#[test]
fn loopbody_after_call() {
    // Instrumented functions:
    #[loggable(skip_params)]
    fn f(log: Rc<RefCell<Vec<u8>>>, test_nested_loop: bool) {
        let expected = if test_nested_loop {
            concat!(
                "loop_instrumenter(..) {\n",
                // Iterations [0..=2] are removed.
                "  { // Loop body start.\n",
                "    { // Loop body start.\n",
                "      f(..) {", // Caching stopped.
            )
        } else {
            concat!(
                "loop_instrumenter(..) {\n",
                // Iterations [0..=2] are removed.
                "  { // Loop body start.\n",
                "    f(..) {", // Caching stopped.
            )
        };
        // Assert: Caching stopped.
        test_assert!(log, expected);
    }
    #[loggable(skip_params)]
    fn loop_instrumenter(log: Rc<RefCell<Vec<u8>>>, test_nested_loop: bool) {
        let mut while_count = 0;
        while while_count < 7 {
            match while_count {
                0..=2 => {
                    // Assert:
                    //  Loop body start is being cached (before those with child(ren)).
                    //  A few childless iterations are removed (before those with child(ren)).
                    test_assert!(
                        log,
                        concat!(
                            "loop_instrumenter(..) {",
                            // Iterations [0..=1] are removed.
                        )
                    )
                }
                3 => {
                    if !test_nested_loop {
                        f(log.clone(), test_nested_loop); // Stop caching.
                    } else {
                        let mut loop_count = 0;
                        loop {
                            match loop_count {
                                0..=2 => {
                                    // Assert:
                                    //  Loop body start is being cached (before those with child(ren)).
                                    //  A few childless iterations are removed (before those with child(ren)).
                                    test_assert!(
                                        log,
                                        concat!(
                                            "loop_instrumenter(..) {",
                                            // Nested loop's iterations [0..=1] are removed.
                                        )
                                    );
                                }
                                3 => {
                                    f(log.clone(), test_nested_loop); // Stop caching both loops.
                                }
                                4..=6 => {
                                    // Assert:
                                    //  Loop body start is being cached (after those with child(ren)).
                                    //  A few childless iterations are removed (after those with child(ren)).
                                    test_assert!(
                                        log,
                                        concat!(
                                            "loop_instrumenter(..) {\n",
                                            // Iterations [0..=2] are removed.
                                            "  { // Loop body start.\n",
                                            "    { // Loop body start.\n",
                                            "      f(..) {}\n", // Caching stopped.
                                            "    } // Loop body end.\n", // Nested loop's iteration [3] ended.
                                                                         //   Nested loop's iterations [4..=6] are being cached.
                                                                         //   Nested loop's iterations [4..=5] have been removed.
                                        )
                                    )
                                }
                                _ => (),
                            }
                            loop_count += 1;
                            if loop_count > 6 {
                                break;
                            }
                        }
                    }
                }
                4..=6 => {
                    let expected = if !test_nested_loop {
                        concat!(
                            "loop_instrumenter(..) {\n",
                            // Iterations [0..=2] are removed.
                            "  { // Loop body start.\n",
                            "    f(..) {}\n", // Caching stopped.
                            "  } // Loop body end.\n", // The iteration with child(ren).
                                              // The iterations [4..=6] are being cached.
                                              // The iterations [4..=5] have been removed.
                        )
                    } else {
                        concat!(
                            "loop_instrumenter(..) {\n",
                            // Iterations [0..=2] are removed.
                            "  { // Loop body start.\n",
                            "    { // Loop body start.\n",
                            "      f(..) {}\n",          // Caching stopped.
                            "    } // Loop body end.\n", // Nested loop's iteration [3] ended.
                            //   Nested loop's iterations [4..=6] have been removed.
                            "  } // Loop body end.\n", // Outer loop's iteration [3] ended.
                                                       // Outer loop's iterations [4..=6] are being cached.
                                                       // Outer loop's iterations [4..=5] have been removed.
                        )
                    };
                    // Assert:
                    //  Loop body start is being cached (after those with child(ren)).
                    //  A few childless iterations are removed (after those with child(ren)).
                    test_assert!(log, expected);
                }
                _ => (),
            }

            while_count += 1;
        }
    }

    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    // Generate the log and check it step by step:
    loop_instrumenter(log.clone(), false); // No nested loop.

    flush_log();
    log.borrow_mut().clear();

    loop_instrumenter(log.clone(), true); // Has nested loop.
    flush_log();
}

// A: `loopbody_after_loopbody()`:
// // `for` { // Loop start.
//   {} // [0]. Childless loop body. Gets removed.
//   {
//     f() {}
//   } // [1]. Loop body with a child. Is logged as is.
//   Assert:
//     Iteration [0] is removed.
//     Iteration [1] is logged as is.
//
//   {} // [2]. Childless loop body. Gets removed. Does not increment the repeat count.
//   {} // [3]. Childless loop body. Gets removed. Does not increment the repeat count.
//   Assert: Iterations [2] and [3] did not affect the log.
//
//   {
//     f() {}
//   } // [4]. Loop body with the same children as in [1]. Increments the repeat count and gets removed.
//   {} // [5]. Childless loop body. Gets removed. Does not increment the repeat count.
// // } // Loop end.
// Assert:
//   Iteration [4] is logged as a repeat count 1.
#[test]
fn loopbody_after_loopbody() {
    // Instrumented functions:
    #[loggable]
    fn f() {}
    // Function that instruments the `for` loop
    // since the loops (the expressions) cannot be annotated directly in Rust.
    #[loggable(skip_params)]
    fn loop_instrumenter(log: Rc<RefCell<Vec<u8>>>, different_iterations: bool) {
        let mut iter_count_sum = 0;
        let loop_result = // At the moment of writing the unit value `()`
        // is the only known possible value returnable by the `for` loop.
        for iter_count in 0..6 {
            iter_count_sum += iter_count; // Generate some testable behavior that must not be affected by the FCL instrumentation.

            if iter_count == 1 {
                f(); // Generate some call log.
            } else if iter_count == 2 {
                // Assert:
                // Iteration [0] is removed.
                // Iteration [1] is logged as is.
                test_assert!(
                    log,
                    concat!(
                        "loop_instrumenter(..) {\n",
                        // Iteration [0] is removed.
                        "  { // Loop body start.\n",
                        "    f() {}\n",
                        "  } // Loop body end.\n", // Iteration [1] is logged as is.
                    )
                )
            } else if iter_count == 4 {
                // Assert: Iterations [2] and [3] did not affect the log (they don't make instrumented calls).
                test_assert!(
                    log,
                    concat!(
                        "loop_instrumenter(..) {\n",
                        "  { // Loop body start.\n",
                        "    f() {}\n",
                        "  } // Loop body end.\n",
                        // Iterations [2] and [3] did not affect the log.
                    ),
                );

                f(); // Generate some call log.
                if different_iterations {
                    f(); // Generate the difference from [1].
                }
            }
            // Iteration [5] is childless, gets removed.
        };

        if different_iterations {
            // Assert: Iteration [4] is logged as is.
            test_assert!(
                log,
                concat!(
                    "loop_instrumenter(..) {\n",
                    "  { // Loop body start.\n",
                    "    f() {}\n",
                    "  } // Loop body end.\n",
                    "  { // Loop body start.\n", // Iteration [4] is logged as is.
                    "    f() {}\n",
                    "    // f() repeats 1 time(s).\n", // The difference from [1].
                    "  } // Loop body end.\n",
                ),
            );
        } else {
            // Assert: Iteration [4] is logged as a repeat count 1.
            test_assert!(
                log,
                concat!(
                    "loop_instrumenter(..) {\n",
                    "  { // Loop body start.\n",
                    "    f() {}\n",
                    "  } // Loop body end.\n",
                    "  // Loop body repeats 1 time(s).\n", // Iteration [4] is logged as a repeat count 1.
                ),
            );
        }

        // Assert: Behavior didn't change because of call logging.
        assert_eq!(iter_count_sum, 15);
        assert_eq!(loop_result, ());
    }

    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    // Generate the log and check it step by step:
    loop_instrumenter(log.clone(), false); // Identical iterations.

    flush_log();
    log.borrow_mut().clear();

    loop_instrumenter(log.clone(), true); // Different iterations.
    flush_log();
}

#[test]
fn adjacent_identical_loops() {
    // Instrumented functions:
    #[loggable]
    fn f() {}
    #[loggable]
    fn loop_instrumenter() {
        // Two adjacent loops with identical bodies:
        for _ in 0..2 {
            f()
        }
        for _ in 0..3 {
            f()
        }
        // They must not be shown as one loop.
    }

    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    // Generate the log:
    loop_instrumenter();

    test_assert!(
        log,
        concat!(
            "loop_instrumenter() {\n",
            "  { // Loop body start.\n",
            "    f() {}\n",
            "  } // Loop body end.\n",
            "  // Loop body repeats 1 time(s).\n",
            "  { // Loop body start.\n", // The second loop is not logged as repeating iterations of the previous loop.
            "    f() {}\n",
            "  } // Loop body end.\n",
            "  // Loop body repeats 2 time(s).\n",
            "} // loop_instrumenter().\n",
        ),
    );
}

// #[test]
// fn add_loopbody_start() {}
