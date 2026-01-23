use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::loggable;

#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::{THREAD_DECORATOR, THREAD_LOGGER};

// High-level logic to test:
//     [{call | loop_body}  // Previous sibling.
//         // `call` case: No previous sibling, log the call.
//         [Repeats]
//     ]
//     if ! caching_is_active {
//         if call_of_interest_with_diff_name {
//             // Log the prev sibling's repeat count.
//             // Log the call.
//         } else { // call_of_interest_with_same_name
//             // Begin caching.
//         }
//     } else { // caching_is_active
//         // If started in the enclosing loop body then
//         //     flush and stop caching.
//         // (else continue caching).
//     }
// Test cases:
//     A: `differs_from_prev_sibling`:
//     previous_sibling_with_diff_name() {}
//     // Test the log (`assert_eq!()`): Only the log above.
//     [// Repeats]
//     // Test the log: No repeat count in the log.
//     call_of_interest() {}
//     // Test the log: The whole log, including the previous sibling's repeat count.

//     B: `caching_and_flush`:
//     // Input (actual calls):
//     call() {} // Previous sibling with the same name.
//     // Test the log: Only the log above.
//     call() {} // Call of interest. Start caching.
//     // Test the log: No repeat count in the log (cached).
//     call() {} // Call of interest. Continue caching.
//     // Test the log: No repeat count in the log (cached).
//     // Flush the log.
//     // Ouput (function call log):
//     call() {} // (`call` case: No previous sibling, log the call)
//     // Repeats 2 time(s).  // Cache flush.

//     C: `flush_loopbodys_repeat_count`:
//     { // Loop body
//         some_calls() {}
//     }
//     // Test the log: Only the log above.
//     [Repeats]
//     // Test the log: No repeat count in the log.
//     // ! caching_is_active
//     call() {}
//     // Test the log: The whole log, including the loop_body's repeat count.

//     D: `flush_initial_loopbody`:
//     { // Enclosing loop body. Start caching upon initial loop body.
//         // Test the log: Nothing in the log.
//         { // Intermediate enclosing loop body. Continue caching.
//             // Test the log: Nothing in the log.
//             call() {
//                 // Test the log: The log above, flushed.
//             }
//         }
//     }
//     // Test the log: The whole log above.

// TODO: Consider `mod single_thread` -> `mod add_call::single_thread` for the `cargo test` to add `add_call::` to the log:
// ```
// running 4 tests
// test add_call::single_thread::caching_and_flush ... ok
// test add_call::single_thread::differs_from_prev_sibling ... ok
// ```
mod single_thread {
    use super::*;

