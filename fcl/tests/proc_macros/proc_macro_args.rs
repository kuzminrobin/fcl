use std::cell::RefCell;
use std::rc::Rc;
use fcl_proc_macros::loggable;
// use fcl_proc_macros::{loggable, non_loggable};
use crate::common::*;

mod call_params;
mod closure_coords;

#[test]
fn closure_coords() {
    let log = substitute_log_writer!();

    // skip_closure_coords/{No arg}
    {
        #[loggable(skip_closure_coords)]
        fn m() {
            #[loggable] // `skip_closure_coords` is inherited.
            fn n() {
                Some(0).map(|y| y + 2);
            }
            n();
        }
        m();
        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "m() {\n",
            "  m::n() {\n",
            "    m::n::closure{..}(y: 0) {} -> 2\n", // Still no coords
            "  } // m::n().\n",
            "} // m().\n",
        ));
        log.borrow_mut().clear();
    }

    // {No arg}/skip_closure_coords
    #[loggable]
    fn m() {
        #[loggable(skip_closure_coords)]
        fn n() {
            Some(0).map(|x| x + 3);
        }
        n();
    }
    m();
    flush_log();
    #[rustfmt::skip]
    test_assert!(log, concat!(
        "m() {\n",
        "  m::n() {\n",
        "    m::n::closure{..}(x: 0) {} -> 3\n", // Still no coords
        "  } // m::n().\n",
        "} // m().\n",
    ));
    log.borrow_mut().clear();
}