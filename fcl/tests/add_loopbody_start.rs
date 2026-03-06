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

//   High-level logic to test:
//     [parent // Previous parent to activate caching.]
//     parent {  // It's a call or an enclosing loop body. Caching is {inactive, active}.
//       Either a previous loop [with [non-]identical children];
//       or a call: {function, closure};
// A:    or a previous iteration of the current loop [A: with [non-]identical children]
//
// A:    { // Loop body start that's being tested.
//
// Test cases:
//
// A: `basics`:
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
fn basics() {
    // Instrumented functions:
    #[loggable]
    fn f() {}
    // Function that instruments the `for` loop
    // since the loops (the expressions) cannot be annotated directly in Rust.
    #[loggable]
    fn loop_instrumenter(log: Rc<RefCell<Vec<u8>>>, different_iterations: bool) {

        // Remove "loop_instrumenter(log: RefCell { value: [...] }, different_iterations: ...) {\n" 
        // from the log because this log fragment is different 
        // for multiple calls of `loop_instrumenter()`.
        // TODO: Suppress param printing instead of log clearing here.
        log.borrow_mut().clear();

        let mut iter_count_sum = 0;
        let loop_result = // At the moment of writing the unit value `()`
        // is the only known possible value returnable by the `for` loop.
        for iter_count in 0..6 {
            iter_count_sum += iter_count; // Generate some testable state.

            if iter_count == 1 {
                f(); // Generate some call log.
            } else if iter_count == 2 {
                // Assert:
                // Iteration [0] is removed.
                // Iteration [1] is logged as is.
                test_assert!(
                    log,
                    concat!(
                        "\n", // Stayed in decorator after "loop_instrumenter(...) {" went to the call log.
                        // Iteration [0] is removed.
                        "  { // Loop body start.\n",
                        "    f() {}\n",
                        "  } // Loop body end.\n", // Iteration [1] is logged as is.
                    )
                )
            } else if iter_count == 4 {
                // Assert: Iterations [2] and [3] did not affect the log.
                test_assert!(
                    log,
                    concat!(
                        "\n",
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
        };

        if different_iterations {
            // Assert: Iteration [4] is logged as is.
            test_assert!(
                log,
                concat!(
                    "\n",
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
                    "\n",
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
        // Two adjacent identical loops:
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
            "  { // Loop body start.\n",
            "    f() {}\n",
            "  } // Loop body end.\n",
            "  // Loop body repeats 2 time(s).\n",
            "} // loop_instrumenter().\n",
        ),
    );

}

// #[test]
// fn add_loopbody_start() {}
