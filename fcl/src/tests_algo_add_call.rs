use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::loggable;

use crate as fcl;
use crate::call_log_infra::instances::THREAD_DECORATOR; //, THREAD_LOGGER};

// High-level logic to test:
//     [{call | loop_body}  // previous sibling
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
//     // Test the log (`assert_eq!()`): No repeat count in the log.
//     call_of_interest() {}
//     // Test the log (`assert_eq!()`): The whole log, including the previous sibling's repeat count.

//     B: `caching_and_flush`:
//     // Input:
//     call() {} // Previous sibling with the same name.
//     // Test the log (`assert_eq!()`): Only the log above.
//     call() {} // Call of interest. Start caching.
//     // Test the log (`assert_eq!()`): No repeat count in the log.
//     call() {} // Call of interest. Continue caching.
//     // Test the log (`assert_eq!()`): No repeat count in the log.
//     // flush the log
//     // Ouput:
//     call() {} // (`call` case: No previous sibling, log the call)
//     // Repeats 2 time(s).  // Cache flush.

//     C: `flush_loopbodys_repeat_count`:
//     { // Loop body
//         some_calls() {}
//     }
//     // Test the log (`assert_eq!()`): Only the log above.
//     [Repeats]
//     // Test the log (`assert_eq!()`): No repeat count in the log.
//     // ! caching_is_active
//     call() {}
//     // Test the log (`assert_eq!()`): The whole log, including the loop_body's repeat count.

//     D: `flush_initial_loopbody`:
//     { // Enclosing loop body. Caching is active upon initial loop body.
//         // Test the log (`assert_eq!()`): Nothing in the log.
//         { // Intermediate enclosing loop body. Caching is still active.
//             // Test the log (`assert_eq!()`): Nothing in the log.
//             call() {
//                 // Test the log (`assert_eq!()`): The log above, flushed.
//             }
//         }
//     }
//     // Test the log (`assert_eq!()`): The whole log above.
mod singlethreaded {
    use super::*;

    #[test]
    fn differs_from_prev_sibling() {
        // A: `differs_from_prev_sibling`:
        // previous_sibling_with_diff_name() {}
        // // Test the log (`assert_eq!()`): Only the log above.
        // [// Repeats]
        // // Test the log (`assert_eq!()`): No repeat count in the log.
        // call_of_interest() {}
        // // Test the log (`assert_eq!()`): The whole log, including the previous sibling's repeat count.
        #[loggable]
        fn previous_sibling_with_diff_name() {}
        #[loggable]
        fn call_of_interest() {}

        // Mock log writer creation and substitution of the default one:
        let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
        THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

        previous_sibling_with_diff_name();
        unsafe {
            assert_eq!(
                std::str::from_utf8_unchecked(&*log.borrow()),
                concat!("previous_sibling_with_diff_name() {}\n",)
            )
        };

        let len_before = log.borrow().len(); // TODO: Remove after the investigation below. 
        previous_sibling_with_diff_name();
        assert_eq!(len_before, log.borrow().len()); // TODO: Remove after the investigation below. 

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
        // causes an unexpected stack buffer overrun (instead of an ordinary test failure report)
        // which is likely not the same as the stack overflow,
        // that doesn't happen
        // if the second call to `previous_sibling_with_diff_name()` above is commented out.
        // 
        // Makes a suspicion that the second call to `previous_sibling_with_diff_name()` above
        // affects the subsequent `assert_eq!()`. After which
        // the successful `assert_eq!()` is still OK, but the unsuccessful one
        // starts generating the failure report, for that it
        // starts iterating through the `log` and overflows some buffer on the stack.
        // 
        // The second call to `previous_sibling_with_diff_name()` above does not affect the `log`.
        // At least the log length stays the same before and after 
        // the call to `previous_sibling_with_diff_name()` above. 
        // Thus, another suspicion is a bug in `assert_eq!()` or `concat!()` or `std::str::from_utf8_unchecked()`.
        // 
        // Below is the work-around of the `assert_eq!()` above. It handles the `assert_eq!()` failure 
        // in an expected way.
        unsafe { 
            let result_log  = String::from(std::str::from_utf8_unchecked(&*log.borrow()));
            assert_eq!(
                result_log, 
                concat!(
                    "previous_sibling_with_diff_name() {}\n", // Assert: No repeat count in the log.
                )
            )
        };

        call_of_interest();
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
}
