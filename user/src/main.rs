#![feature(c_variadic)]
#![feature(stmt_expr_attributes)] // Loggable closures.
#![feature(proc_macro_hygiene)] // Loggable closures.
// #![feature(min_specialization)]

use std::thread;
use std::time::Duration;

use fcl::call_log_infra::THREAD_LOGGER;
// use fcl::{ClosureLogger, closure_logger};
use fcl_proc_macros::{loggable, non_loggable};

fn main() {
    // TODO: -> macro, or simplify otherwise.
    // set_is_on(true);
    THREAD_LOGGER.with(|logger| logger.borrow_mut().set_logging_is_on(true)); // Turn logging on.

    let result = thread::Builder::new().name("T1".into()).spawn(thread_func); // T1 thread.
    calls(); // main() thread.
    let _ = result.unwrap().join();
}

#[loggable]
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
    //     if infra.borrow_mut().logging_is_on() {
    //         _l = Some(FunctionLogger::new("main"))
    //     }
    // });
    // // let _l = FunctionLogger::new("main");

    for _ in 0..10 {
        f();
    }
    g();
    f();
    for _ in 0..7 {
        g();
    }

    _h();
    // let _s = MyStruct::new();
    // let _ = _s.method();

    let _b = Some(true).map(
        // #[loggable]
        // #[rustfmt::skip]
        move |b| -> bool {
            /*println!("Lambda"); */
            Some(b)
                .map(
                    // main()::closure()::closure() {}
                    // #[loggable]
                    |v| !v,
                )
                .unwrap()
        },
    );
    // assert_eq!(Some(false), _b);

    {
        struct MyStruct;
        // #[loggable]
        impl MyStruct {
            // #[loggable(prefix=)]
            #[loggable(prefix=MyStruct)]
            fn new() -> Self {
                Self
            }
            #[loggable(prefix=MyStruct)]
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
        pub fn gen_func<T, U>() {}
        gen_func::<bool, i32>();
    }

    {
        trait MyTrait {
            #[loggable(prefix=MyTrait)]
            fn trait_method(&self) { // Virtual function.
                // Default implementation.
            }
        }
        struct MyStruct;
        impl MyTrait for MyStruct {
            #[loggable(prefix=MyStruct)]
            fn trait_method(&self) { // Virtual function override.
                // Override of the default.
            }
        }
        struct MyStrNonOverride;
        impl MyTrait for MyStrNonOverride {
            // Uses the default implementation.
        }
        MyStruct.trait_method(); // Calls MyStruct::trait_method() override.
        MyStrNonOverride.trait_method(); // Calls MyTrait ::trait_method() default.
    }
    {
        trait MyPureTrait {
            // #[loggable]      // Error: expected `|`
            fn pure_method(&self); // No defualt behavior. Pure virtual function with no def-n.
        }
        struct MyStruct;
        impl MyPureTrait for MyStruct {
            // #[loggable(prefix=MyStruct)]
            #[loggable(prefix=<MyStruct as MyPureTrait>)]
            // TODO: Unexpected result: `MyPureTrait :: pure_method() {}`.
            // Expected `<MyStruct as MyPureTrait> :: pure_method() {}`.
            // Consider `#[loggable((MyStruct as MyPureTrait)::pure_method)]`. Doesn't work. Results in `pure_method`.
            // Consider `#[loggable(MyStruct::as::MyPureTrait::pure_method)]`. Doesn't work. Results in `pure_method`.
            fn pure_method(&self) {}
        }
        MyStruct.pure_method();
    }
    #[loggable]
    {
        fn f(i: u8) {
            g(i);
        }
        fn g(i: u8) {
            if i == 8 {
                // println!("stdout output");
                // panic!("main(): Testing the panic");
                eprintln!("Sample stderr output in main()");
                // panic!("main(): Panicking voluntarily")
            }
        }

        for i in 0..10 {
            f(i);
        }
        g(20);
    }
    {
        // struct LoopbodyLogger;
        // impl LoopbodyLogger {
        //     fn set_retval(&mut self, ) {

        //     }
        // }
        #[loggable]
        fn f() {
            let loop_retval = {
                let mut i = 0;
                {
                    let _ret_val = loop {
                    // while i < 3 {
                    // for i in 0..3 {

                        // Log loopbody start

                        // {
                            g(i);
                            h(i);
                            if i == 2 {
                                break 5;
                            }
                            i += 1;
                        // };

                        // Log loopbody end
                    };
                    _ret_val
                    // log_exprloop_end(_ret_val)
                }
            };
            println!("stdout: f()::loop -> {}", loop_retval);
        }
        fn g(_i: i32) {
            ifunc();
        }
        fn h(_i: i32) {}
        fn ifunc() {}

        f();
    }
    #[loggable]
    {
        fn z() {}
        fn x() {
            z();
            z();
        }
        fn y() {}
        for _ in 0..2 {
            x();
            y();
        }
        for _ in 0..3 {
            x();
            y();
        }
        z();
    }
}

