// fn f() {
//     let mut generic_func_name = String::with_capacity(64);
//     generic_func_name.push_str("f");
//     if !true {
//         generic_func_name.push_str("<");
//         let generic_arg_names_vec: Vec<&'static str> = alloc::vec::Vec::new();
//         for (idx, generic_arg_name) in generic_arg_names_vec.into_iter().enumerate() {
//             if idx != 0 {
//                 generic_func_name.push_str(",");
//             }
//             generic_func_name.push_str(generic_arg_name);
//         }
//         generic_func_name.push_str(">");
//     }
//     use fcl::{CallLogger, MaybePrint};
//     let param_val_str = None;
//     let mut callee_logger = fcl::FunctionLogger::new(&generic_func_name, param_val_str);
//     let ret_val = (move || {
//         let _c = Some(5).map(|value| {
//             use fcl::{CallLogger, MaybePrint};
//             // let mut optional_callee_logger = None;
//             let ret_val = fcl::call_log_infra::instances::THREAD_LOGGER.with(|logger| {
//                 if !logger.borrow().logging_is_on() {
//                     return (move || true)();
//                 }

//                 // Else (logging is on):
//                 let param_val_str = Some(format!(
//                     "value: {}", 
//                     value.maybe_print(),
//                 ));
//                 let mut callee_logger = fcl::FunctionLogger::new(
//                     "f()::closure{1,1:1,0}",
//                     param_val_str,
//                 );

//                 let ret_val = (move || true)();

//                 let ret_val_str = format!("{}", ret_val.maybe_print());
//                 callee_logger.set_ret_val(ret_val_str);

//                 ret_val
//             });
//             ret_val
//         });
//     })();
//     if false {
//         let ret_val_str = alloc::__export::must_use({
//             let res =
//                 alloc::fmt::format(alloc::__export::format_args!("{}", ret_val.maybe_print()));
//             res
//         });
//         callee_logger.set_ret_val(ret_val_str);
//     }
//     ret_val
// }

// #[fcl_proc_macros::loggable] // The procedural macro that does the instrumetation.
// fn f() { // The user's function definition.
//     let _c = Some(5).map(
//         |value| true    // The user's closure definition.
//     ); 
// }


