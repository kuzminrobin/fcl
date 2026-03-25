use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::{loggable, non_loggable};

#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::THREAD_DECORATOR;

use crate::common::*;


#[test]
fn in_static() {
    #[loggable(skip_closure_coords)]
    // The `instrumenter()` recursively makes the local entities loggable by default.
    fn instrumenter() {
        static STATIC_VAR: u8 = {
            // NOTE: The const context.
            // * The called functions must be const (cannot be `#[loggable]`).
            // * The const closures `Some(4).map(const |x| true)` are not allowed on stable:
            //   error[E0277]: the trait bound `[const] Destruct` is not satisfied.
            #[non_loggable]
            const fn non_loggable_const_initializer() -> u8 {
                1
            }
            non_loggable_const_initializer()
        };

        // Assert: The behavior didn't change because of FCL.
        let testable_behavior = STATIC_VAR;
        assert_eq!(1, testable_behavior);

        static S: std::sync::LazyLock<bool> = {
            fn loggable_initializer() -> bool {
                false
            }
            #[non_loggable]
            fn non_loggable_initializer() -> bool {
                true
            }
            std::sync::LazyLock::new(|| loggable_initializer() | non_loggable_initializer())
        };

        // Assert: The behavior didn't change because of FCL.
        let testable_behavior = *S;
        assert_eq!(true, testable_behavior);

        // TODO: `#[non_loggable] static S..`
    }

    let log = substitute_log_writer!();

    // Generate some log:
    instrumenter();

    #[rustfmt::skip]
    test_assert!(log, concat!(
        "instrumenter() {\n",
           // Assert: `non_loggable_const_initializer()` is not logged:
        "  instrumenter::closure{..}() {\n",
        "    instrumenter::loggable_initializer() {} -> false\n",
            // Assert: "instrumenter::non_loggable_initializer() {} -> true\n" is not logged:
        "  } -> true // instrumenter::closure{..}().\n",
        "} // instrumenter().\n",
    ));
}

#[test]
fn in_fn() {
    #[loggable]
    // The `instrumenter()` recursively makes the local entities loggable by default.
    fn instrumenter() {
        // fn and closure:
        #[non_loggable]
        fn non_loggable_fn() {
            // Non-loggable fn and closure.
            Some(2).map(|x| x + 1);
        }
        non_loggable_fn();

        // mod
        #[non_loggable]
        mod non_loggable_mod {
            pub fn non_loggable_mod_fn() {}
        }
        non_loggable_mod::non_loggable_mod_fn();

        // trait
        #[non_loggable]
        trait NonLoggableTrait {
            fn non_loggable_trait_fn() {}
        }
        impl NonLoggableTrait for i8 {}
        i8::non_loggable_trait_fn();

        // trait impl
        #[non_loggable]
        impl NonLoggableTrait for u8 {
            fn non_loggable_trait_fn() {}
        }
        u8::non_loggable_trait_fn();

        // struct impl
        struct LoggingNeutralStruct {}
        #[non_loggable]
        impl LoggingNeutralStruct {
            fn non_loggable_impl_struct_fn() {}
            fn non_loggable_impl_struct_self_fn(&mut self) {}
        }
        LoggingNeutralStruct::non_loggable_impl_struct_fn();
        LoggingNeutralStruct {}.non_loggable_impl_struct_self_fn();
    }

    let log = substitute_log_writer!();

    // Generate some log:
    instrumenter();

    #[rustfmt::skip]
    test_assert!(log, concat!(
        "instrumenter() {",
            // Assert: The following are not logged:
            // * `non_loggable_fn()` and closure in it.
            //   "  instrumenter::non_loggable_fn() {\n",
            //   "    instrumenter::non_loggable_fn::closure{28,25:28,33}(x: 2) {} -> 3\n",
            //   "  } // instrumenter::non_loggable_fn().\n",
            // * `non_loggable_mod::non_loggable_mod_fn()`.
            //   "  instrumenter::non_loggable_mod::non_loggable_mod_fn() {}\n"
            // * `<i8 as NonLoggableTrait>::non_loggable_trait_fn()`.
            //   "  instrumenter::NonLoggableTrait::non_loggable_trait_fn() {}\n"
            // * `<u8 as NonLoggableTrait>::non_loggable_trait_fn()`. 
            //   "  instrumenter::<u8 as NonLoggableTrait>::non_loggable_trait_fn() {}\n"
            // * `LoggingNeutralStruct::non_loggable_impl_struct_fn()`. 
            //   "  instrumenter::LoggingNeutralStruct::non_loggable_impl_struct_fn() {}\n"
            // * `LoggingNeutralStruct::non_loggable_impl_struct_self_fn(&mut self: {})`.
            //   "  instrumenter::LoggingNeutralStruct::non_loggable_impl_struct_self_fn(self: &mut ?) {}\n"
        "}\n",
    ));
}

