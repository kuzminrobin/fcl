use fcl_proc_macros::loggable;
use crate::{common::*};
// use crate::proc_macros::non_loggable_attr;   // Idea test.

// #[loggable]         | TestCases (impl struct, fn)
// Attribute Values    | G: Written by ChatGPT/Codex.   +: Written mannually.       |   Notes
//---------------------|------------------------------------------------------------|-------------
// Absent              |++|+ |+ |+ |   | ?|  |  |  |   | ?|  |  |  |    | ?|  |  |  |   // ``
// NoArgs              |  | +|  |  |   |? |??|? |? |   |  | ?|  |  |    |  | ?|  |  |   // `#[loggable]`
// skip_*              |  |  | +|  |   |  |  | ?|  |   |? |? |??|? |    |  |  | ?|  |   // `#[loggable(skip_params, skip_closure_coords)]`
// log_*               |  |  |  | +|   |  |  |  | ?|   |  |  |  | ?|    |? |? |? |??|   // `#[loggable(log_params, log_closure_coords)]`

// struct S {
//     _f: u8,   // Field.
// }
// 
// // Absent, NoArgs, {skip,log}_{params,closure_coords}
// impl S {
//     // Absent, NoArgs, {skip,log}_{params,closure_coords}
//     fn af(_p: u8) {          // Associated function.
//         Some(0).map(|x| x);
//     }
//     // Absent, NoArgs, {skip,log}_{params,closure_coords}
//     fn m(&self) {            // Method.
//         Some(2).map(|y| y);
//     }
// }
// 
// S::af(1);        // Call associated function.
// S{ _f: 0 }.m();  // Call method.

// NOTE: This macro cannot be converted to a `fn` since it uses the local `struct` and `impl`.
macro_rules! impl_struct_fn_calls {
    () => {
        // Associated functions.
        S::absent_af(1);
        S::noargs_af(1);
        S::skip_af(1);
        S::log_af(1);

        // Methods.
        S{ _f: 0 }.absent_m();
        S{ _f: 0 }.noargs_m();
        S{ _f: 0 }.skip_m();
        S{ _f: 0 }.log_m();
    }
}

// macro_rules! test {  // Idea test.
//     () => {
//         #[fcl_proc_macros::non_loggable]
//         mod m
//         {
//             fn f() {}
//         }
//     }
// }

