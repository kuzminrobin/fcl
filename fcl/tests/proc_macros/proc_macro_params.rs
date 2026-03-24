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

//
// #[loggable] | TestCases
// Attribute   | (outer (enclosing) function, inner (local) function)
// Values      | G: Written by ChatGPT/Codex.                               |   Notes
//-------------|------------------------------------------------------------+-------------
// Absent      |++|+ |+ |+ |   | +|  |  |  |   | G|  |  |  |    | G|  |  |  |   // ``
// NoArgs      |  | +|  |  |   |+ |++|+ |+ |   |  | +|  |  |    |  | G|  |  |   // `#[loggable]`
// skip_params |  |  | +|  |   |  |  | +|  |   |G |+ |GG|+ |    |  |  | G|  |   // `#[loggable(skip_params)]`
// log_params  |  |  |  | +|   |  |  |  | +|   |  |  |  | +|    |G |G |G |GG|   // `#[loggable(log_params)]`

#[test]
fn param_pass() {
    let log = substitute_log_writer!();

    // Absent/Absent
    {
        // Attribute is absent. Nothing to log.
        fn f() {
            // Attribute is absent. Nothing to log.
            fn g(_b: bool) {}
            g(true);
        }
        f();

        flush_log();
        test_assert!(log, concat!("",)); // Assert: No log.
        log.borrow_mut().clear();
    }
    // Absent/{No args}
    {
        // Attribute is absent. Nothing to log.
        fn f() {
            #[loggable] // {No args}. 
            fn g(_b: bool) {} // The params must be logged.
            g(true);
        }
        f();

        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
                                // Assert: `f()` is not logged.
            "g(_b: true) {}\n", // Assert: The params are logged.
        ));
        log.borrow_mut().clear();
    }

    // Absent/skip_params
    {
        // Attribute is absent.
        fn f(_a: u8) {
            #[loggable(skip_params)] // Must not log params.
            fn g(_b: bool) {}
            g(true);
        }
        f(2);

        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
                            // Assert: `f()` is not logged.
            "g(..) {}\n",   // Assert: Params are skipped.
        ));
        log.borrow_mut().clear();
    }

    // Absent/log_params
    {
        // Attribute is absent.
        fn f(_a: u8) {
            #[loggable(log_params)] // Must log params.
            fn g(_b: bool) {}
            g(true);
        }
        f(2);

        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
                                // Assert: `f()` is not logged.
            "g(_b: true) {}\n", // Assert: Params are logged.
        ));
        log.borrow_mut().clear();
    }

    // {No args}/Absent
    {
        #[loggable] // Must log params.
        fn f(_fp: u8) {
            // Attribute is absent.
            fn g(_b: bool) {} // Must log prefix `f::`, params.
            g(true);
        }
        f(1);

        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "f(_fp: 1) {\n",            // Assert: Params.
            "  f::g(_b: true) {}\n",    // Assert: Prefix, params.
            "} // f().\n",
        ));
        //
        log.borrow_mut().clear();
    }

    // {No args}/{No args}
    {
        #[loggable] // {No args}. Must log params.
        fn h(_hp: u16) {
            #[loggable] // {No args}. Must log prefix `h::`, params.
            fn i(_ip: u8) {}
            i(1);
        }
        h(9);

        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "h(_hp: 9) {\n",        // Assert: Params.
            "  h::i(_ip: 1) {}\n",  // Assert: Prefix `h::`, params.
            "} // h().\n",
        ));
        log.borrow_mut().clear();
    }

    // {No args}/skip_params
    {
        #[loggable] // {No args}. Must log params.
        fn j(_u: u8) {
            #[loggable(skip_params)] // Must log prefix, must not log params.
            fn k(_i: i8) {}
            k(3);
        }
        j(5);

        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(_u: 5) {\n",     // Params.
            "  j::k(..) {}\n",  // No params.
            "} // j().\n",
        ));
        log.borrow_mut().clear();
    }

    // {No args}/log_params
    {
        #[loggable] // {No args}. Must log params.
        fn j(_u: u8) {
            #[loggable(log_params)] // Must log prefix, params.
            fn k(_i: i8) {}
            k(3);
        }
        j(4);

        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(_u: 4) {\n",         // Params.
            "  j::k(_i: 3) {}\n",   // Prefix, params.
            "} // j().\n",
        ));
        log.borrow_mut().clear();
    }

    // skip_params/Absent
    {
        #[loggable(skip_params)] // Must skip params.
        fn j(_u: u8) {
            // Attribute is absent; must log prefix, skip params.
            fn k(_i: i8) {}
            k(3);
        }
        j(4);

        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(..) {\n",        // No params.
            "  j::k(..) {}\n",  // Prefix, no params (inherited).
            "} // j().\n",
        ));
        log.borrow_mut().clear();
    }

    // skip_params/{No args}
    {
        #[loggable(skip_params)] // Must skip params.
        fn j(_u: u8) {
            #[loggable] // Must log prefix, skip params.
            fn k(_i: i8) {}
            k(3);
        }
        j(4);
        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(..) {\n",        // No params.
            "  j::k(..) {}\n",  // Prefix, no params.
            "} // j().\n",
        ));
        log.borrow_mut().clear();
    }
    // skip_params/skip_params
    {
        #[loggable(skip_params)] // Must skip params.
        fn j(_u: u8) {
            #[loggable(skip_params)] // Must log prefix, skip params.
            fn k(_i: i8) {}
            k(3);
        }
        j(4);
        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(..) {\n",        // Skip params.
            "  j::k(..) {}\n",  // Prefix. Skip params.
            "} // j().\n",
        ));
        log.borrow_mut().clear();
    }

    // skip_params/log_params
    {
        #[loggable(skip_params)]    // Must skip params.
        fn j(_u: u8) {
            #[loggable(log_params)] // Must log prefix, params.
            fn k(_i: i8) {}
            k(3);
        }
        j(4);
        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(..) {\n",            // Skip params.
            "  j::k(_i: 3) {}\n",   // Log prefix, params.
            "} // j().\n",
        ));
        log.borrow_mut().clear();
    }

    // log_params/Absent
    {
        #[loggable(log_params)] // Must log params.
        fn j(_u: u8) {
            // Attribute is absent. Must log prefix, params.
            fn k(_i: i8) {}
            k(3);
        }
        j(4);
        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(_u: 4) {\n",         // Params.
            "  j::k(_i: 3) {}\n",   // Prefix, params.
            "} // j().\n",
        ));
        log.borrow_mut().clear();
    }

    // log_params/{No args}
    {
        #[loggable(log_params)] // Must log params.
        fn j(_u: u8) {
            #[loggable] // Must lop prefix, params.
            fn k(_i: i8) {}
            k(3);
        }
        j(4);
        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(_u: 4) {\n",         // Params.
            "  j::k(_i: 3) {}\n",   // Prefix, params.
            "} // j().\n",
        ));
        log.borrow_mut().clear();
    }

    // log_params/skip_params
    {
        #[loggable(log_params)] // Must log params.
        fn j(_u: u8) {
            #[loggable(skip_params)] // Must log prefix, skip params.
            fn k(_i: i8) {}
            k(3);
        }
        j(4);
        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(_u: 4) {\n",     // Params.
            "  j::k(..) {}\n",  // Prefix, skip params.
            "} // j().\n",
        ));
        log.borrow_mut().clear();
    }

    // log_params/log_params
    {
        #[loggable(log_params)] // Must log params.
        fn j(_u: u8) {
            #[loggable(log_params)] // Must log prefix, params.
            fn k(_i: i8) {}
            k(3);
        }
        j(4);
        flush_log();
        #[rustfmt::skip]
        test_assert!(log, concat!(
            "j(_u: 4) {\n",         // Params.
            "  j::k(_i: 3) {}\n",   // Prefix, params.
            "} // j().\n",
        ));
        log.borrow_mut().clear();
    }

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