    #[test]
    fn differs_from_prev_sibling() {
        // A: `differs_from_prev_sibling`:
        // previous_sibling_with_diff_name() {}
        // // Test the log: Only the log above.
        // [// Repeats]
        // // Test the log: No repeat count in the log.
        // call_of_interest() {}
        // // Test the log: The whole log, including the previous sibling's repeat count.
        #[loggable]
        fn previous_sibling_with_diff_name() {}
        #[loggable]
        fn call_of_interest() {}

        // Mock log writer creation and substitution of the default one:
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        // Generate the log and check it step by step:
        
        previous_sibling_with_diff_name();
        // Test the log: Only the log above.
        unsafe {
            assert_eq!(
                std::str::from_utf8_unchecked(&*log.borrow()),
                concat!("previous_sibling_with_diff_name() {}\n",)
            )
        };

        previous_sibling_with_diff_name(); // Repeats.
        // NOTE: In the `assert_eq!()` below
        // unsafe {
        //     assert_eq!(
        //         std::str::from_utf8_unchecked(&*log.borrow()),
        //         concat!(
        //             "previous_sibling_with_diff_name() {}\n", // Assert: No repeat count in the log.
        //         )
        //     )
        // }
        // adding a space in the beginning of the string literal (`" prev...`)
        // causes, instead of an ordinary test failure report, an unexpected stack buffer overrun
        // (which is very likely not the same as the stack overflow),
        // that doesn't happen
        // if the second call to `previous_sibling_with_diff_name()` above is commented out.
        //
        // Causes a suspicion that the second call to `previous_sibling_with_diff_name()` above
        // affects the subsequent `assert_eq!()`. After which
        // the successful `assert_eq!()` is still OK, but the unsuccessful one
        // starts generating the failure report. For that
        // it starts iterating through the `log` and overflows some buffer on the stack.
        //
        // The second call to `previous_sibling_with_diff_name()` above does not affect the `log` though.
        // At least the `log` length stays the same before and after
        // the second call to `previous_sibling_with_diff_name()` above.
        // Thus, another suspicion is a bug in `assert_eq!()` or `concat!()` or `std::str::from_utf8_unchecked()`.
        //
        // Below (after the comment) is the work-around of the `assert_eq!()` above - the `log` value is copied
        // before the `assert_eq!()`. The work-around handles the `assert_eq!()` failure
        // in the expected way.
        //
        // UPDATE: In the other test the `log` contents have to be copied outside of `assert_eq!()`.
        // Otherwise in case of a test failure the panic occurs "while processing panic" and the
        // test executor aborts reporting the exit code: 0xc0000409, STATUS_STACK_BUFFER_OVERRUN (same as in this test).
        // Likely because upon the comparison failure during `assert_eq!()`, the `assert_eq!()` panics
        // while `log.borrow()` is still active, the `log` being borrowed for reading. 
        // And the FCL's panic hook tries to borrow the `log` again, but for writing, 
        // fails, and likely eventually panics again.
        // This is likely the reason for this test too, 
        // * but why the second call to `previous_sibling_with_diff_name()` above
        //   affects the behavior of `assert_eq!()` below, 
        // * and why the first call to `previous_sibling_with_diff_name()` above
        //   does not affect the subsequent `assert_eq!()` above
        // is still unclear.
        // Leaving the comment and the UPDATE above for the analysis of the future possible test malfunction.
        unsafe {
            let result_log = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
            // Test the log: No repeat count in the log.
            assert_eq!(
                result_log,
                concat!(
                    "previous_sibling_with_diff_name() {}\n",
                )
            )
        };

        call_of_interest();
        // Test the log: The whole log, including the previous sibling's repeat count.
        unsafe {
            assert_eq!(
                std::str::from_utf8_unchecked(&*log.borrow()),
                concat!(
                    "previous_sibling_with_diff_name() {}\n",
                    "// previous_sibling_with_diff_name() repeats 1 time(s).\n",
                    "call_of_interest() {}\n",
                )
            )
        };
    }

    #[test]
    fn caching_and_flush() {
        // B: `caching_and_flush`:
        //
        // // Input:
        // call() {} // Previous sibling with the same name.
        // // Test the log: Only the log above.
        // call() {} // Call of interest. Start caching.
        // // Test the log: No repeat count in the log.
        // call() {} // Call of interest. Continue caching.
        // // Test the log: No repeat count in the log.
        // // flush the log
        //
        // // Ouput:
        // call() {} // (`call` case: No previous sibling, log the call)
        // // Repeats 2 time(s).  // Cache flush.

        #[loggable]
        fn call() {}

        // Mock log writer creation and substitution of the default one:
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        call();
        unsafe {
            assert_eq!(
                std::str::from_utf8_unchecked(&*log.borrow()),
                concat!("call() {}\n",) // Only the log above.
            )
        };
        call();
        unsafe {
            assert_eq!(
                std::str::from_utf8_unchecked(&*log.borrow()),
                concat!("call() {}\n",) // Caching starts. No repeat count in the log.
            )
        };
        call();
        unsafe {
            assert_eq!(
                std::str::from_utf8_unchecked(&*log.borrow()),
                concat!("call() {}\n",) // Caching continues. No repeat count in the log.
            )
        };

        // Flush the log:
        THREAD_LOGGER.with(|logger| {
            #[cfg(feature = "singlethreaded")]
            let logger = logger.borrow_mut();

            logger.borrow_mut().flush();
        });
        #[rustfmt::skip]
        unsafe {
            assert_eq!(
                std::str::from_utf8_unchecked(&*log.borrow()),
                concat!(
                    "call() {}\n", // No previous sibling, log the call.
                    "// call() repeats 2 time(s).\n" // Cache flush.
                ) 
            )
        };
    }

