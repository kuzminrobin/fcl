use fcl_proc_macros::loggable;

use crate::common::*;
/////////////////////////////////////////////
// #[loggable]         | TestCases. (trait, fn in macro)
// Attribute           | 
// Values              | c: Copied from `trait_fn` and adapted                      |   Notes
//---------------------|------------------------------------------------------------|-------------
// Absent              |cc|c |c |c |   | c|  |  |  |   | c|  |  |  |    | c|  |  |  |   // ``
// NoArgs              |  | c|  |  |   |c |cc|c |c |   |  | c|  |  |    |  | c|  |  |   // `#[loggable]`
// skip_*              |  |  | c|  |   |  |  | c|  |   |c |c |cc|c |    |  |  | c|  |   // `#[loggable(skip_params, skip_closure_coords)]`
// log_*               |  |  |  | c|   |  |  |  | c|   |  |  |  | c|    |c |c |c |cc|   // `#[loggable(log_params, log_closure_coords)]`

// Test code that is to generate the FCL log:
// 
// // Absent, NoArgs, {skip,log}_{params,closure_coords}
// trait Tr {
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
// impl Tr for i8 {}
// i8::af(1);   // Call associated function.
// 1.m();       // Call method.
/////////////////////////////////////////////

// TODO: `macro_rules` definition inside of an enclosing entity (test the args/prefix passing).

// TODO: Consider and document the handling of 
// * the other attrs before and after `#[loggable`;
// * combinations of multiple `#[loggable`, `#[non_loggable]`.

