#![feature(specialization)]

pub mod call_log_infra; // TODO: Really `pub`?
pub mod writer; // TODO: Really `pub`?
mod output_sync;

use call_log_infra::THREAD_LOGGER;

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
    // _dropper: CalleeLogger,
    ret_val_str: Option<String>,
}

impl FunctionLogger {
    pub fn new(func_name: &str, param_vals: Option<String>) -> Self {
        THREAD_LOGGER.with(|logger| {
            logger.borrow_mut().log_call(func_name, param_vals)
        });
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
        THREAD_LOGGER.with(|logger| 
            logger.borrow_mut().log_ret(self.ret_val_str.take()));
    }
}

pub struct LoopbodyLogger;

impl LoopbodyLogger {
    pub fn new() -> Self {
        THREAD_LOGGER.with(|logger| {
            logger.borrow_mut().log_loopbody_start()
        });
        Self
    }
}
impl Drop for LoopbodyLogger {
    fn drop(&mut self) {
        THREAD_LOGGER.with(|logger| 
            logger.borrow_mut().log_loopbody_end());
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