#[loggable]
fn thread_func() {
    THREAD_LOGGER.with(|logger| logger.borrow_mut().set_logging_is_on(true)); // Turn logging on.
    // CALL_LOG_INFRA.with(|infra| infra.borrow_mut().set_is_on(true)); // Turn logging on.

    THREAD_LOGGER.with(|logger| {
        logger
            .borrow_mut()
            .set_thread_indent(&"                                  ")
    });
    // CALL_LOG_INFRA.with(|infra| {
    //     infra
    //         .borrow_mut()
    //         .set_thread_indent(&"                                  ")
    // });

    // // If logging is enabled, create the call logger.
    // let mut _l = None;
    // CALL_LOG_INFRA.with(|infra| {
    //     if infra.borrow_mut().logging_is_on() {
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
        //     if infra.borrow_mut().logging_is_on() {
        //         _logger = Some(FunctionLogger::new("f2"))
        //     }
        // });
    }

    for _ in 0..10 {
        f2();
    }
    // println!("thread_func() called f2()");

    g();
    ff();
    for _ in 0..5 {
        g();
    }

    _h();
    // let _s = MyStruct::new();
    // let _ = _s.method();

    thread::sleep(Duration::from_millis(1));
    let _b = Some(true).map(
        #[loggable]
        move |b| -> bool {
            Some(b)
                .map(
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
            #[loggable(prefix = MyStruct)]
            fn new() -> Self {
                Self
            }
            #[loggable(prefix = MyStruct)]
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
        pub fn gen_func<T, U>() {}
        gen_func::<bool, i32>();
    }

    {
        trait MyTrait {
            #[loggable(prefix = MyTrait)]
            fn trait_method(&self) { // Virtual function.
                // Default implementation.
            }
        }
        struct MyStruct;
        impl MyTrait for MyStruct {
            #[loggable(prefix = MyStruct)]
            fn trait_method(&self) { // Virtual function override.
                // Override of the default.
            }
        }
        struct MyStrNonOverride;
        impl MyTrait for MyStrNonOverride {
            // Uses the default implementation.
        }
        MyStruct.trait_method(); // Calls MyStruct::trait_method() override.
        MyStrNonOverride.trait_method(); // Calls MyTrait ::trait_method() default.
    }
    {
        trait MyPureTrait {
            // #[loggable]      // Error: expected `|`
            fn pure_method(&self); // No defualt behavior. Pure virtual function with no def-n.
        }
        #[derive(std::fmt::Debug)] 
        struct MyStruct;
        #[loggable]
        impl MyPureTrait for MyStruct {
            #[loggable(prefix = <MyStruct as MyPureTrait>)] // OK.
            fn pure_method(&self) {}
        }
        MyStruct.pure_method();
    }
    {
        struct LoggableStruct(bool);
        #[loggable]
        impl LoggableStruct {
            // type MyType = u8;
            fn assoc_func() {}
            fn assoc_method(&self) {}
            fn assoc_funcb<T>() {}
        }
        #[loggable]
        impl Iterator for LoggableStruct {
            type Item = u8; // Is just forwarded from input to output (despite of `#[loggable(prefix=LoggableStruct)] type Item = ...`).
            fn next(&mut self) -> Option<Self::Item> {
                if self.0 {
                    self.0 = false;
                    // Some(1)
                    return Some(1);
                } else {
                    None
                }
            }
        }
        LoggableStruct::assoc_func();
        LoggableStruct(false).assoc_method();
        LoggableStruct::assoc_funcb::<bool>();

        for _it in LoggableStruct(true) {
            // Call `next()`.
        }
        // let iter = (&LoggableStruct).iter();
    }

    // {
    //     trait ToStr<T, M> {
    //         fn to_string(value: T) -> String;
    //     }
    //     struct S;
    //     struct P;
    //     impl<T> ToStr<T, P> for S 
    //     where T: ToString
    //     {
    //         fn to_string(value: T) -> String {
    //             value.to_string()
    //         }
    //     }
    //     struct NP;
    //     impl<T> ToStr<T, NP> for S 
    //     // where T: !ToString
    //     {
    //         fn to_string(_value: T) -> String {
    //             String::new()
    //             // value.to_string()
    //         }
    //     }
    //     let s: String = S::to_string(true);
    // }
    // {
    //     // fn to_string<Marker, T>
    //     struct S<T> {
    //         pd: PhantomData<T>
    //     }
    //     impl<T> S<T> {
    //         fn debug_str(_val: &T) -> String {
    //             String::new()
    //         }
    //     }
    //     impl<T: std::fmt::Debug> S<T> {
    //         fn debug_str(_val: &T) -> String {
    //             format!("_val: {:?}, ", _val)
    //         }
    //     }
    // }

    {
        #[loggable]
        fn f<F /*: FnOnce()*/>(fun: F, _b: bool)
        where
            F: FnOnce(),
        {
            // let params = String::new() + &format!("_b: {:?}, ", _b) + &format!("_b: {:?}, ", _b);
            // println!("{}", params);
            // println!("fun: {:?}, _b: {:?}", fun.to_string(), _b);
            fun()
        }
        #[loggable]
        f(
            || (),
            true,
        );
    }
    {
        struct ST;
        #[loggable]
        impl ST {
            fn f() {}
            fn g(&self) {}
            fn h(&mut self) {}
            #[non_loggable]
            fn i(self) {}
        }
        let mut st = ST;
        ST::f();    // ST :: f() {}
        st.g(); // ST :: g() {}
        st.h(); // ST :: h() {}
        st.i(); // - (#[non_loggable])
        
    }
    {
        #[loggable]
        fn f_with_f<F /*: FnOnce()*/>(fun: F, _b: bool)
        where
            F: FnOnce(),
        {
            // let param_format_str = format!("fun: {}, _b: {}, ", fun.maybe_print(), _b.maybe_print());
            // let params = "fun: ?, _b: true";
            // // let params = String::new() + &format!("_b: {:?}, ", _b) + &format!("_b: {:?}, ", _b);
            // // println!("{}", params);
            // // println!("fun: {:?}, _b: {:?}", fun.to_string(), _b);
            // println!("fun: {}", fun.maybe_print());
            // println!("_b: {}", _b.maybe_print());
            fun()
        }
        f_with_f(
            #[loggable]
            || (),
            true,
        );

    }

    #[loggable]
    fn fs(_s: String) {}
    fs(String::from("Abc"));

    // println!("thread_func() ends");

    #[loggable]
    {
        fn f(i: u8) {
            g(i);
        }
        #[loggable]
        fn g(i: u8) {
            // fn _println() {}
            // _println();
            if i == 8 {
                // println!("stdout output");
                // std::io::_eprint("0. stdout: hmm. ");
                // {
                //     std::io::_eprint(std::format_args_nl!("1. stderr: T1 stderr output"));
                // };
                // {
                //     std::io::_eprint(println!("1. stderr: T1 stderr output"));
                // };

                println!("0. stdout: hmm. ");
                eprintln!["1. stderr: T1 stderr output"];
                println!("2. stdout: hmm...");
                eprintln!("3. stderr: Oh");

                // panic!("T1: Testing the panic");

                // std::io::_print(std::format_args_nl!("0. stdout: hmm. "));
                // std::io::_eprint(std::format_args_nl!("1. stderr: T1 stderr output"));

                // std::io::_print(std::format_args!("hmm"));
                // panic!("Panicking volunterely")
            }
        }

        for i in 0..10 {
            f(i);
        }
        g(20);
    }
    #[loggable]
    fn ff() {
        thread::sleep(Duration::from_millis(1)); 
    }

    #[loggable]
    fn gg(i: i32) {
        ff();
    }

    // for i in 0..150 {
    //     gg(i);
    // }
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
//   f() {
//                                   f2() {}
//                                   // f2() repeats 9 time(s).
//                                   g() {
//                                     f() {
//   } // f().
//   f() {
//                                     } // f().
//                                   } // g().
//                                   f() {
//   } // f().
//   f() {
//                                   } // f().
//                                   g() {
//                                     f() {
//   } // f().
//   f() {
//                                     } // f().
//                                   } // g().
//                                   g() {
//   } // f().
//   f() {
//                                     f() {
//   } // f().
//   f() {
//                                     } // f().
//                                   } // g().
//                                   g() {
//                                     f() {
//   } // f().
//                                     } // f().
//                                   } // g().
//   f() {
//                                   g() {
//                                     f() {
//   } // f().
//                                     } // f().
//                                   } // g().
//   f() {
//                                   g() {
//                                     f() {
//   } // f().
//                                     } // f().
//                                   } // g().
//   f() {
//                                   _h() {
//                                     _i < T, U >() {
//                                       j() {}
//                                     } // _i < T, U >().
//                                   } // _h().
//   } // f().
//   f() {
//                                   closure{206,9:217,9}() {
//                                     closure{213,21:214,26}() {}
//                                   } // closure{206,9:217,9}().
//                                   MyStruct :: new() {
//   } // f().
//                                   } // MyStruct :: new().
//   g() {
//                                   MyStruct :: method < T, U >() {
//     f() {
//                                   } // MyStruct :: method < T, U >().
//                                   gen_func < T, U >() {}
//     } // f().
//   } // g().
//                                   MyStruct :: trait_method() {}
//                                   MyTrait :: trait_method() {}
//   f() {
//                                   MyPureTrait :: pure_method() {}
//   } // f().
//   g() {
//     f() {}
//   } // g().
//   // g() repeats 6 time(s).
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

// +-calls
// | +-f
//                                   +-f2
//                                     f2 repeats 9 time(s).
//                                   +-g
//                                   | +-f
// | +-f
//                                   +-f
// | +-f
//                                   +-g
//                                   | +-f
// | +-f
//                                   +-g
// | +-f
//                                   | +-f
// | +-f
//                                   +-g
//                                   | +-f
// | +-f
//                                   +-g
//                                   | +-f
// | +-f
//                                   +-g
//                                   | +-f
// | +-f
//                                   +-_h
//                                   | +-_i < T, U >
//                                   | | +-j
// | +-f
//                                   +-closure{206,9:217,9}
//                                   | +-closure{213,21:214,26}
// | +-g
// | | +-f
//                                   +-MyStruct :: new
//                                   +-MyStruct :: method < T, U >
// | +-f
//                                   +-gen_func < T, U >
//                                   +-MyStruct :: trait_method
//                                   +-MyTrait :: trait_method
//                                   +-MyPureTrait :: pure_method
// | +-g
// | | +-f
// |   g repeats 6 time(s).
// | +-_h
// | | +-_i < T, U >
// | | | +-j
// | +-closure{79,9:90,9}
// | | +-closure{86,21:87,26}
// | +-MyStruct :: new
// | +-MyStruct :: method < T, U >
// | +-gen_func < T, U >
// | +-MyStruct :: trait_method
// | +-MyTrait :: trait_method
// | +-MyPureTrait :: pure_method