    #[test]
    fn flush_loopbodys_repeat_count() {
        // C: `flush_loopbodys_repeat_count`:
        // { // Loop body
        //     some_calls() {}
        // }
        // // Test the log: Only the log above.
        // [Repeats]
        // // Test the log: No repeat count in the log.
        // // ! caching_is_active
        // call() {}
        // // Test the log: The whole log, including the loop_body's repeat count.

        // The instrumented functions that will generate the call log.
        #[loggable]
        fn some_call() {}
        #[loggable]
        fn call() {}
        // The loop body cannot be attributed with `#[loggable]` directly 
        // (Rust: the proc macros cannot be applied to expressions). 
        // The loop body is placed into a `#[loggable]` function
        // that recursively instruments the loop body.
        #[loggable]
        fn instrumented_loopbody_container(log: Rc<RefCell<Vec<u8>>>) {
            // Generate some call log and make sure it is correct:

            for _ in 0..3 {
                // The instrumented loop body.
                some_call();
            }

            // Test the log: Make sure that there's no repeat count in the log.
            #[rustfmt::skip]
            unsafe {
                // (Inside of the instrumented function?) the `log` contents have to be copied outside of `assert_eq!()`.
                // Otherwise in case of a test failure the panic occurs "while processing panic" and the 
                // test executor aborts reporting the exit code: 0xc0000409, STATUS_STACK_BUFFER_OVERRUN.
                // Likely because upon the comparison failure during `assert_eq!()`, the `assert_eq!()` panics
                // while `log.borrow()` is still active, being borrowed for reading. 
                // And the FCL's panic hook tries to borrow the log again,
                // but for writing, fails, and likely eventually panics again.
                let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                assert_eq!(
                    log_contents,
                    concat!(
                        "instrumented_loopbody_container(log: RefCell { value: [] }) {\n",
                        "  { // Loop body start.\n",
                        "    some_call() {}\n",
                        "  } // Loop body end.\n",
                        // Make sure that there's no repeat count in the log.
                    ) 
                )
            };

            call();
            #[rustfmt::skip]
            unsafe {
                let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                assert_eq!(
                    log_contents,
                    concat!(
                        "instrumented_loopbody_container(log: RefCell { value: [] }) {\n",
                        "  { // Loop body start.\n",
                        "    some_call() {}\n",
                        "  } // Loop body end.\n",
                        "  // Loop body repeats 2 time(s).\n", // The repeat count is flushed (since `! caching_is_active`).
                        "  call() {}\n"
                    ) 
                )
            };
        }

        // Mock log writer creation and substitution of the default one:
        let log: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        // The call log generation and the checks:
        instrumented_loopbody_container(log.clone());
    }

