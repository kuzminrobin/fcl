use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::{loggable};

#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::{THREAD_DECORATOR};

use crate::common::*;

// f() {
//     { // Loop body start.
//         g();                // Has child(ren). 
//     }
//     // Repeats...           // Not logged yet.
//                             // Assert: No repeat count logged.
//     Loop end.
//                             // Assert: The repeat count is logged.
//     { // Loop body start.   // Identical loop. Must be logged separately from the previous one.
//         g();
//     }
//     // Repeats...           // Not logged yet.
//                             // Assert: The identical loop is being logged separately from the previous one.
//     Loop end.
//                             // Assert: The repeat count is logged.
// }
#[test]
fn loop_end() {
    #[loggable]
    fn g() {}

    fn assert_between_iters(loop_count: usize, next_iter_count: usize, log: Rc<RefCell<Vec<u8>>>) -> bool {
        // Iteration [0] is logged as is.
        // [1] increments the repeat count.
        match loop_count {
            0 => {  // In the first loop
                // Before iteration [2] (after [1]): 
                if next_iter_count == 2 {
                    #[rustfmt::skip]
                    test_assert!(log, concat!(
                        "loop_instrumenter(..) {\n",
                        "  { // Loop body start.\n",
                        "    g() {}\n",
                        "  } // Loop body end.\n",
                        // Assert: No repeat count logged.
                    ));
                }
            }
            1 => { // In the second loop
                // Before iteration [2] (after [1]): 
                if next_iter_count == 2 {
                        #[rustfmt::skip]
                        test_assert!(log, concat!(
                            "loop_instrumenter(..) {\n",
                            "  { // Loop body start.\n",
                            "    g() {}\n",
                            "  } // Loop body end.\n",
                            "  // Loop body repeats 1 time(s).\n",
                            "  { // Loop body start.\n",
                            "    g() {}\n",
                            "  } // Loop body end.\n",  // Assert: The identical loop is being logged separately from the previous one.
                        ));
                }
            }
            _ => ()
        }

        next_iter_count < 2     // Do iterations [0, 1].
    }

    #[loggable(skip_params)]
    fn loop_instrumenter(log: Rc<RefCell<Vec<u8>>>) {
        let mut iter_count = 0;
        while assert_between_iters(0, iter_count, log.clone()) {
            g();
            iter_count += 1;
        }
        
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "loop_instrumenter(..) {\n",
            "  { // Loop body start.\n",
            "    g() {}\n",
            "  } // Loop body end.\n",
            "  // Loop body repeats 1 time(s).\n", // Assert: The repeat count is logged.
        ));

        iter_count = 0;
        while assert_between_iters(1, iter_count, log.clone()) {
            g();
            iter_count += 1;
        }
        
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "loop_instrumenter(..) {\n",
            "  { // Loop body start.\n",
            "    g() {}\n",
            "  } // Loop body end.\n",
            "  // Loop body repeats 1 time(s).\n",
            "  { // Loop body start.\n",
            "    g() {}\n",
            "  } // Loop body end.\n",
            "  // Loop body repeats 1 time(s).\n",    // Assert: The repeat count is logged.
        ));
   }

    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    loop_instrumenter(log.clone());
    
}