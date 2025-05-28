// use fcl_proc_macros::{function_logger, loggable};
// #[loggable]
// fn f() {}

#![feature(c_variadic)]
#![feature(stmt_expr_attributes)] // Loggable closures.
#![feature(proc_macro_hygiene)] // Loggable closures.

use std::thread;
use std::time::Duration;

use fcl::call_log_infra::CALL_LOG_INFRA;
use fcl::{FunctionLogger, ClosureLogger, closure_logger};
use fcl_proc_macros::{function_logger, loggable};

// TODO:
// Macro
//  Simple
//  proc_macro_attr
// Testing.

#[loggable]
// #[rustfmt::skip]
fn f() {
    thread::sleep(Duration::from_millis(1));
}

#[loggable]
fn g() {
    f();
}

#[loggable]
fn _h() {
    let _x = 1 + 2;
    unsafe { _i::<i32, bool>(1, 2.0, true) };
}

#[loggable]
// #[somemyattr]
// #[anotherattr]
pub(crate) unsafe extern "C" fn _i<T, U>(_x: i32, _y: f32, _z: bool, ...) -> f64 {
    #[loggable]
    fn j(_x: u32, _y: u32) -> bool {
        // Local function.
        true
    }

    let _ = j(0, 1);
    -1.0
}

#[loggable]
fn calls() {

    // // If logging is enabled, create the call logger.
    // let mut _l = None;
    // CALL_LOG_INFRA.with(|infra| {
    //     if infra.borrow_mut().is_on() {
    //         _l = Some(FunctionLogger::new("main"))
    //     }
    // });
    // // let _l = FunctionLogger::new("main");

    for _ in 0..10 {
        f();
    }
    g();
    f();
    for _ in 0..30 {
        g();
    }

    _h();
    // let _s = MyStruct::new();
    // let _ = _s.method();

    let _b = Some(true).map(
        #[loggable]
        // #[rustfmt::skip]
        move |b| -> bool {
            /*println!("Lambda"); */
            Some(b)
                .map(
                    // main()::closure()::closure() {}
                    #[loggable]
                    |v| !v
                )
                .unwrap()
        }
    );
    // assert_eq!(Some(false), _b);

    {
        struct MyStruct;
        impl MyStruct {
            #[loggable(MyStruct::new)]
            fn new() -> Self {
                Self
            }
            #[loggable(MyStruct::method)]
            fn method<T, U>(&self) -> bool {
                thread::sleep(Duration::from_millis(1));
                false
            }
        }
        let ms = MyStruct::new(); // new() {}
        ms.method::<bool, i32>(); // method() {}
    }
    {
        #[loggable]
        pub fn gen_func<T, U>() {}  // TODO: No generics logged.
        gen_func::<bool, i32>();
    }

    {
        trait MyTrait {
            #[loggable(MyTrait::trait_method)]
            fn trait_method(&self) { // Virtual function.
                // Default implementation.
            }
        }
        struct MyStruct;
        impl MyTrait for MyStruct {
            #[loggable(MyStruct::trait_method)]
            fn trait_method(&self) { // Virtual function override.
                // Override of the default.
            }
        }
        struct MyStrNonOverride;
        impl MyTrait for MyStrNonOverride {
            // Uses the default implementation.
        }
        MyStruct.trait_method();         // Calls MyStruct::trait_method() override.
        MyStrNonOverride.trait_method(); // Calls MyTrait ::trait_method() default.
    }
    {
        trait MyPureTrait {
            // #[loggable]      // Error: expected `|`
            fn pure_method(&self); // No defualt behavior. Pure virtual function with no def-n.
        }
        struct MyStruct;
        impl MyPureTrait for MyStruct {
            #[loggable(<MyStruct as MyPureTrait>::pure_method)] // TODO: Wrong result: `MyPureTrait :: pure_method() {}`. Expected `<MyStruct as MyPureTrait> :: pure_method() {}`.
            fn pure_method(&self) {}
        }
        MyStruct.pure_method();
    }
}

