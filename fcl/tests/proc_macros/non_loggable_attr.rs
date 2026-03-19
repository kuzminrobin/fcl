use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::{loggable, non_loggable};

use fcl::call_log_infra::instances::{THREAD_DECORATOR/* , THREAD_LOGGER*/};

use crate::common::*;

/*
use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::loggable;

#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::{THREAD_DECORATOR, THREAD_LOGGER};
 */
// fn
// static

#[test]
fn in_impl() {
    #[loggable]
    fn prefixing_test(log: Rc<RefCell<Vec<u8>>>) {

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
            "  prefixing_test()::TestTrait<T>::trait_fn(_p: \"Log\") {}\n",
            "  prefixing_test()::<u8 as TestTrait<bool>>::trait_fn_loggable() {}\n",
            "  prefixing_test()::TestTrait<T>::trait_fn_loggable() {}\n",
        ));
    }

    // Create the mock log writer and substitute the default one with it:
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));

    THREAD_DECORATOR.with(|decorator| decorator.borrow_mut().set_writer(log.clone()));

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

        #[super::non_loggable]
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