#[test]
fn in_impl() {
    #[loggable]
    fn prefixing_test(log: Rc<RefCell<Vec<u8>>>) {
        // TODO: `impl Struct`.

        // In the `trait` - loggable, in `impl` - non-loggable.
        trait TestTrait<T> {
            fn trait_fn(_p: &str) {} // Loggable in the `trait`.
            fn trait_fn_loggable() {} // Loggable in the `trait`.
        }
        impl TestTrait<bool> for u8 {
            #[non_loggable]
            fn trait_fn(_p: &str) {} // Loggable in the `trait`, non_loggable in the `impl`.
            fn trait_fn_loggable() {} // Loggable (after `#[non_loggable]` fn) both in the `trait` and in the `impl`.
        }
        impl TestTrait<f32> for u8 {} // All the associated funcs are loggable from the `trait`.

        // Generate some test log:
        <u8 as TestTrait<bool>>::trait_fn(&"Don't Log"); // Do not log.
        <u8 as TestTrait<f32>>::trait_fn(&"Log");

        <u8 as TestTrait<bool>>::trait_fn_loggable();
        <u8 as TestTrait<f32>>::trait_fn_loggable();

        #[rustfmt::skip]
        test_assert!(log, concat!(
            "prefixing_test(log: RefCell { value: [] }) {\n",
            // Assert: `<u8 as TestTrait<bool>>::trait_fn()` is not logged.
            "  prefixing_test::TestTrait<T>::trait_fn(_p: \"Log\") {}\n",
            "  prefixing_test::<u8 as TestTrait<bool>>::trait_fn_loggable() {}\n",
            "  prefixing_test::TestTrait<T>::trait_fn_loggable() {}\n",
        ));
    }

    let log = substitute_log_writer!();

    prefixing_test(log.clone());

    log.borrow_mut().clear();

    // In the `trait` - non-loggable, in the `impl` - loggable.
    trait TestTrait<T> {
        fn trait_fn(_p: &str) {} // Non-loggable in the `trait`.
        fn trait_fn_non_loggable() {} // Non-loggable in the `trait`.
    }
    impl TestTrait<bool> for u8 {
        #[loggable]
        fn trait_fn(_p: &str) {} // Non-loggable in the `trait`, loggable in the `impl`.
        fn trait_fn_non_loggable() {} // Non-loggable (after `#[loggable]` fn) both in the `trait` and in the `impl`.
    }
    impl TestTrait<f32> for u8 {} // All the associated funcs are non-loggable from the `trait`.

    // Generate some test log:
    <u8 as TestTrait<bool>>::trait_fn(&"Log");
    <u8 as TestTrait<f32>>::trait_fn(&"Don't log"); // Do not log.
    <u8 as TestTrait<bool>>::trait_fn_non_loggable(); // Do not log.
    <u8 as TestTrait<f32>>::trait_fn_non_loggable(); // Do not log.

    #[rustfmt::skip]
    test_assert!(log, concat!(
        "trait_fn(_p: \"Log\") {}\n",
        // Assert: All other calls are not logged.
    ));
}

#[test]
fn in_trait() {
    #[loggable]
    trait TestSuperTraitA<T: Default> {
        #[non_loggable]
        fn non_loggable_trait_fn_a() -> T {
            T::default()
        }
    }
    #[loggable]
    trait TestSuperTraitB<U: Default> {
        fn loggable_trait_fn_b() -> U {
            U::default()
        }
    }
    #[loggable]
    trait TestTrait<T: Default, U: Default>: TestSuperTraitA<T> + TestSuperTraitB<U> {
        fn loggable_fn() {}

        #[non_loggable]
        fn non_loggable_fn() {}
    }

    impl TestSuperTraitA<bool> for bool {}
    impl TestSuperTraitB<u8> for bool {}
    impl TestTrait<bool, u8> for bool {}

    // Create the mock log writer and substitute the default one with it:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    // Generate some log:
    bool::loggable_fn();
    bool::non_loggable_fn(); // Not to be logged.
    bool::loggable_fn();
    bool::non_loggable_trait_fn_a(); // Not to be logged.
    bool::loggable_trait_fn_b();
    flush_log();

    #[rustfmt::skip]
    test_assert!(log, concat!(
        "TestTrait<T:Default,U:Default>::loggable_fn() {}\n",
        // Assert: `#[non_loggable]` function is not logged.
        "// TestTrait<T:Default,U:Default>::loggable_fn() repeats 1 time(s).\n",
        // Assert: `#[non_loggable]` function is not logged.
        "TestSuperTraitB<U:Default>::loggable_trait_fn_b() {} -> 0\n",
    ));
}

#[test]
fn in_mod() {
    #[loggable]
    mod test_mod {
        pub fn loggable_fn() {}

        #[fcl_proc_macros::non_loggable]
        // #[super::non_loggable]
        pub fn non_loggable_fn() {}
    }
    // Mock log writer creation and substitution of the default one:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

    test_mod::loggable_fn();
    test_mod::non_loggable_fn(); // Not to be logged.
    test_mod::loggable_fn();
    flush_log();

    #[rustfmt::skip]
    test_assert!(log, concat!(
        "test_mod::loggable_fn() {}\n",
        // Assert: `#[non_loggable]` function is not logged.
        "// test_mod::loggable_fn() repeats 1 time(s).\n",
    ));
}
