use fcl_proc_macros::loggable;

use crate::common::*;
/////////////////////////////////////////////
// #[loggable]         | TestCases
// Attribute           | (fn, init fn)
// Values              | G: Written by ChatGPT/Codex.   +: Written mannually.       |   Notes
//---------------------|------------------------------------------------------------|-------------
// Absent              |++|+ |+ |+ |   | +|  |  |  |   | +|  |  |  |    | +|  |  |  |   // ``
// NoArgs              |  | +|  |  |   |+ |++|+ |+ |   |  | +|  |  |    |  | +|  |  |   // `#[loggable]`
// skip_*              |  |  | +|  |   |  |  | +|  |   |+ |+ |++|+ |    |  |  | +|  |   // `#[loggable(skip_params, skip_closure_coords)]`
// log_*               |  |  |  | +|   |  |  |  | +|   |  |  |  | +|    |+ |+ |+ |++|   // `#[loggable(log_params, log_closure_coords)]`

// Test code that is to generate the FCL log:
//
// // Absent, NoArgs, {skip,log}_{params,closure_coords}
// fn f(expr_branch: bool) {
//
//      // Absent, NoArgs, {skip,log}_{params,closure_coords}
//      fn g(ret_some: bool, p: u8) -> Option<u8> {
//          if ret_some {
//              Some(p).map(|x| x - 1)  // Assign smth in `let ... else` below.
//          } else {
//              None                    // Go to `else` part in `let ... else` below.
//          }
//      }
//
//      let Some(_y) = g(expr_branch, 1) else {
//          // Absent, NoArgs, {skip,log}_{params,closure_coords}
//          fn h(p: u8) -> Option<u8> {
//              Some(p).map(|x| x + 1)
//          }
//          let _ = h(3);
//          return;
//      };
// }
//
// f(true);     // Call `f::g()`.
// f(false);    // Call `f::g()`, `f::h()`.
/////////////////////////////////////////////

