use std::cell::RefCell;
use std::rc::Rc;
use fcl_proc_macros::loggable;
use crate::common::*;


//
// #[loggable]         | TestCases
// Attribute           | (outer (enclosing) function, inner (local) function)
// Values              | G: Written by ChatGPT/Codex.                               |   Notes
//---------------------|------------------------------------------------------------|-------------
// Absent              |GG|? |? |? |   | ?|  |  |  |   | ?|  |  |  |    | ?|  |  |  |   // ``
// NoArgs              |  | ?|  |  |   |? |??|? |? |   |  | ?|  |  |    |  | ?|  |  |   // `#[loggable]`
// skip_closure_coords |  |  | ?|  |   |  |  | ?|  |   |? |? |??|? |    |  |  | ?|  |   // `#[loggable(skip_closure_coords)]`
// log_closure_coords  |  |  |  | ?|   |  |  |  | ?|   |  |  |  | ?|    |? |? |? |??|   // `#[loggable(log_closure_coords)]`

#[test]
fn closure_coords() {
    let log = substitute_log_writer!();

    // Absent/Absent
    {
        // Attribute is absent. Nothing to log.
        fn f() {
            // Attribute is absent. Nothing to log.
            fn g() {
                Some(0).map(|x| x);
            }
            g();
        }
        f();

        flush_log();
        test_assert!(log, concat!("",)); // Assert: No log.
        log.borrow_mut().clear();
    }

    // Absent/NoArgs
    {
        // Attribute is absent. Nothing to log.
        fn f() {
            #[loggable] // Log closure coords.
            fn g() {
                Some(0).map(|x| x);
            }
            g();
        }
        f();

        // Unstable assert:
        //
        // #[rustfmt::skip]
        // test_assert!(log, concat!(
        //     "g() {\n",
        //     "  g::closure{43,29:43,33}(x: 0) {} -> 0\n",     // The closure coordinates `43,29:43,33` are file-change-intolerant.
        //     "} // g().\n",
        // ));
        //
        // Work-around:
        #[rustfmt::skip]
        assert_except_closure_coords!(
            log, 
            concat!(
                "g() {\n",
                "  g::closure{",
            ),
            concat!(
                             "}(x: 0) {} -> 0\n",
                "} // g().\n",
            )
        );
    }

    // // Absent/skip_closure_coords
    // {
    //     // Attribute is absent. Nothing to log.
    //     fn f() {
    //         #[loggable(skip_closure_coords)]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "g() {\n",
    //         "  g::closure{..}(x: 0) {} -> 0\n",
    //         "} // g().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // Absent/log_closure_coords
    // {
    //     // Attribute is absent. Nothing to log.
    //     fn f() {
    //         #[loggable(log_closure_coords)]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "g() {\n",
    //         "  g::closure{87,29:87,33}(x: 0) {} -> 0\n",
    //         "} // g().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // NoArgs/Absent
    // {
    //     #[loggable]
    //     fn f() {
    //         // Attribute is absent; should inherit log_closure_coords.
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{109,29:109,33}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // NoArgs/NoArgs
    // {
    //     #[loggable]
    //     fn f() {
    //         #[loggable]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{133,29:133,33}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // NoArgs/skip_closure_coords
    // {
    //     #[loggable]
    //     fn f() {
    //         #[loggable(skip_closure_coords)]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{..}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // NoArgs/log_closure_coords
    // {
    //     #[loggable]
    //     fn f() {
    //         #[loggable(log_closure_coords)]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{181,29:181,33}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // skip_closure_coords/Absent
    // {
    //     #[loggable(skip_closure_coords)]
    //     fn f() {
    //         // Attribute is absent; should inherit skip_closure_coords.
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{..}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // skip_closure_coords/NoArgs
    // {
    //     #[loggable(skip_closure_coords)]
    //     fn f() {
    //         #[loggable]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{..}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // skip_closure_coords/skip_closure_coords
    // {
    //     #[loggable(skip_closure_coords)]
    //     fn f() {
    //         #[loggable(skip_closure_coords)]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{..}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // skip_closure_coords/log_closure_coords
    // {
    //     #[loggable(skip_closure_coords)]
    //     fn f() {
    //         #[loggable(log_closure_coords)]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{277,29:277,33}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // log_closure_coords/Absent
    // {
    //     #[loggable(log_closure_coords)]
    //     fn f() {
    //         // Attribute is absent; should inherit log_closure_coords.
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{301,29:301,33}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // log_closure_coords/NoArgs
    // {
    //     #[loggable(log_closure_coords)]
    //     fn f() {
    //         #[loggable]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{325,29:325,33}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // log_closure_coords/skip_closure_coords
    // {
    //     #[loggable(log_closure_coords)]
    //     fn f() {
    //         #[loggable(skip_closure_coords)]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{..}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }

    // // log_closure_coords/log_closure_coords
    // {
    //     #[loggable(log_closure_coords)]
    //     fn f() {
    //         #[loggable(log_closure_coords)]
    //         fn g() {
    //             Some(0).map(|x| x);
    //         }
    //         g();
    //     }
    //     f();

    //     flush_log();
    //     #[rustfmt::skip]
    //     test_assert!(log, concat!(
    //         "f() {\n",
    //         "  f::g() {\n",
    //         "    f::g::closure{373,29:373,33}(x: 0) {} -> 0\n",
    //         "  } // f::g().\n",
    //         "} // f().\n",
    //     ));
    //     log.borrow_mut().clear();
    // }
}