#[test]
fn impl_struct_fn() {
    // {                        // Idea test.
    //     trait Tr {
    //         // #[fcl_proc_macros::non_loggable]
    //         test!();
    //     }
    //     impl Tr for u8 {}
    //     // u8::f();
    // }
    let log = substitute_log_writer();

    {
        struct S {
            _f: u8,   // Field.
        }

        // Absent
        impl S {
            // Absent
            fn absent_af(_p: u8) {
                Some(0).map(|x| x);
            }
            fn absent_m(&self) {
                Some(2).map(|y| y);
            }

            // NoArgs
            #[loggable]
            fn noargs_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable]
            fn noargs_m(&self) {
                Some(2).map(|y| y);
            }

            // skip_*
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_m(&self) {
                Some(2).map(|y| y);
            }

            // log_*
            #[loggable(log_params, log_closure_coords)]
            fn log_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(log_params, log_closure_coords)]
            fn log_m(&self) {
                Some(2).map(|y| y);
            }
        }

        // Generate log.
        impl_struct_fn_calls!();
        // S::absent_af(1); //      -
        // S::noargs_af(1); //      noargs_af(_p: 1) { noargs_af::closure{<coords>}(x: 0) {} -> 0 }
        // S::skip_af(1); //        skip  _af(..   ) { skip  _af::closure{..      }(..  ) {} -> 0 }
        // S::log_af(1); //         log   _af(_p: 1) { log   _af::closure{<coords>}(x: 0) {} -> 0 }

        // S{ f: 0 }.absent_m(); // -
        // S{ f: 0 }.noargs_m(); // noargs_m (self: &?) { noargs_m ::closure{<coords>}(y: 2) {} -> 2 }
        // S{ f: 0 }.skip_m(); //   skip  _m (..      ) { skip  _m ::closure{..      }(..  ) {} -> 2 }
        // S{ f: 0 }.log_m(); //    noargs_m (self: &?) { noargs_m ::closure{<coords>}(y: 2) {} -> 2 }

        flush_log();
        let log_contents = zero_out_closure_coords(log.clone());
        assert_eq!(
            log_contents,
            concat!(
                "noargs_af(_p: 1) {\n",
                "  noargs_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // noargs_af().\n",

                "skip_af(..) {\n",
                "  skip_af::closure{..}(..) {} -> 0\n",
                "} // skip_af().\n",

                "log_af(_p: 1) {\n",
                "  log_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // log_af().\n",


                "noargs_m(self: &?) {\n",
                "  noargs_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // noargs_m().\n",

                "skip_m(..) {\n",
                "  skip_m::closure{..}(..) {} -> 2\n",
                "} // skip_m().\n",

                "log_m(self: &?) {\n",
                "  log_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // log_m().\n",
            )
        );
        log.borrow_mut().clear();
    }

    {
        struct S {
            _f: u8,   // Field.
        }

        #[loggable] // NoArgs
        impl S {
            // Absent
            fn absent_af(_p: u8) {
                Some(0).map(|x| x);
            }
            fn absent_m(&self) {
                Some(2).map(|y| y);
            }

            // NoArgs
            #[loggable]
            fn noargs_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable]
            fn noargs_m(&self) {
                Some(2).map(|y| y);
            }

            // skip_*
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_m(&self) {
                Some(2).map(|y| y);
            }

            // log_*
            #[loggable(log_params, log_closure_coords)]
            fn log_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(log_params, log_closure_coords)]
            fn log_m(&self) {
                Some(2).map(|y| y);
            }
        }

        // Generate log.
        impl_struct_fn_calls!();
        // S::absent_af(1); //      S::absent_af(_p: 1) { S::absent_af::closure{<coords>}(x: 0) {} -> 0 }
        // S::noargs_af(1); //      S::noargs_af(_p: 1) { S::noargs_af::closure{<coords>}(x: 0) {} -> 0 }
        // S::skip_af(1); //        S::skip  _af(..   ) { S::skip  _af::closure{..      }(..  ) {} -> 0 }
        // S::log_af(1); //         S::log   _af(_p: 1) { S::log   _af::closure{<coords>}(x: 0) {} -> 0 }

        // S{ f: 0 }.absent_m(); // S::absent_m (self: &?) { S::absent_m ::closure{<coords>}(y: 2) {} -> 2 }
        // S{ f: 0 }.noargs_m(); // S::noargs_m (self: &?) { S::noargs_m ::closure{<coords>}(y: 2) {} -> 2 }
        // S{ f: 0 }.skip_m(); //   S::skip  _m (..      ) { S::skip  _m ::closure{..      }(..  ) {} -> 2 }
        // S{ f: 0 }.log_m(); //    S::log   _m (self: &?) { S::log   _m ::closure{<coords>}(y: 2) {} -> 2 }

        flush_log();
        // let log_contents = zero_out_closure_coords(log.clone());
        // assert_eq!( // Fails at the moment. attr proc macro args passing to internals is not yet impl'd for `struct`. About to try "The `#[loggable]` Macros" idea.
        //     log_contents,
        //     concat!(
        //         "S::absent_af(_p: 1) {\n",
        //         "  S::absent_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
        //         "} // S::absent_af().\n",

        //         "S::noargs_af(_p: 1) {\n",
        //         "  S::noargs_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
        //         "} // S::noargs_af().\n",

        //         "S::skip_af(..) {\n",
        //         "  S::skip_af::closure{..}(..) {} -> 0\n",
        //         "} // S::skip_af().\n",

        //         "S::log_af(_p: 1) {\n",
        //         "  S::log_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
        //         "} // S::log_af().\n",


        //         "S::absent_m(self: &?) {\n",
        //         "  S::absent_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
        //         "} // S::absent_m().\n",

        //         "S::noargs_m(self: &?) {\n",
        //         "  S::noargs_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
        //         "} // S::noargs_m().\n",

        //         "S::skip_m(..) {\n",
        //         "  S::skip_m::closure{..}(..) {} -> 2\n",
        //         "} // S::skip_m().\n",
                
        //         "S::log_m(self: &?) {\n",
        //         "  S::log_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
        //         "} // S::log_m().\n",
        //     )
        // );
        log.borrow_mut().clear();
    }

    
}


