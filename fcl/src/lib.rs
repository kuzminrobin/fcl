#![feature(specialization)]

pub mod call_log_infra; // TODO: Really `pub`?
mod output_sync;
pub mod writer; // TODO: Really `pub`?

#[cfg(feature = "singlethreaded")]
use fcl_traits::CallLogger;

use call_log_infra::{instances::THREAD_LOGGER};

pub trait MaybePrint {
    fn maybe_print(&self) -> String;
}
impl<T> MaybePrint for T {
    default fn maybe_print(&self) -> String {
        String::from("?")
    }
}

impl<T: std::fmt::Debug> MaybePrint for T {
    fn maybe_print(&self) -> String {
        format!("{:?}", self)
    }
}

pub struct FunctionLogger {
    ret_val_str: Option<String>,
}

impl FunctionLogger {
    pub fn new(func_name: &str, param_vals: Option<String>) -> Self {
        #[cfg(feature = "singlethreaded")]
        THREAD_LOGGER.with(|logger| {
            logger
                .borrow_mut()
                .borrow_mut()
                .log_call(func_name, param_vals)
        });
        #[cfg(not(feature = "singlethreaded"))]
        THREAD_LOGGER.with(|logger| logger.borrow_mut().log_call(func_name, param_vals));
        Self {
            // _dropper: CalleeLogger,
            ret_val_str: None,
        }
    }
    pub fn set_ret_val(&mut self, ret_val_str: String) {
        self.ret_val_str = Some(ret_val_str);
    }
}
impl Drop for FunctionLogger {
    fn drop(&mut self) {
        #[cfg(feature = "singlethreaded")]
        THREAD_LOGGER.with(|logger| {
            logger
                .borrow_mut()
                .borrow_mut()
                .log_ret(self.ret_val_str.take());
        });

        #[cfg(not(feature = "singlethreaded"))]
        THREAD_LOGGER.with(|logger| logger.borrow_mut().log_ret(self.ret_val_str.take()));
    }
}

pub struct LoopbodyLogger;

impl LoopbodyLogger {
    pub fn new() -> Self {
        #[cfg(feature = "singlethreaded")]
        THREAD_LOGGER.with(|logger| {
            logger
                .borrow_mut()
                .borrow_mut()
                .log_loopbody_start();
        });

        #[cfg(not(feature = "singlethreaded"))]
        THREAD_LOGGER.with(|logger| logger.borrow_mut().log_loopbody_start());
        Self
    }
}
impl Drop for LoopbodyLogger {
    fn drop(&mut self) {
        #[cfg(feature = "singlethreaded")]
        THREAD_LOGGER.with(|logger| {
            // use fcl_traits::CallLogger;
            logger
                .borrow_mut()
                .borrow_mut()
                .log_loopbody_end();
        });

        #[cfg(not(feature = "singlethreaded"))]
        THREAD_LOGGER.with(|logger| logger.borrow_mut().log_loopbody_end());
    }
}

// pub struct ClosureLogger {
//     _dropper: CalleeLogger
// }

// impl ClosureLogger {
//     pub fn new(start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> Self {
//         THREAD_LOGGER.with(|logger| {
//             logger
//                 .borrow_mut()
//                 .log_call(&CalleeName::Closure(ClosureInfo {
//                     start_line,
//                     start_column,
//                     end_line,
//                     end_column,
//                 }))
//         });
//         Self { _dropper: CalleeLogger }
//     }
// }

// macro_rules! fcl {
//     () => {
//         // Global data shared by all the threads:
//         // TODO: Test with {file, socket, pipe} writer as an arg to `ThreadSharedWriter::new()`.
//         static mut THREAD_SHARED_WRITER: LazyLock<ThreadSharedWriterPtr> = LazyLock::new(|| {
//             Arc::new(RefCell::new(ThreadSharedWriter::new(
//                 Some(crate::writer::FclWriter::Stdout),
//                 // None,
//                 // Some(Box::new(std::io::stderr /*stdout*/())), /*None*/
//             )))
//         });
//         static mut CALL_LOGGER_ARBITER: LazyLock<Arc<Mutex<CallLoggerArbiter>>> = LazyLock::new(|| {
//             Arc::new(Mutex::new({
//                 let mut arbiter = unsafe { CallLoggerArbiter::new((*THREAD_SHARED_WRITER).clone()) };
//                 arbiter.set_std_output_sync();
//                 arbiter.set_panic_sync();
//                 arbiter
//             }))
//         });

//         // TODO: COnsider removing `LazyLock<RefCell<>>`.
//         static mut ORIGINAL_PANIC_HOOK: LazyLock<
//             RefCell<Option<Box<dyn Fn(&std::panic::PanicHookInfo<'_>)>>>,
//         > = LazyLock::new(|| RefCell::new(None));

//         // Global data per thread. Each thread has its own copy of these data.
//         // These data are initialized first upon thread start, and destroyed last upon thread termination.
//         thread_local! {
//             pub static THREAD_LOGGER: RefCell<Box<dyn CallLogger>> = {
//                 RefCell::new(Box::new(CallLoggerAdapter::new(
//                     {
//                         unsafe {
//                             match (*CALL_LOGGER_ARBITER).lock() {
//                                 Ok(mut guard) => {
//                                     guard.add_thread_logger(Box::new(
//                                         CallLogInfra::new(Rc::new(RefCell::new(
//                                             // fcl_decorators::TreeLikeDecorator::new(
//                                             //     Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
//                                             //     None, None, None))))))
//                                             fcl_decorators::CodeLikeDecorator::new(
//                                                 Some(Box::new(WriterAdapter::new((*THREAD_SHARED_WRITER).clone()))),
//                                                 None))))))
//                                 }
//                                 Err(e) => {
//                                     debug_assert!(false, "Unexpected mutex lock failure: '{:?}'", e);
//                                 }
//                             }
//                         }
//                         let call_logger_arbiter;
//                         unsafe {
//                             call_logger_arbiter = (*CALL_LOGGER_ARBITER).clone();
//                         }
//                         call_logger_arbiter
//                     })))
//             };
//         }
//     }
// }
