use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::{loggable, non_loggable};

use fcl::call_log_infra::instances::{THREAD_DECORATOR/* , THREAD_LOGGER*/};

use crate::{common::*};

/*
use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::loggable;

#[cfg(feature = "singlethreaded")]
use fcl::CallLogger;
use fcl::call_log_infra::instances::{THREAD_DECORATOR, THREAD_LOGGER};
 */
// trait
// impl
// in trait loggable in impl non_loggable
//      vice versa
// fn
// static

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
