use std::cell::RefCell;
use std::rc::Rc;
use fcl_proc_macros::loggable;
// use fcl_proc_macros::{loggable, non_loggable};
use crate::common::*;

// #[test]
// fn tmp0() {
//     #[loggable]
//     fn h() {
//         #[loggable]
//         fn i() {}
//         i();
//     }
    
//     // #[loggable]
//     // fn m() {
//     //     #[loggable(skip_closure_coords)]
//     //     fn n() {
//     //         Some(0).map(|y| y + 2);
//     //     }
//     //     n();
//     // }
//     // // fn m()
//     // // {
//     // //     use fcl :: { CallLogger, MaybePrint }; let ret_val = fcl :: call_log_infra
//     // //     :: instances ::
//     // //     THREAD_LOGGER.with(| logger |
//     // //     {
//     // //         let param_val_str = None; let mut body = move ||
//     // //         {
//     // //             #[loggable(prefix = m, log_params, skip_closure_coords)] fn n()
//     // //             { Some(0).map(| y | y + 2); } n();
//     // //         }; if ! logger.borrow().logging_is_on() { return body(); } let mut
//     // //         generic_func_name = String :: with_capacity(64);
//     // //         generic_func_name.push_str("m"); if ! true
//     // //         {
//     // //             generic_func_name.push_str("<"); let generic_arg_names_vec : Vec <
//     // //             & 'static str > = vec! []; for (idx, generic_arg_name) in
//     // //             generic_arg_names_vec.into_iter().enumerate()
//     // //             {
//     // //                 if idx != 0 { generic_func_name.push_str(","); }
//     // //                 generic_func_name.push_str(generic_arg_name);
//     // //             } generic_func_name.push_str(">");
//     // //         } let mut callee_logger = fcl :: CalleeLogger ::
//     // //         new(& generic_func_name, param_val_str); let ret_val = body(); if
//     // //         false
//     // //         {
//     // //             let ret_val_str = format! ("{}", ret_val.maybe_print());
//     // //             callee_logger.set_ret_val(ret_val_str);
//     // //         } ret_val
//     // //     }); ret_val
//     // // }
// }

#[test]
fn param_pass() {
    #[loggable]
    fn f() {
        fn g(b: bool) {
            Some(1).map(|x| x + 1);
        }
        g(true);
    }

    let log = substitute_log_writer!();
    f();

    // The following assert is unstable because the closure coordinates change upon file update.
    // 
    // #[rustfmt::skip]
    // test_assert!(log, concat!(
    //     "f() {\n",
    //     "  f::g(b: true) {\n",
    //     "    f::g::closure{62,25:62,33}(x: 1) {} -> 2\n",    // The coords {62,25:62,33} change.
    //     "  } // f::g().\n",
    //     "} // f().\n",
    // ));
    // 
    // The work-around follows.

    let log_contents = unsafe { String::from(std::str::from_utf8_unchecked(&*log.borrow())) };

    #[rustfmt::skip]
    let start = concat!(
            "f() {\n",
            "  f::g(b: true) {\n",
            "    f::g::closure{",
    );
    // Find the `start` at the beginning of the `log_contents`.
    let optional_index = log_contents.find(start);

    // Assert: The `start` is found,
    if let Some(index) = optional_index {
        // at the beginning of the `log_contents`.
        assert_eq!(0, index);

        // Assert: The end is found after the `start`.
        #[rustfmt::skip]
        assert!(log_contents[start.len()..].find(concat!(
                               /*62,25:62,33*/ "}(x: 1) {} -> 2\n",
            "  } // f::g().\n",
            "} // f().\n",
        )).is_some());
    } else {
        assert!(false, "Failed to find expected fragment");
    }

    #[loggable]
    fn h() {
        #[loggable]
        fn i() {}
        i();
    }
    log.borrow_mut().clear();
    h();
    #[rustfmt::skip]
    test_assert!(log, concat!(
        "h() {\n",
        "  h::i() {}\n",  // Still "h::".
        "} // h().\n",
    ));

    {
        #[loggable(skip_params)]
        fn j(_u: u8) {
            #[loggable] // (skip_params) is inherited.
            fn k(_i: i8) {}
            k(3);
        }
        log.borrow_mut().clear();
        j(4);
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(..) {\n",
            "  j::k(..) {}\n",  // Still no params.
            "} // j().\n",
        ));
    }
    {
        #[loggable]
        fn j(_u: u8) {
            #[loggable(skip_params)] // (skip_params) is user-provided.
            fn k(_i: i8) {}
            k(3);
        }
        flush_log();
        log.borrow_mut().clear();
        j(5);
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(_u: 5) {\n",     // Params.
            "  j::k(..) {}\n",  // No params.
            "} // j().\n",
        ));
    }

    {
        #[loggable(skip_closure_coords)]
        fn m() {
            #[loggable]
            fn n() {
                Some(0).map(|y| y + 2);
            }
            n();
        }
        log.borrow_mut().clear();
        m();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "m() {\n",
            "  m::n() {\n",
            "    m::n::closure{..}(y: 0) {} -> 2\n", // Still no coords
            "  } // m::n().\n",
            "} // m().\n",
        ));
    }

    flush_log();
    log.borrow_mut().clear();
    
    #[loggable]
    fn m() {
        #[loggable(skip_closure_coords)]
        fn n() {
            Some(0).map(|x| x + 3);
        }
        n();
    }
    m();
    #[rustfmt::skip]
    test_assert!(log, concat!(
        "m() {\n",
        "  m::n() {\n",
        "    m::n::closure{..}(x: 0) {} -> 3\n", // Still no coords
        "  } // m::n().\n",
        "} // m().\n",
    ));
}

