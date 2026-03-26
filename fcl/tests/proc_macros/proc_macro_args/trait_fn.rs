use std::cell::RefCell;
use std::rc::Rc;
use fcl_proc_macros::loggable;
use crate::common::*;
//
// #[loggable]         | TestCases
// Attribute           | (trait, fn)
// Values              | G: Written by ChatGPT/Codex.   +: Written mannually.       |   Notes
//---------------------|------------------------------------------------------------|-------------
// Absent              |++|+ |+ |+ |   | G|  |  |  |   | G|  |  |  |    | G|  |  |  |   // ``
// NoArgs              |  | +|  |  |   |G |GG|G |G |   |  | G|  |  |    |  | G|  |  |   // `#[loggable]`
// skip_*              |  |  | +|  |   |  |  | G|  |   |G |G |GG|G |    |  |  | G|  |   // `#[loggable(skip_params, skip_closure_coords)]`
// log_*               |  |  |  | +|   |  |  |  | G|   |  |  |  | G|    |G |G |G |GG|   // `#[loggable(log_params, log_closure_coords)]`

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
// impl Tr for i8 {}

// i8::af(1);
// 1.m();

#[test]
fn trit_fn() {
    let log = substitute_log_writer!();

    {
        // Absent
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
        i8::absent_af(1); //    -
        i8::noargs_af(1); //    noargs_af(_p: 1) { noargs_af::closure{<coords>}(x: 0) {} -> 0 }
        i8::skip_af(1); //      skip  _af(..   ) { skip  _af::closure{..      }(..  ) {} -> 0 }
        i8::log_af(1); //       log   _af(_p: 1) { log   _af::closure{<coords>}(x: 0) {} -> 0 }

        1.absent_m(); //        -
        1.noargs_m(); //        noargs_m (self: &1) { noargs_m ::closure{<coords>}(y: 2) {} -> 2 }
        1.skip_m(); //          skip  _m (..      ) { skip  _m ::closure{..      }(..  ) {} -> 2 }
        1.log_m(); //           noargs_m (self: &1) { noargs_m ::closure{<coords>}(y: 2) {} -> 2 }

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

                "noargs_m(self: &1) {\n",
                "  noargs_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // noargs_m().\n",
                "skip_m(..) {\n",
                "  skip_m::closure{..}(..) {} -> 2\n",
                "} // skip_m().\n",
                "log_m(self: &1) {\n",
                "  log_m::closure{0,0:0,0}(y: 2) {} -> 2\n",
                "} // log_m().\n",
            )
        );
        log.borrow_mut().clear();
    }

    {
        #[loggable] // NoArgs
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
        i8::absent_af(1); //    Tr::absent_af(_p: 1) { Tr::absent_af::closure{<coords>}(x: 0) {} -> 0 }
        i8::noargs_af(1); //    Tr::noargs_af(_p: 1) { Tr::noargs_af::closure{<coords>}(x: 0) {} -> 0 }
        i8::skip_af(1); //      Tr::skip  _af(..   ) { Tr::skip  _af::closure{..      }(..  ) {} -> 0 }
        i8::log_af(1); //       Tr::log   _af(_p: 1) { Tr::log   _af::closure{<coords>}(x: 0) {} -> 0 }

        1.absent_m(); //        Tr::absent_m (self: &1) { Tr::absent_m ::closure{<coords>}(y: 2) {} -> 2 }
        1.noargs_m(); //        Tr::noargs_m (self: &1) { Tr::noargs_m ::closure{<coords>}(y: 2) {} -> 2 }
        1.skip_m(); //          Tr::skip  _m (..      ) { Tr::skip  _m ::closure{..      }(..  ) {} -> 2 }
        1.log_m(); //           Tr::log   _m (self: &1) { Tr::log   _m ::closure{<coords>}(y: 2) {} -> 2 }

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
        i8::absent_af(1); //    Tr::absent_af(..   ) { Tr::absent_af::closure{..      }(..  ) {} -> 0 }
        i8::noargs_af(1); //    Tr::noargs_af(..   ) { Tr::noargs_af::closure{..      }(..  ) {} -> 0 }
        i8::skip_af(1); //      Tr::skip  _af(..   ) { Tr::skip  _af::closure{..      }(..  ) {} -> 0 }
        i8::log_af(1); //       Tr::log   _af(_p: 1) { Tr::log   _af::closure{<coords>}(x: 0) {} -> 0 }

        1.absent_m(); //        Tr::absent_m (..      ) { Tr::absent_m ::closure{..      }(..  ) {} -> 2 }
        1.noargs_m(); //        Tr::noargs_m (..      ) { Tr::noargs_m ::closure{..      }(..  ) {} -> 2 }
        1.skip_m(); //          Tr::skip  _m (..      ) { Tr::skip  _m ::closure{..      }(..  ) {} -> 2 }
        1.log_m(); //           Tr::log   _m (self: &1) { Tr::log   _m ::closure{<coords>}(y: 2) {} -> 2 }

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
        i8::absent_af(1); //    Tr::absent_af(_p: 1) { Tr::absent_af::closure{<coords>}(x: 0) {} -> 0 }
        i8::noargs_af(1); //    Tr::noargs_af(_p: 1) { Tr::noargs_af::closure{<coords>}(x: 0) {} -> 0 }
        i8::skip_af(1); //      Tr::skip  _af(..   ) { Tr::skip  _af::closure{..      }(..  ) {} -> 0 }
        i8::log_af(1); //       Tr::log   _af(_p: 1) { Tr::log   _af::closure{<coords>}(x: 0) {} -> 0 }

        1.absent_m(); //        Tr::absent_m (self: &1) { Tr::absent_m ::closure{<coords>}(y: 2) {} -> 2 }
        1.noargs_m(); //        Tr::noargs_m (self: &1) { Tr::noargs_m ::closure{<coords>}(y: 2) {} -> 2 }
        1.skip_m(); //          Tr::skip  _m (..      ) { Tr::skip  _m ::closure{..      }(..  ) {} -> 2 }
        1.log_m(); //           Tr::log   _m (self: &1) { Tr::log   _m ::closure{<coords>}(y: 2) {} -> 2 }

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

    

/*
    {
        // Absent
        mod m {
            use super::*;

            pub fn absent(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable]
            pub fn noargs(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(skip_params, skip_closure_coords)]
            pub fn skip(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(log_params, log_closure_coords)]
            pub fn log(_p: u8) {
                Some(0).map(|x| x);
            }
        }

        m::absent(1); //   -
        m::noargs(1); //   noargs(_p: 1) { noargs::closure{<coords>}(x: 0) -> 0 {}}
        m::skip(1); //     skip  (..   ) { skip  ::closure{..      }(..  ) -> 0 {}}
        m::log(1); //      log   (_p: 1) { log   ::closure{<coords>}(x: 0) -> 0 {}}

        flush_log();
        let log_contents = zero_out_closure_coords(log.clone());
        assert_eq!(
            log_contents,
            concat!(
                "noargs(_p: 1) {\n",
                "  noargs::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // noargs().\n",
                "skip(..) {\n",
                "  skip::closure{..}(..) {} -> 0\n",
                "} // skip().\n",
                "log(_p: 1) {\n",
                "  log::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // log().\n",
            )
        );
        log.borrow_mut().clear();
    }

    {
        #[loggable] // NoArgs
        mod m {
            use super::*;

            pub fn absent(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable]
            pub fn noargs(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(skip_params, skip_closure_coords)]
            pub fn skip(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(log_params, log_closure_coords)]
            pub fn log(_p: u8) {
                Some(0).map(|x| x);
            }
        }

        m::absent(1); //   m::absent(_p: 1) { m::absent::closure{<coords>}(x: 0) -> 0 {}}
        m::noargs(1); //   m::noargs(_p: 1) { m::noargs::closure{<coords>}(x: 0) -> 0 {}}
        m::skip(1); //     m::skip  (..   ) { m::skip  ::closure{..      }(..  ) -> 0 {}}
        m::log(1); //      m::log   (_p: 1) { m::log   ::closure{<coords>}(x: 0) -> 0 {}}

        flush_log();
        let log_contents = zero_out_closure_coords(log.clone());

        assert_eq!(
            log_contents,
            concat!(
                "m::absent(_p: 1) {\n",
                "  m::absent::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // m::absent().\n",
                "m::noargs(_p: 1) {\n",
                "  m::noargs::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // m::noargs().\n",
                "m::skip(..) {\n",
                "  m::skip::closure{..}(..) {} -> 0\n",
                "} // m::skip().\n",
                "m::log(_p: 1) {\n",
                "  m::log::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // m::log().\n",
            )
        );
        log.borrow_mut().clear();
    }

    {
        #[loggable(skip_params, skip_closure_coords)] // skip_*
        mod m {
            use super::*;

            pub fn absent(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable]
            pub fn noargs(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(skip_params, skip_closure_coords)]
            pub fn skip(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(log_params, log_closure_coords)]
            pub fn log(_p: u8) {
                Some(0).map(|x| x);
            }
        }

        m::absent(1); //   m::absent(..   ) { m::absent::closure{..      }(..  ) -> 0 {}}
        m::noargs(1); //   m::noargs(..   ) { m::noargs::closure{..      }(..  ) -> 0 {}}
        m::skip(1); //     m::skip  (..   ) { m::skip  ::closure{..      }(..  ) -> 0 {}}
        m::log(1); //      m::log   (_p: 1) { m::log   ::closure{<coords>}(x: 0) -> 0 {}}

        flush_log();
        let log_contents = zero_out_closure_coords(log.clone());

        assert_eq!(
            log_contents,
            concat!(
                "m::absent(..) {\n",
                "  m::absent::closure{..}(..) {} -> 0\n",
                "} // m::absent().\n",
                "m::noargs(..) {\n",
                "  m::noargs::closure{..}(..) {} -> 0\n",
                "} // m::noargs().\n",
                "m::skip(..) {\n",
                "  m::skip::closure{..}(..) {} -> 0\n",
                "} // m::skip().\n",
                "m::log(_p: 1) {\n",
                "  m::log::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // m::log().\n",
            )
        );
        log.borrow_mut().clear();
    }

    {
        #[loggable(log_params, log_closure_coords)] // log_*
        mod m {
            use super::*;

            pub fn absent(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable]
            pub fn noargs(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(skip_params, skip_closure_coords)]
            pub fn skip(_p: u8) {
                Some(0).map(|x| x);
            }
            #[loggable(log_params, log_closure_coords)]
            pub fn log(_p: u8) {
                Some(0).map(|x| x);
            }
        }

        m::absent(1); //   m::absent(_p: 1) { m::absent::closure{<coords>}(x: 0) -> 0 {}}
        m::noargs(1); //   m::noargs(_p: 1) { m::noargs::closure{<coords>}(x: 0) -> 0 {}}
        m::skip(1); //     m::skip  (..   ) { m::skip  ::closure{..      }(..  ) -> 0 {}}
        m::log(1); //      m::log   (_p: 1) { m::log   ::closure{<coords>}(x: 0) -> 0 {}}

        flush_log();
        let log_contents = zero_out_closure_coords(log.clone());

        assert_eq!(
            log_contents,
            concat!(
                "m::absent(_p: 1) {\n",
                "  m::absent::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // m::absent().\n",
                "m::noargs(_p: 1) {\n",
                "  m::noargs::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // m::noargs().\n",
                "m::skip(..) {\n",
                "  m::skip::closure{..}(..) {} -> 0\n",
                "} // m::skip().\n",
                "m::log(_p: 1) {\n",
                "  m::log::closure{0,0:0,0}(x: 0) {} -> 0\n",
                "} // m::log().\n",
            )
        );
        log.borrow_mut().clear();
    }
*/
}

