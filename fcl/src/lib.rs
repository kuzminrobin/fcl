#![feature(specialization)]

pub mod call_log_infra; // TODO: Really `pub`?
pub mod writer; // TODO: Really `pub`?
mod output_sync;

use call_log_infra::THREAD_LOGGER;
// use code_commons::{ClosureInfo};
// use code_commons::{CalleeName, ClosureInfo};

pub trait MaybePrint {
    fn maybe_print(&self) -> String;
}
impl<T> MaybePrint for T {
    default fn maybe_print(&self) -> String {
        String::from("?")
    }
}

impl<T: std::fmt::Debug> MaybePrint for T {
    // impl<T: std::fmt::Display> MaybePrint for T {
    fn maybe_print(&self) -> String {
        format!("{:?}", self)
    }
}

// #[macro_export]
// macro_rules! closure_logger {
//     ($start_line:expr, $start_col:expr, $end_line:expr, $end_col:expr) => {
//         let mut _logger = None;
//         fcl::call_log_infra::THREAD_LOGGER.with(|logger| {
//             if logger.borrow_mut().logging_is_on() {
//                 _logger = Some(ClosureLogger::new($start_line, $start_col, $end_line, $end_col))
//             }
//         });
//     }
// }

// struct CalleeLogger; // TODO: Merge with FunctionLogger.
// impl Drop for CalleeLogger {
//     fn drop(&mut self) {
//         THREAD_LOGGER.with(|logger|
//             logger.borrow_mut().log_ret(self.output));
//     }
// }

pub struct FunctionLogger {
    // _dropper: CalleeLogger,
    output: Option<String>, // TODO: -> ret_val_str
}

impl FunctionLogger {
    pub fn new(func_name: &str, param_vals: Option<String>) -> Self {
        THREAD_LOGGER.with(|logger| {
            logger.borrow_mut().log_call(func_name, param_vals)
        });
        Self {
            // _dropper: CalleeLogger,
            output: None,
        }
    }
    pub fn set_output(&mut self, output: String) {
        self.output = Some(output);
    }
}
impl Drop for FunctionLogger {
    fn drop(&mut self) {
        THREAD_LOGGER.with(|logger| 
            logger.borrow_mut().log_ret(self.output.take()));
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
