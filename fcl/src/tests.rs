use std::cell::RefCell;
use std::rc::Rc;

use fcl_proc_macros::loggable;

use crate::call_log_infra::instances::THREAD_DECORATOR;
use crate as fcl;

#[loggable]
fn f() {}

#[test]
fn basics() {
    let log = Rc::new(RefCell::new(Vec::with_capacity(1024)));
    THREAD_DECORATOR.with(|decorator| 
        decorator.borrow_mut().set_writer(log.clone()));
    // let s = unsafe { String::from_utf8_unchecked(log) };
    // let s_slice = unsafe { std::str::from_utf8_unchecked(&*log.borrow()) };
    f();
    unsafe { assert_eq!(std::str::from_utf8_unchecked(&*log.borrow()), "f() {}\n")  };

    // let s_slice = unsafe { std::str::from_utf8_unchecked(&*log.borrow()) };
    // assert_eq!(s_slice, "f() {}");

    // assert_eq!(String::from_utf8(&*log.borrow())/*.as_bytes() */, String::from("f() {}"));//b"f() {}"); //Vec::from_raw_parts());  //String::from(""))
}
