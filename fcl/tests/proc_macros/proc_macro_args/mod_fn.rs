use fcl_proc_macros::loggable;
use crate::common::*;

//
// #[loggable]         | TestCases
// Attribute           | (mod, fn)
// Values              | G: Written by ChatGPT/Codex.   +: Written mannually.       |   Notes
//---------------------|------------------------------------------------------------|-------------
// Absent              |++|+ |+ |+ |   | +|  |  |  |   | +|  |  |  |    | G|  |  |  |   // ``
// NoArgs              |  | +|  |  |   |+ |++|+ |+ |   |  | +|  |  |    |  | G|  |  |   // `#[loggable]`
// skip_*              |  |  | +|  |   |  |  | +|  |   |+ |+ |++|+ |    |  |  | G|  |   // `#[loggable(skip_params, skip_closure_coords)]`
// log_*               |  |  |  | +|   |  |  |  | +|   |  |  |  | +|    |G |G |G |GG|   // `#[loggable(log_params, log_closure_coords)]`

// // Absent, NoArgs, {skip,log}_{params,closure_coords}
// mod m {
//     // Absent, NoArgs, {skip,log}_{params,closure_coords}
//     fn f() {
//         Some(0).map(|x| x);
//     }
// }

#[test]
fn mod_fn() {
    let log = substitute_log_writer();

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

}