// Other attrs before. 
#[loggable] // TODO: Consider/document/implement testing the args: (prefix=..., {log,skip}_{params,closure_coords})
// Other attrs after.
macro_rules! trait_contents {
    (
        // TODO: Test the user's params here (e.g. `fn` def-n passed as an arg, how it will be logged).
    ) => {
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
}

/*
// Other attrs before.
// Other attrs after.
macro_rules! loggable_macro_trait_contents {
    ($prefix:path, $params_setting:ident, $closure_coords_setting:ident,) => {
        // No any other attrs.
        #[loggable_block_contents(prefix =  $prefix, $params_setting, $closure_coords_setting)]
        // No any other attrs.
        fn loggable_block_contents() {
            fn absent_af(_p: u8) {
                Some(0).map(|x| x);
            }
            fn absent_m(&self) {
                Some(2).map(|y| y);
            }
            #[loggable]
            fn noargs_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable]
            fn noargs_m(&self) {
                Some(2).map(|y| y);
            }
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(skip_params, skip_closure_coords)]
            fn skip_m(&self) {
                Some(2).map(|y| y);
            }
            #[loggable(log_params, log_closure_coords)]
            fn log_af(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(log_params, log_closure_coords)]
            fn log_m(&self) {
                Some(2).map(|y| y);
            }
        }
    };
}
*/

// NOTE: Cannot be converted to a `fn` since uses the local `trait` and `impl`.
macro_rules! trait_fn_calls {
    () => {
        // Associated functions.
        i8::absent_af(1);
        i8::noargs_af(1);
        i8::skip_af(1);
        i8::log_af(1);

        // Methods.
        1.absent_m();
        1.noargs_m();
        1.skip_m();
        1.log_m();
    }
}

#[test]
fn trait_macro() {
    let log = substitute_log_writer();

    {
        // Absent
        trait Tr {
            // #[loggable_block_contents(prefix =  :: ,log_params,log_closure_coords)]
            // mod loggable_block_contents {
            //     fn absent_af(_p: u8) {
            //         Some(0).map(|x| x);
            //     }
            //     fn absent_m(&self) {
            //         Some(2).map(|y| y);
            //     }
            //     #[loggable]
            //     fn noargs_af(_p: u8) {
            //         Some(0).map(|x| x);
            //     }
            //     #[loggable]
            //     fn noargs_m(&self) {
            //         Some(2).map(|y| y);
            //     }
            //     #[loggable(skip_params, skip_closure_coords)]
            //     fn skip_af(_p: u8) {
            //         Some(0).map(|x| x);
            //     }
            //     #[loggable(skip_params, skip_closure_coords)]
            //     fn skip_m(&self) {
            //         Some(2).map(|y| y);
            //     }
            //     #[loggable(log_params, log_closure_coords)]
            //     fn log_af(_p: u8) {
            //         Some(0).map(|x| x);
            //     }
            //     #[loggable(log_params, log_closure_coords)]
            //     fn log_m(&self) {
            //         Some(2).map(|y| y);
            //     }
            // }            

            // #[fcl_proc_macros::loggable_block_contents(prefix = ::trait_contents, log_params,log_closure_coords)]fn loggable_block_contents(){
            //     fn absent_af(_p:u8){
            //         Some(0).map(|x|x);
            //     }fn absent_m(&self){
            //         Some(2).map(|y|y);
            //     }#[loggable]fn noargs_af(_p:u8){
            //         Some(0).map(|x|x);
            //     }#[loggable]fn noargs_m(&self){
            //         Some(2).map(|y|y);
            //     }#[loggable(skip_params,skip_closure_coords)]fn skip_af(_p:u8){
            //         Some(0).map(|x|x);
            //     }#[loggable(skip_params,skip_closure_coords)]fn skip_m(&self){
            //         Some(2).map(|y|y);
            //     }#[loggable(log_params,log_closure_coords)]fn log_af(_p:u8){
            //         Some(0).map(|x|x);
            //     }#[loggable(log_params,log_closure_coords)]fn log_m(&self){
            //         Some(2).map(|y|y);
            //     }
            // }

            // TODO: Document this {
            // Other attrs before. // They are considered handled before the `#[loggable` below (and they are absent when expanding `#[loggable` below). 
            #[loggable] // Instruments/prepends the macro name and adds 3 macro args. Retains the `Other attrs after` below.
            // Other attrs after. // They get the instrumented/prepended macro name with 3 extra macro args.
            trait_contents!{}   // Expands to:
            /*
            // No any other attrs.
            #[loggable_block_contents(prefix =  :: ,log_params,log_closure_coords)]
            // No any other attrs.
            fn loggable_block_contents() {
                fn absent_af(_p: u8) {
                    Some(0).map(|x| x);
                }
                . . .
            */            
            // } // TODO: Document this.


            // // Absent
            // fn absent_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // fn absent_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // NoArgs
            // #[loggable]
            // fn noargs_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable]
            // fn noargs_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // skip_*
            // #[loggable(skip_params, skip_closure_coords)]
            // fn skip_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable(skip_params, skip_closure_coords)]
            // fn skip_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // log_*
            // #[loggable(log_params, log_closure_coords)]
            // fn log_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable(log_params, log_closure_coords)]
            // fn log_m(&self) {
            //     Some(2).map(|y| y);
            // }
        }
        impl Tr for i8 {}

        // Generate log.
        trait_fn_calls!();
        // i8::absent_af(1); //    absent_af(_p: 1) { absent_af::closure{<coords>}(x: 0) {} -> 0 }
        // i8::noargs_af(1); //    noargs_af(_p: 1) { noargs_af::closure{<coords>}(x: 0) {} -> 0 }
        // i8::skip_af(1); //      skip  _af(..   ) { skip  _af::closure{..      }(..  ) {} -> 0 }
        // i8::log_af(1); //       log   _af(_p: 1) { log   _af::closure{<coords>}(x: 0) {} -> 0 }

        // 1.absent_m(); //        absent_m (self: &1) { absent_m ::closure{<coords>}(y: 2) {} -> 2 }
        // 1.noargs_m(); //        noargs_m (self: &1) { noargs_m ::closure{<coords>}(y: 2) {} -> 2 }
        // 1.skip_m(); //          skip  _m (..      ) { skip  _m ::closure{..      }(..  ) {} -> 2 }
        // 1.log_m(); //           noargs_m (self: &1) { noargs_m ::closure{<coords>}(y: 2) {} -> 2 }

        flush_log();
        let log_contents = zero_out_closure_coords(log.clone());
        assert_eq!(
            log_contents,
            concat!(
                "__::trait_contents::absent_af(_p: 1) {\n",
                "  __::trait_contents::absent_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // __::trait_contents::absent_af().\n",
                "__::trait_contents::noargs_af(_p: 1) {\n",
                "  __::trait_contents::noargs_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // __::trait_contents::noargs_af().\n",
                "__::trait_contents::skip_af(..) {\n",
                "  __::trait_contents::skip_af::closure{..}(..) {} -> 0\n",
                "} // __::trait_contents::skip_af().\n",
                "__::trait_contents::log_af(_p: 1) {\n",
                "  __::trait_contents::log_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // __::trait_contents::log_af().\n",

                "__::trait_contents::absent_m(self: &1) {\n",
                "  __::trait_contents::absent_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // __::trait_contents::absent_m().\n",
                "__::trait_contents::noargs_m(self: &1) {\n",
                "  __::trait_contents::noargs_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // __::trait_contents::noargs_m().\n",
                "__::trait_contents::skip_m(..) {\n",
                "  __::trait_contents::skip_m::closure{..}(..) {} -> 2\n",
                "} // __::trait_contents::skip_m().\n",
                "__::trait_contents::log_m(self: &1) {\n",
                "  __::trait_contents::log_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // __::trait_contents::log_m().\n",
            )
        );
        log.borrow_mut().clear();
    }

    {
        #[loggable] // NoArgs
        trait Tr {
            #[loggable]
            trait_contents!{}
            // // The contents of the trait cannot be extracted into a macro
            // // since the trait's `#[loggable]` cannot 
            // // {penetrate into the macro invocation 
            // // and instrument the result of the macro expansion}.
            // // See details in `quote_as_macro()`.

            // // Absent
            // fn absent_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // fn absent_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // NoArgs
            // #[loggable]
            // fn noargs_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable]
            // fn noargs_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // skip_*
            // #[loggable(skip_params, skip_closure_coords)]
            // fn skip_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable(skip_params, skip_closure_coords)]
            // fn skip_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // log_*
            // #[loggable(log_params, log_closure_coords)]
            // fn log_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable(log_params, log_closure_coords)]
            // fn log_m(&self) {
            //     Some(2).map(|y| y);
            // }
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
                "Tr::trait_contents::absent_af(_p: 1) {\n",
                "  Tr::trait_contents::absent_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::trait_contents::absent_af().\n",
                "Tr::trait_contents::noargs_af(_p: 1) {\n",
                "  Tr::trait_contents::noargs_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::trait_contents::noargs_af().\n",
                "Tr::trait_contents::skip_af(..) {\n",
                "  Tr::trait_contents::skip_af::closure{..}(..) {} -> 0\n",
                "} // Tr::trait_contents::skip_af().\n",
                "Tr::trait_contents::log_af(_p: 1) {\n",
                "  Tr::trait_contents::log_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::trait_contents::log_af().\n",

                "Tr::trait_contents::absent_m(self: &1) {\n",
                "  Tr::trait_contents::absent_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::trait_contents::absent_m().\n",
                "Tr::trait_contents::noargs_m(self: &1) {\n",
                "  Tr::trait_contents::noargs_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::trait_contents::noargs_m().\n",
                "Tr::trait_contents::skip_m(..) {\n",
                "  Tr::trait_contents::skip_m::closure{..}(..) {} -> 2\n",
                "} // Tr::trait_contents::skip_m().\n",
                "Tr::trait_contents::log_m(self: &1) {\n",
                "  Tr::trait_contents::log_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::trait_contents::log_m().\n",
            )
        );
        log.borrow_mut().clear();
    }

    {
        #[loggable(skip_params, skip_closure_coords)] // skip_*
        trait Tr {
            #[loggable]
            trait_contents!{}

            // // Absent
            // fn absent_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // fn absent_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // NoArgs
            // #[loggable]
            // fn noargs_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable]
            // fn noargs_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // skip_*
            // #[loggable(skip_params, skip_closure_coords)]
            // fn skip_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable(skip_params, skip_closure_coords)]
            // fn skip_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // log_*
            // #[loggable(log_params, log_closure_coords)]
            // fn log_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable(log_params, log_closure_coords)]
            // fn log_m(&self) {
            //     Some(2).map(|y| y);
            // }
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
                "Tr::trait_contents::absent_af(..) {\n",
                "  Tr::trait_contents::absent_af::closure{..}(..) {} -> 0\n",
                "} // Tr::trait_contents::absent_af().\n",
                "Tr::trait_contents::noargs_af(..) {\n",
                "  Tr::trait_contents::noargs_af::closure{..}(..) {} -> 0\n",
                "} // Tr::trait_contents::noargs_af().\n",
                "Tr::trait_contents::skip_af(..) {\n",
                "  Tr::trait_contents::skip_af::closure{..}(..) {} -> 0\n",
                "} // Tr::trait_contents::skip_af().\n",
                "Tr::trait_contents::log_af(_p: 1) {\n",
                "  Tr::trait_contents::log_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::trait_contents::log_af().\n",

                "Tr::trait_contents::absent_m(..) {\n",
                "  Tr::trait_contents::absent_m::closure{..}(..) {} -> 2\n",
                "} // Tr::trait_contents::absent_m().\n",
                "Tr::trait_contents::noargs_m(..) {\n",
                "  Tr::trait_contents::noargs_m::closure{..}(..) {} -> 2\n",
                "} // Tr::trait_contents::noargs_m().\n",
                "Tr::trait_contents::skip_m(..) {\n",
                "  Tr::trait_contents::skip_m::closure{..}(..) {} -> 2\n",
                "} // Tr::trait_contents::skip_m().\n",
                "Tr::trait_contents::log_m(self: &1) {\n",
                "  Tr::trait_contents::log_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::trait_contents::log_m().\n",
            )
        );
        log.borrow_mut().clear();
    }

    {
        #[loggable(log_params, log_closure_coords)] // log_*
        trait Tr {
            #[loggable]
            trait_contents!{}

            // // Absent
            // fn absent_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // fn absent_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // NoArgs
            // #[loggable]
            // fn noargs_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable]
            // fn noargs_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // skip_*
            // #[loggable(skip_params, skip_closure_coords)]
            // fn skip_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable(skip_params, skip_closure_coords)]
            // fn skip_m(&self) {
            //     Some(2).map(|y| y);
            // }

            // // log_*
            // #[loggable(log_params, log_closure_coords)]
            // fn log_af(_p: u8) {
            //     Some(0).map(|x| x);
            // }
            // #[loggable(log_params, log_closure_coords)]
            // fn log_m(&self) {
            //     Some(2).map(|y| y);
            // }
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
                "Tr::trait_contents::absent_af(_p: 1) {\n",
                "  Tr::trait_contents::absent_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::trait_contents::absent_af().\n",
                "Tr::trait_contents::noargs_af(_p: 1) {\n",
                "  Tr::trait_contents::noargs_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::trait_contents::noargs_af().\n",
                "Tr::trait_contents::skip_af(..) {\n",
                "  Tr::trait_contents::skip_af::closure{..}(..) {} -> 0\n",
                "} // Tr::trait_contents::skip_af().\n",
                "Tr::trait_contents::log_af(_p: 1) {\n",
                "  Tr::trait_contents::log_af::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // Tr::trait_contents::log_af().\n",

                "Tr::trait_contents::absent_m(self: &1) {\n",
                "  Tr::trait_contents::absent_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::trait_contents::absent_m().\n",
                "Tr::trait_contents::noargs_m(self: &1) {\n",
                "  Tr::trait_contents::noargs_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::trait_contents::noargs_m().\n",
                "Tr::trait_contents::skip_m(..) {\n",
                "  Tr::trait_contents::skip_m::closure{..}(..) {} -> 2\n",
                "} // Tr::trait_contents::skip_m().\n",
                "Tr::trait_contents::log_m(self: &1) {\n",
                "  Tr::trait_contents::log_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // Tr::trait_contents::log_m().\n",
            )
        );
        log.borrow_mut().clear();
    }
}

// TODO: Closure defined in one fn but called by another fn.