    #[test]
    fn flush_initial_loopbody() {
        // D: `flush_initial_loopbody`:
        // { // Enclosing loop body. Caching is active upon initial loop body.
        //     // Test the log: Nothing's in the log.
        //     { // Intermediate enclosing loop body. Caching is still active.
        //         // Test the log: Nothing's in the log.
        //         call() {
        //             // Test the log: The log above, flushed.
        //         }
        //     }
        // }
        // // Test the log: The whole log above.

        // The loop body cannot be instrumented directly (see details in the other test). 
        // It is placed into an instrumented function
        // that recursively instruments the loop body.
        #[loggable]
        fn instrumented_loopbody_container(log: Rc<RefCell<Vec<u8>>>) {
            for index0 in 0..2 { // Enclosing loop body. Caching starts upon initial loop body.
                // Test the log: Nothing's in the log:
                if index0 == 0 {
                    #[rustfmt::skip]
                    unsafe {
                        let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                        assert_eq!(
                            log_contents,
                            concat!(
                                "instrumented_loopbody_container(log: RefCell { value: [] }) {",
                                // The initial loop body is cached.
                            ) 
                        )
                    };
                }

                for index1 in 0_usize..3_usize { // Intermediate enclosing loop body. Caching is still active.
                    // Test the log: Nothing is in the log:
                    if index0 == 0 && index1 == 0 {
                        #[rustfmt::skip]
                        unsafe {
                            let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                            assert_eq!(
                                log_contents,
                                concat!(
                                    "instrumented_loopbody_container(log: RefCell { value: [] }) {",
                                    // The initial loop body is cached.
                                    // The initial intermediate loop body is cached.
                                ) 
                            )
                        };
                    }

                    // Make a call that stops caching:
                    call(log.clone(), (index0, index1));
                }
            }
        }

        #[loggable]
        fn call(log: Rc<RefCell<Vec<u8>>>, indices: (usize, usize)) {
            // Test the log: The log up to this moment, flushed.
            if indices.0 == 0 && indices.1 == 0 {
                #[rustfmt::skip]
                unsafe {
                    let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
                    assert_eq!(
                        log_contents,
                        concat!(
                            "instrumented_loopbody_container(log: RefCell { value: [] }) {\n",
                            "  { // Loop body start.\n",    // Caching has stopped. Flushed.
                            "    { // Loop body start.\n",  // Caching has stopped. Flushed.
                            // The call that has put an end to caching (disregard the `call()` params below):
                            // TODO: After implementing the param logging suppression
                            // suppress the params logging in `fn call()` above
                            // and remove the params from the string literals below.
                            "      call(log: RefCell { value: [105, 110, 115, 116, 114, 117, 109, 101, 110, 116, 101, 100, 95, ",
                                                              "108, 111, 111, 112, 98, 111, 100, 121, 95, 99, 111, 110, 116, 97, ",
                                                              "105, 110, 101, 114, 40, 108, 111, 103, 58, 32, 82, 101, 102, 67, ",
                                                              "101, 108, 108, 32, 123, 32, 118, 97, 108, 117, 101, 58, 32, 91, ",
                                                              "93, 32, 125, 41, 32, 123] }, ",
                                        "indices: (0, 0)) {"
                        )
                    )
                };
            }
        }

        // Mock log writer creation and substitution of the default one:
        let log: Rc<RefCell<Vec<u8>>> = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        // Generate the call log and check it at certain steps:
        instrumented_loopbody_container(log.clone());

        // Test the log: The whole call log above.
        #[rustfmt::skip]
        unsafe {
            let log_contents  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
            assert_eq!(
                log_contents,
                concat!(
                    "instrumented_loopbody_container(log: RefCell { value: [] }) {\n",
                    "  { // Loop body start.\n",
                    "    { // Loop body start.\n",
                    // Disregard the `call()` params below.
                    // TODO: After implementing the param logging suppression
                    // suppress the params logging in `fn call()` above
                    // remove the `call()` params from the string literals below.
                    "      call(log: RefCell { value: [105, 110, 115, 116, 114, 117, 109, 101, 110, 116, 101, 100, 95, ",
                                                      "108, 111, 111, 112, 98, 111, 100, 121, 95, 99, 111, 110, 116, 97, ",
                                                      "105, 110, 101, 114, 40, 108, 111, 103, 58, 32, 82, 101, 102, 67, ",
                                                      "101, 108, 108, 32, 123, 32, 118, 97, 108, 117, 101, 58, 32, 91, ",
                                                      "93, 32, 125, 41, 32, 123] }, ",
                                "indices: (0, 0)) {}\n",
                    "    } // Loop body end.\n",
                    "    // Loop body repeats 2 time(s).\n",
                    "  } // Loop body end.\n",
                    "  // Loop body repeats 1 time(s).\n",
                    "} // instrumented_loopbody_container().\n"
                ) 
            )
        };
    }
}