#[test]
fn fn_init_fn() {
    enum NestedAttr {
        Absent, // ``
        NoArgs, // `#[loggable]`
        Skip,   // `#[loggable(skip_params, skip_closure_coords)]`
        Log,    // `#[loggable(log_params, log_closure_coords)]`
    }
    let log = substitute_log_writer();

    // Defining an `fn` instead of a macro here reports an inconvenent line number of an `assert_eq` failure in the body
    // (the same line number for different `fn` calls).
    macro_rules! test_log {
        ($log:expr, $expected_str:expr) => {
            flush_log();
            let log_contents = zero_out_closure_coords($log.clone());
            assert_eq!(log_contents, $expected_str);
            $log.borrow_mut().clear();
        };
    }

    {
        macro_rules! f_body {
            ($expr_branch:expr) => {
                // Absent
                fn g(ret_some: bool, p: u8) -> Option<u8> {
                    if ret_some {
                        Some(p).map(|x| x - 1) // Assign smth in `let` below.
                    } else {
                        None // Go to `else` part in `let ... else` below.
                    }
                }

                let Some(_y) = g($expr_branch, 1) else {
                    // Absent
                    fn h(p: u8) -> Option<u8> {
                        Some(p).map(|x| x + 1)
                    }
                    let _ = h(3);
                    return;
                };
            };
            ($meta:meta, $expr_branch:expr) => {
                #[$meta]
                fn g(ret_some: bool, p: u8) -> Option<u8> {
                    if ret_some {
                        Some(p).map(|x| x - 1) // Assign smth in `let` below.
                    } else {
                        None // Go to `else` part in `let ... else` below.
                    }
                }

                let Some(_y) = g($expr_branch, 1) else {
                    #[$meta]
                    fn h(p: u8) -> Option<u8> {
                        Some(p).map(|x| x + 1)
                    }
                    let _ = h(3);
                    return;
                };
            };
        }
        // Absent
        fn f(expr_branch: bool, nested_attr: NestedAttr) {
            match nested_attr {
                NestedAttr::Absent => {
                    f_body!(expr_branch);
                }
                NestedAttr::NoArgs => {
                    f_body!(loggable, expr_branch);
                }
                NestedAttr::Skip => {
                    f_body!(loggable(skip_params, skip_closure_coords), expr_branch);
                }
                NestedAttr::Log => {
                    f_body!(loggable(log_params, log_closure_coords), expr_branch);
                }
            }
        }

        // Generate the FCL log and test it:
        f(false, NestedAttr::Absent); // Call `f::g()`, `f::h()`.
        test_log!(log, concat!("",));

        f(false, NestedAttr::NoArgs); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "g(ret_some: false, p: 1) {} -> None\n",
                "h(p: 3) {\n",
                "  h::closure{", closure_coords!(), "}(x: 3) {} -> 4\n",
                "} -> Some(4) // h().\n",
            )
        );

        f(false, NestedAttr::Skip); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "g(..) {} -> None\n",
                "h(..) {\n",
                "  h::closure{", dots!(), "}(..) {} -> 4\n",
                "} -> Some(4) // h().\n",
            )
        );

        f(false, NestedAttr::Log); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "g(ret_some: false, p: 1) {} -> None\n",
                "h(p: 3) {\n",
                "  h::closure{", closure_coords!(), "}(x: 3) {} -> 4\n",
                "} -> Some(4) // h().\n",
            )
        );

        f(true, NestedAttr::Absent); // Call `f::g()`.
        test_log!(log, concat!("",));

        f(true, NestedAttr::NoArgs); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "g(ret_some: true, p: 1) {\n",
                "  g::closure{", closure_coords!(), "}(x: 1) {} -> 0\n",
                "} -> Some(0) // g().\n",
            )
        );

        f(true, NestedAttr::Skip); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "g(..) {\n",
                "  g::closure{", dots!(), "}(..) {} -> 0\n",
                "} -> Some(0) // g().\n",
            )
        );

        f(true, NestedAttr::Log); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "g(ret_some: true, p: 1) {\n",
                "  g::closure{", closure_coords!(), "}(x: 1) {} -> 0\n",
                "} -> Some(0) // g().\n",
            )
        );
    }

    {
        #[loggable]
        fn f(expr_branch: bool, nested_attr: NestedAttr) {
            match nested_attr {
                NestedAttr::Absent => {
                    // Since enclosing `f()` is `#[loggable]`, using macro `f_body!()` here has a different effect. Using functions explicitly.
                    // Absent
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        // Absent
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
                NestedAttr::NoArgs => {
                    #[loggable]
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        #[loggable]
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
                NestedAttr::Skip => {
                    #[loggable(skip_params, skip_closure_coords)]
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        #[loggable(skip_params, skip_closure_coords)]
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
                NestedAttr::Log => {
                    #[loggable(log_params, log_closure_coords)]
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        #[loggable(log_params, log_closure_coords)]
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
            }
        }

        // Generate the FCL log and test it:
        f(false, NestedAttr::Absent); // Call `f::g()`, `f::h()`.
        test_log!(log, 
            concat!(
                "f(expr_branch: false, nested_attr: ?) {\n",
                "  f::g(ret_some: false, p: 1) {} -> None\n",
                "  f::h(p: 3) {\n",
                "    f::h::closure{", closure_coords!(), "}(x: 3) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(false, NestedAttr::NoArgs); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: false, nested_attr: ?) {\n",
                "  f::g(ret_some: false, p: 1) {} -> None\n",
                "  f::h(p: 3) {\n",
                "    f::h::closure{", closure_coords!(), "}(x: 3) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(false, NestedAttr::Skip); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: false, nested_attr: ?) {\n",
                "  f::g(..) {} -> None\n",
                "  f::h(..) {\n",
                "    f::h::closure{", dots!(), "}(..) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(false, NestedAttr::Log); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: false, nested_attr: ?) {\n",
                "  f::g(ret_some: false, p: 1) {} -> None\n",
                "  f::h(p: 3) {\n",
                "    f::h::closure{", closure_coords!(), "}(x: 3) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::Absent); // Call `f::g()`.
        test_log!(log, 
            concat!(
                "f(expr_branch: true, nested_attr: ?) {\n",
                "  f::g(ret_some: true, p: 1) {\n",
                "    f::g::closure{", closure_coords!(), "}(x: 1) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::NoArgs); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: true, nested_attr: ?) {\n",
                "  f::g(ret_some: true, p: 1) {\n",
                "    f::g::closure{", closure_coords!(), "}(x: 1) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::Skip); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: true, nested_attr: ?) {\n",
                "  f::g(..) {\n",
                "    f::g::closure{", dots!(), "}(..) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::Log); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: true, nested_attr: ?) {\n",
                "  f::g(ret_some: true, p: 1) {\n",
                "    f::g::closure{", closure_coords!(), "}(x: 1) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );
    }

    {
        #[loggable(skip_params, skip_closure_coords)]
        fn f(expr_branch: bool, nested_attr: NestedAttr) {
            match nested_attr {
                NestedAttr::Absent => {
                    // Since enclosing `f()` is `#[loggable]`, using macro `f_body!()` here has a different effect. Using functions explicitly.
                    // Absent
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        // Absent
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
                NestedAttr::NoArgs => {
                    #[loggable]
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        #[loggable]
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
                NestedAttr::Skip => {
                    #[loggable(skip_params, skip_closure_coords)]
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        #[loggable(skip_params, skip_closure_coords)]
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
                NestedAttr::Log => {
                    #[loggable(log_params, log_closure_coords)]
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        #[loggable(log_params, log_closure_coords)]
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
            }
        }

        // Generate the FCL log and test it:
        f(false, NestedAttr::Absent); // Call `f::g()`, `f::h()`.
        test_log!(log, 
            concat!(
                "f(..) {\n",
                "  f::g(..) {} -> None\n",
                "  f::h(..) {\n",
                "    f::h::closure{", dots!(), "}(..) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(false, NestedAttr::NoArgs); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "f(..) {\n",
                "  f::g(..) {} -> None\n",
                "  f::h(..) {\n",
                "    f::h::closure{", dots!(), "}(..) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(false, NestedAttr::Skip); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "f(..) {\n",
                "  f::g(..) {} -> None\n",
                "  f::h(..) {\n",
                "    f::h::closure{", dots!(), "}(..) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(false, NestedAttr::Log); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "f(..) {\n",
                "  f::g(ret_some: false, p: 1) {} -> None\n",
                "  f::h(p: 3) {\n",
                "    f::h::closure{", closure_coords!(), "}(x: 3) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::Absent); // Call `f::g()`.
        test_log!(log, 
            concat!(
                "f(..) {\n",
                "  f::g(..) {\n",
                "    f::g::closure{", dots!(), "}(..) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::NoArgs); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "f(..) {\n",
                "  f::g(..) {\n",
                "    f::g::closure{", dots!(), "}(..) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::Skip); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "f(..) {\n",
                "  f::g(..) {\n",
                "    f::g::closure{", dots!(), "}(..) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::Log); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "f(..) {\n",
                "  f::g(ret_some: true, p: 1) {\n",
                "    f::g::closure{", closure_coords!(), "}(x: 1) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );
    }

    {
        #[loggable(log_params, log_closure_coords)]
        fn f(expr_branch: bool, nested_attr: NestedAttr) {
            match nested_attr {
                NestedAttr::Absent => {
                    // Since enclosing `f()` is `#[loggable]`, using macro `f_body!()` here has a different effect. Using functions explicitly.
                    // Absent
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        // Absent
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
                NestedAttr::NoArgs => {
                    #[loggable]
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        #[loggable]
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
                NestedAttr::Skip => {
                    #[loggable(skip_params, skip_closure_coords)]
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        #[loggable(skip_params, skip_closure_coords)]
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
                NestedAttr::Log => {
                    #[loggable(log_params, log_closure_coords)]
                    fn g(ret_some: bool, p: u8) -> Option<u8> {
                        if ret_some {
                            Some(p).map(|x| x - 1) // Assign smth in `let` below.
                        } else {
                            None // Go to `else` part in `let ... else` below.
                        }
                    }

                    let Some(_y) = g(expr_branch, 1) else {
                        #[loggable(log_params, log_closure_coords)]
                        fn h(p: u8) -> Option<u8> {
                            Some(p).map(|x| x + 1)
                        }
                        let _ = h(3);
                        return;
                    };
                }
            }
        }

        // Generate the FCL log and test it:
        f(false, NestedAttr::Absent); // Call `f::g()`, `f::h()`.
        test_log!(log, 
            concat!(
                "f(expr_branch: false, nested_attr: ?) {\n",
                "  f::g(ret_some: false, p: 1) {} -> None\n",
                "  f::h(p: 3) {\n",
                "    f::h::closure{", closure_coords!(), "}(x: 3) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(false, NestedAttr::NoArgs); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: false, nested_attr: ?) {\n",
                "  f::g(ret_some: false, p: 1) {} -> None\n",
                "  f::h(p: 3) {\n",
                "    f::h::closure{", closure_coords!(), "}(x: 3) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(false, NestedAttr::Skip); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: false, nested_attr: ?) {\n",
                "  f::g(..) {} -> None\n",
                "  f::h(..) {\n",
                "    f::h::closure{", dots!(), "}(..) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(false, NestedAttr::Log); // Call `f::g()`, `f::h()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: false, nested_attr: ?) {\n",
                "  f::g(ret_some: false, p: 1) {} -> None\n",
                "  f::h(p: 3) {\n",
                "    f::h::closure{", closure_coords!(), "}(x: 3) {} -> 4\n",
                "  } -> Some(4) // f::h().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::Absent); // Call `f::g()`.
        test_log!(log, 
            concat!(
                "f(expr_branch: true, nested_attr: ?) {\n",
                "  f::g(ret_some: true, p: 1) {\n",
                "    f::g::closure{", closure_coords!(), "}(x: 1) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::NoArgs); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: true, nested_attr: ?) {\n",
                "  f::g(ret_some: true, p: 1) {\n",
                "    f::g::closure{", closure_coords!(), "}(x: 1) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::Skip); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: true, nested_attr: ?) {\n",
                "  f::g(..) {\n",
                "    f::g::closure{", dots!(), "}(..) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );

        f(true, NestedAttr::Log); // Call `f::g()`.
        test_log!(
            log,
            concat!(
                "f(expr_branch: true, nested_attr: ?) {\n",
                "  f::g(ret_some: true, p: 1) {\n",
                "    f::g::closure{", closure_coords!(), "}(x: 1) {} -> 0\n",
                "  } -> Some(0) // f::g().\n",
                "} // f().\n",
            )
        );
    }
}