#[fcl_proc_macros::loggable]
pub fn main() {
    // fcl::_single_threaded_otimization!();
    // fcl::call_log_infra::THREAD_LOGGER.with(|logger| logger.borrow_mut().set_logging_is_on(true)); // Turn logging on.
    // fcl::set_thread_indent!(String::from("                "));
    let _a = Some(0);

    use root::*;
    f();
    g();

    {
        use crate::*;
        crate::S.d(); // Expected: S::d()
        // crate::<S as Tr>.d();  // Expected: S::d()
        crate::S2.d(); // Expected: Tr::d()
        crate::S2.e();

        let s = S;
        let s2 = S2;
        let a: Vec<&dyn Tr> = vec![&s, &s2];
        a[0].d(); // Expected: S::d()
        a[1].d(); // Expected: Tr::d()
    }
    for _ in 0..3 {
        f();
    }
    fn r(rp: *mut i32) -> *const i32 {
        rp
    }
    let mut v = 1;
    let my_ref = &mut v;
    let _rp = r(my_ref as *mut i32);
    println!("Raw pointer: {:?}", my_ref as *mut i32);

    {
        type Link<Node> = Option<Box<Node>>;
        #[derive(Debug)]
        struct Node {
            _next: Link<Node>,
        }
        let list = Some(Box::new(Node {
            _next: Some(Box::new(Node { _next: None })),
        }));
        fn ls(head: Link<Node>) -> Link<Node> {
            head
        }
        let _list2 = ls(list);
    }
    {
        struct MyPoint {
            x: i32,
            y: i32,
        }
        fn pattern_param_fn(
            MyPoint {
                x, /*: _x*/
                y: _y,
            }: MyPoint,
        ) {
        }
        pattern_param_fn(MyPoint { x: 2, y: -4 }); // main()::pattern_param_fn(MyPoint{x: 2, y: _y: -4}) {}

        fn ref_pattern_param_fn(
            &mut MyPoint {
                x, /*: _x*/
                y: _y,
            }: &mut MyPoint,
        ) {
        }
        ref_pattern_param_fn(&mut MyPoint { x: 2, y: -4 }); // main()::ref_pattern_param_fn(&mut MyPoint{x: 2, y: _y: -4}) {}

        fn tp((a, b): (i32, bool)) {}
        tp((8, false)); // main()::tp((a: 8, b: false)) {}
        fn tpp((MyPoint { x: _x1, y: _y1 }, MyPoint { x: _x2, y: _y2 }): (MyPoint, MyPoint)) {}
        tpp((MyPoint { x: 2, y: -4 }, MyPoint { x: -5, y: 6 })); // main()::tpp((MyPoint{x: _x1: 2, y: _y1: -4}, MyPoint{x: _x2: -5, y: _y2: 6})) {}

        struct MyTupleStruct(i32, char);
        fn f(MyTupleStruct(_i, _c): MyTupleStruct) {}
        f(MyTupleStruct(7, 'K')); // main()::f(MyTupleStruct(_i: 7, _c: 'K')) {}

        struct MyTupleStructS(MyPoint, char);
        fn fts(MyTupleStructS(MyPoint { x, y: _y }, char): MyTupleStructS) {}
        assert!(fcl::logging_is_on!());
        fcl::push_logging_is_on!(false);
        assert!(!fcl::logging_is_on!());
        fts(MyTupleStructS(MyPoint { x: -3, y: 8 }, 'h')); // main()::fts(MyTupleStructS(MyPoint{x: -3, y: _y: 8}, char: 'h')) {}
        fcl::pop_logging_is_on!();
        assert!(fcl::logging_is_on!());

        fn fs(&[a, b, ref i @ .., y, z]: &[i32; 6]) {}
        fs(&[0, 1, 2, 3, 4, 5]); // main()::fs(& [a: 0, b: 1, i: [2, 3], y: 4, z: 5]) {}
    }
    {
        use fcl_proc_macros::loggable;
        #[loggable]  // The instrumenting macro for `my_func()`.
        fn my_func() {
            f(); // Function `my_func()` invokes function `f()` 3 times.
            f();
            f();
            m::g(5); // Then function `g()` from module `m` (see below).

            #[derive(std::fmt::Debug)]
            struct MyStruct { // Structure defined locally in `my_func()`.
                _field: u8
            }
            impl MyStruct {
                fn my_method(&mut self) { // `my_method()` also gets instrumented automatically.
                    struct MyPoint {
                        x: i32,
                        y: i32
                    }
                    let mut p = MyPoint {
                        x: 2,
                        y: -4
                    };
                    // The closure below also gets instrumented automatically.
                    let my_closure = |param: u16, &mut MyPoint{ x, y }| { 
                        x + y
                    };
                    let _ = my_closure(1, &mut p);

                    eprintln!("This is a sample stderr output.");
                }
            }
            let mut s = MyStruct{ _field: 3 };
            s.my_method(); // Function `my_func()` invokes function `MyStruct::my_method()`.
        }

        // The instrumenting macro for the whole `mod m` below, 
        // all the internals get instrumented automatically.
        #[loggable] 
        mod m {
            pub fn g(param: i32) {
                for _ in 0..100 {
                    h();
                    i();
                }
            }
            fn h() {}
            fn i() -> f32 {
                j();
                -1.23
            }
            fn j() {}
        }

        #[loggable] // The instrumenting macro for `f()`.
        fn f() {}

        my_func(); // Invocation of the function.
    }
}

// use fcl_proc_macros::loggable;
#[fcl_proc_macros::loggable]
mod root {

    // use fcl_proc_macros::loggable;
    // use fcl::FunctionLogger;

    // mod m0; // Compiler Error: non-inline modules in proc macro input are unstable. see issue #54727
    mod m1 {}

    mod m {
        // use fcl_proc_macros::loggable;
        // use fcl::FunctionLogger;

        fn h() {}
        pub fn i() {
            h();
        }
    }
    pub fn f() {}
    pub fn g() {
        m::i();
    }
    // // #[fcl_proc_macros::loggable]
    // pub fn main() {
    //     fcl/*::call_log_infra */::_single_threaded_otimization!();
    //     // fcl::call_log_infra::THREAD_LOGGER.with(|logger| logger.borrow_mut().set_logging_is_on(true)); // Turn logging on.
    //     let _a = Some(0);
    //     f();
    //     g();

    //     {
    //         use crate::*;
    //         crate::S.d();  // Expected: S::d()
    //         // crate::<S as Tr>.d();  // Expected: S::d()
    //         crate::S2.d(); // Expected: Tr::d()
    //         crate::S2.e();

    //         let s = S;
    //         let s2 = S2;
    //         let a: Vec<&dyn Tr> = vec![ &s, &s2 ];
    //         a[0].d();   // Expected: S::d()
    //         a[1].d();   // Expected: Tr::d()
    //     }
    // }
}
// pub use root::*;

#[fcl_proc_macros::loggable]
trait Tr {
    fn d(&self) {} // Tr::d()
    fn e(&self);
}
struct S;
#[fcl_proc_macros::loggable]
impl Tr for S {
    fn d(&self) { // S::d()
        // <Self as Tr>::d(self); // NOTE: Causes recursion of S::d() instead of calling Tr::d().
        // Tr::d(self);    // NOTE: Causes recursion of S::d() instead of calling Tr::d().
        // self.Tr::d();
    }
    fn e(&self) {}
}
struct S2;
#[fcl_proc_macros::loggable]
impl Tr for S2 {
    // Reuses Tr::d()
    fn e(&self) {
        Some(1).map(|val| !val);
    }
}