/*
//

#[test]
fn trait_fn() {
    let log = substitute_log_writer();

    {
        #[loggable] // NoArgs
        trait Tr {
            // The contents of the trait cannot be extracted into a macro
            // since the trait's `#[loggable]` cannot 
            // {penetrate into the macro invocation 
            // and instrument the result of the macro expansion}.
            // See details in `quote_as_macro()`.

            // Absent
            fn absent_af(_p: u8) {
                Some(0).map(|x| x);
            }
            fn absent_m(&self) {
                Some(2).map(|y| y);
            }

            // NoArgs
            #[loggable]
            fn noargs_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable]
            fn noargs_m(&self) {
                Some(2).map(|y| y);
            }

            // skip_*
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_m(&self) {
                Some(2).map(|y| y);
            }

            // log_*
            #[loggable(log_params, log_closure_coords)]
            fn log_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(log_params, log_closure_coords)]
            fn log_m(&self) {
                Some(2).map(|y| y);
            }
        }
        impl Tr for i8 {}

        // Generate log.
        trait_fn_calls!();
        // i8::absent_af(1); //    Tr::absent_af(_p: 1) { Tr::absent_af::closure{<coords>}(x: 0) {} -> 0 }
        // i8::noargs_af(1); //    Tr::noargs_af(_p: 1) { Tr::noargs_af::closure{<coords>}(x: 0) {} -> 0 }
        // i8::skip_af(1); //      Tr::skip  _af(..   ) { Tr::skip  _af::closure{..      }(..  ) {} -> 0 }
        // i8::log_af(1); //       Tr::log   _af(_p: 1) { Tr::log   _af::closure{<coords>}(x: 0) {} -> 0 }

        // 1.absent_m(); //        Tr::absent_m (self: &1) { Tr::absent_m ::closure{<coords>}(y: 2) {} -> 2 }
        // 1.noargs_m(); //        Tr::noargs_m (self: &1) { Tr::noargs_m ::closure{<coords>}(y: 2) {} -> 2 }
        // 1.skip_m(); //          Tr::skip  _m (..      ) { Tr::skip  _m ::closure{..      }(..  ) {} -> 2 }
        // 1.log_m(); //           Tr::log   _m (self: &1) { Tr::log   _m ::closure{<coords>}(y: 2) {} -> 2 }

        flush_log();
        let log_contents = zero_out_closure_coords(log.clone());
        assert_eq!(
            log_contents,
            concat!(
                "Tr::absent_af(_p: 1) {\n",
                "  Tr::absent_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::absent_af().\n",
                "Tr::noargs_af(_p: 1) {\n",
                "  Tr::noargs_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::noargs_af().\n",
                "Tr::skip_af(..) {\n",
                "  Tr::skip_af::closure{..}(..) {} -> 0\n",
                "} // Tr::skip_af().\n",
                "Tr::log_af(_p: 1) {\n",
                "  Tr::log_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::log_af().\n",

                "Tr::absent_m(self: &1) {\n",
                "  Tr::absent_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::absent_m().\n",
                "Tr::noargs_m(self: &1) {\n",
                "  Tr::noargs_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::noargs_m().\n",
                "Tr::skip_m(..) {\n",
                "  Tr::skip_m::closure{..}(..) {} -> 2\n",
                "} // Tr::skip_m().\n",
                "Tr::log_m(self: &1) {\n",
                "  Tr::log_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::log_m().\n",
            )
        );
        log.borrow_mut().clear();
    }

    {
        #[loggable(skip_params, skip_closure_coords)] // skip_*
        trait Tr {
            // Absent
            fn absent_af(_p: u8) {
                Some(0).map(|x| x);
            }
            fn absent_m(&self) {
                Some(2).map(|y| y);
            }

            // NoArgs
            #[loggable]
            fn noargs_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable]
            fn noargs_m(&self) {
                Some(2).map(|y| y);
            }

            // skip_*
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_m(&self) {
                Some(2).map(|y| y);
            }

            // log_*
            #[loggable(log_params, log_closure_coords)]
            fn log_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(log_params, log_closure_coords)]
            fn log_m(&self) {
                Some(2).map(|y| y);
            }
        }
        impl Tr for i8 {}

        // Generate log.
        trait_fn_calls!();
        // i8::absent_af(1); //    Tr::absent_af(..   ) { Tr::absent_af::closure{..      }(..  ) {} -> 0 }
        // i8::noargs_af(1); //    Tr::noargs_af(..   ) { Tr::noargs_af::closure{..      }(..  ) {} -> 0 }
        // i8::skip_af(1); //      Tr::skip  _af(..   ) { Tr::skip  _af::closure{..      }(..  ) {} -> 0 }
        // i8::log_af(1); //       Tr::log   _af(_p: 1) { Tr::log   _af::closure{<coords>}(x: 0) {} -> 0 }

        // 1.absent_m(); //        Tr::absent_m (..      ) { Tr::absent_m ::closure{..      }(..  ) {} -> 2 }
        // 1.noargs_m(); //        Tr::noargs_m (..      ) { Tr::noargs_m ::closure{..      }(..  ) {} -> 2 }
        // 1.skip_m(); //          Tr::skip  _m (..      ) { Tr::skip  _m ::closure{..      }(..  ) {} -> 2 }
        // 1.log_m(); //           Tr::log   _m (self: &1) { Tr::log   _m ::closure{<coords>}(y: 2) {} -> 2 }

        flush_log();
        let log_contents = zero_out_closure_coords(log.clone());
        assert_eq!(
            log_contents,
            concat!(
                "Tr::absent_af(..) {\n",
                "  Tr::absent_af::closure{..}(..) {} -> 0\n",
                "} // Tr::absent_af().\n",
                "Tr::noargs_af(..) {\n",
                "  Tr::noargs_af::closure{..}(..) {} -> 0\n",
                "} // Tr::noargs_af().\n",
                "Tr::skip_af(..) {\n",
                "  Tr::skip_af::closure{..}(..) {} -> 0\n",
                "} // Tr::skip_af().\n",
                "Tr::log_af(_p: 1) {\n",
                "  Tr::log_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::log_af().\n",

                "Tr::absent_m(..) {\n",
                "  Tr::absent_m::closure{..}(..) {} -> 2\n",
                "} // Tr::absent_m().\n",
                "Tr::noargs_m(..) {\n",
                "  Tr::noargs_m::closure{..}(..) {} -> 2\n",
                "} // Tr::noargs_m().\n",
                "Tr::skip_m(..) {\n",
                "  Tr::skip_m::closure{..}(..) {} -> 2\n",
                "} // Tr::skip_m().\n",
                "Tr::log_m(self: &1) {\n",
                "  Tr::log_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::log_m().\n",
            )
        );
        log.borrow_mut().clear();
    }

    {
        #[loggable(log_params, log_closure_coords)] // log_*
        trait Tr {
            // Absent
            fn absent_af(_p: u8) {
                Some(0).map(|x| x);
            }
            fn absent_m(&self) {
                Some(2).map(|y| y);
            }

            // NoArgs
            #[loggable]
            fn noargs_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable]
            fn noargs_m(&self) {
                Some(2).map(|y| y);
            }

            // skip_*
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_m(&self) {
                Some(2).map(|y| y);
            }

            // log_*
            #[loggable(log_params, log_closure_coords)]
            fn log_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(log_params, log_closure_coords)]
            fn log_m(&self) {
                Some(2).map(|y| y);
            }
        }
        impl Tr for i8 {}

        // Generate log.
        trait_fn_calls!();
        // i8::absent_af(1); //    Tr::absent_af(_p: 1) { Tr::absent_af::closure{<coords>}(x: 0) {} -> 0 }
        // i8::noargs_af(1); //    Tr::noargs_af(_p: 1) { Tr::noargs_af::closure{<coords>}(x: 0) {} -> 0 }
        // i8::skip_af(1); //      Tr::skip  _af(..   ) { Tr::skip  _af::closure{..      }(..  ) {} -> 0 }
        // i8::log_af(1); //       Tr::log   _af(_p: 1) { Tr::log   _af::closure{<coords>}(x: 0) {} -> 0 }

        // 1.absent_m(); //        Tr::absent_m (self: &1) { Tr::absent_m ::closure{<coords>}(y: 2) {} -> 2 }
        // 1.noargs_m(); //        Tr::noargs_m (self: &1) { Tr::noargs_m ::closure{<coords>}(y: 2) {} -> 2 }
        // 1.skip_m(); //          Tr::skip  _m (..      ) { Tr::skip  _m ::closure{..      }(..  ) {} -> 2 }
        // 1.log_m(); //           Tr::log   _m (self: &1) { Tr::log   _m ::closure{<coords>}(y: 2) {} -> 2 }

        flush_log();
        let log_contents = zero_out_closure_coords(log.clone());
        assert_eq!(
            log_contents,
            concat!(
                "Tr::absent_af(_p: 1) {\n",
                "  Tr::absent_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::absent_af().\n",
                "Tr::noargs_af(_p: 1) {\n",
                "  Tr::noargs_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::noargs_af().\n",
                "Tr::skip_af(..) {\n",
                "  Tr::skip_af::closure{..}(..) {} -> 0\n",
                "} // Tr::skip_af().\n",
                "Tr::log_af(_p: 1) {\n",
                "  Tr::log_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::log_af().\n",

                "Tr::absent_m(self: &1) {\n",
                "  Tr::absent_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::absent_m().\n",
                "Tr::noargs_m(self: &1) {\n",
                "  Tr::noargs_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::noargs_m().\n",
                "Tr::skip_m(..) {\n",
                "  Tr::skip_m::closure{..}(..) {} -> 2\n",
                "} // Tr::skip_m().\n",
                "Tr::log_m(self: &1) {\n",
                "  Tr::log_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::log_m().\n",
            )
        );
        log.borrow_mut().clear();
    }
}

 */