// #[loggable]
fn thread_func() {
    CALL_LOG_INFRA.with(|infra| infra.borrow_mut().set_is_on(true)); // Turn logging on.
    CALL_LOG_INFRA.with(|infra| infra.borrow_mut().set_thread_indent(&"                                  "));


    // // If logging is enabled, create the call logger.
    // let mut _l = None;
    // CALL_LOG_INFRA.with(|infra| {
    //     if infra.borrow_mut().is_on() {
    //         _l = Some(FunctionLogger::new("main"))
    //     }
    // });
    // // let _l = FunctionLogger::new("main");

    // println!("thread_func() starts");

    #[loggable]
    fn f2() {
        // use fcl::call_log_infra::CALL_LOG_INFRA;
        // let mut _logger = None;
        // CALL_LOG_INFRA.with(|infra| {
        //     if infra.borrow_mut().is_on() {
        //         _logger = Some(FunctionLogger::new("f2"))
        //     }
        // });
    }

    for _ in 0..10 {
        f2();
    }
    // println!("thread_func() called f2()");

    g();
    f();
    for _ in 0..30 {
        g();
    }

    _h();
    // let _s = MyStruct::new();
    // let _ = _s.method();

    thread::sleep(Duration::from_millis(1));
    let _b = Some(true).map(
        #[loggable]
        // #[rustfmt::skip]
        move |b| -> bool {
            /*println!("Lambda"); */
            Some(b)
                .map(
                    // main()::closure()::closure() {}
                    #[loggable]
                    |v| !v
                )
                .unwrap()
        }
    );
    // assert_eq!(Some(false), _b);

    {
        struct MyStruct;
        impl MyStruct {
            #[loggable(MyStruct::new)]
            fn new() -> Self {
                Self
            }
            #[loggable(MyStruct::method)]
            fn method<T, U>(&self) -> bool {
                thread::sleep(Duration::from_millis(1));
                false
            }
        }
        let ms = MyStruct::new(); // new() {}
        ms.method::<bool, i32>(); // method() {}
    }
    {
        #[loggable]
        pub fn gen_func<T, U>() {}  // TODO: No generics logged.
        gen_func::<bool, i32>();
    }

    {
        trait MyTrait {
            #[loggable(MyTrait::trait_method)]
            fn trait_method(&self) { // Virtual function.
                // Default implementation.
            }
        }
        struct MyStruct;
        impl MyTrait for MyStruct {
            #[loggable(MyStruct::trait_method)]
            fn trait_method(&self) { // Virtual function override.
                // Override of the default.
            }
        }
        struct MyStrNonOverride;
        impl MyTrait for MyStrNonOverride {
            // Uses the default implementation.
        }
        MyStruct.trait_method();         // Calls MyStruct::trait_method() override.
        MyStrNonOverride.trait_method(); // Calls MyTrait ::trait_method() default.
    }
    {
        trait MyPureTrait {
            // #[loggable]      // Error: expected `|`
            fn pure_method(&self); // No defualt behavior. Pure virtual function with no def-n.
        }
        struct MyStruct;
        impl MyPureTrait for MyStruct {
            #[loggable(<MyStruct as MyPureTrait>::pure_method)] // TODO: Wrong result: `MyPureTrait :: pure_method() {}`. Expected `<MyStruct as MyPureTrait> :: pure_method() {}`.
            fn pure_method(&self) {}
        }
        MyStruct.pure_method();
    }
    // println!("thread_func() ends");
}

fn main() {
    // TODO: -> macro, or simplify otherwise.
    // set_is_on(true);
    CALL_LOG_INFRA.with(|infra| infra.borrow_mut().set_is_on(true)); // Turn logging on.

    let thread_handle = thread::spawn(thread_func);
    calls();
    let _ = thread_handle.join();
}

// CodeLikeDecorator:
// main() {                              
//   f() {}
//   // f() repeats 9 time(s).
//   g() {
//     f() {}
//   } // g().
//   f() {}
//   g() {
//     f() {}
//   } // g().
//   // g() repeats 29 time(s).
//   _h() {
//     _i < T, U >() {
//       j() {}
//     } // _i < T, U >().
//   } // _h().
//   closure{76,9:87,9}() {
//     closure{83,21:84,26}() {}
//   } // closure{76,9:87,9}().
//   MyStruct :: new() {}
//   MyStruct :: method < T, U >() {}
//   gen_func < T, U >() {}
//   MyStruct :: trait_method() {}
//   MyTrait :: trait_method() {}
//   MyPureTrait :: pure_method() {}
// } // main().

// TreeLikeDecorator:
// +-main
// | +-f
// |   f repeats 9 time(s).
// | +-g
// | | +-f
// | +-f
// | +-g
// | | +-f
// |   g repeats 29 time(s).
// | +-_h
// | | +-_i < T, U >
// | | | +-j
// | +-closure{76,9:87,9}
// | | +-closure{83,21:84,26}
// | +-MyStruct :: new
// | +-MyStruct :: method < T, U >
// | +-gen_func < T, U >
// | +-MyStruct :: trait_method
// | +-MyTrait :: trait_method
// | +-MyPureTrait :: pure_method

// Threads:
// calls() {
//   f() {                                  f2() {}
//                                   // f2() repeats 9 time(s).
//                                   g() {
//                                     f() {}
// }
//                                   } // g().
//                                   f() {}
//                                   g() {
//                                     f() {}
//                                   } // g().
//   // f() repeats 9 time(s).
//   g() {
//     f() {}
//   } // g().
//   f() {}
//   g() {
//     f() {}
//   } // g().
//                                   // g() repeats 29 time(s).
//                                   _h() {
//                                     _i < T, U >() {
//                                       j() {}
//                                     } // _i < T, U >().
//                                   } // _h().
//                                   closure{196,9:207,9}() {
//                                     closure{203,21:204,26}() {}
//                                   } // closure{196,9:207,9}().
//                                   MyStruct :: new() {}
//                                   MyStruct :: method < T, U >() {}
//                                   gen_func < T, U >() {}
//                                   MyStruct :: trait_method() {}
//                                   MyTrait :: trait_method() {}
//                                   MyPureTrait :: pure_method() {}
//   // g() repeats 29 time(s).
//   _h() {
//     _i < T, U >() {
//       j() {}
//     } // _i < T, U >().
//   } // _h().
//   closure{79,9:90,9}() {
//     closure{86,21:87,26}() {}
//   } // closure{79,9:90,9}().
//   MyStruct :: new() {}
//   MyStruct :: method < T, U >() {}
//   gen_func < T, U >() {}
//   MyStruct :: trait_method() {}
//   MyTrait :: trait_method() {}
//   MyPureTrait :: pure_method() {}
// } // calls().