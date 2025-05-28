pub mod call_log_infra;
pub mod writer;

use call_log_infra::THREAD_LOGGER;
// use call_log_infra::CALL_LOG_INFRA;
use fcl_traits::{CalleeName, ClosureInfo};

#[macro_export]
macro_rules! closure_logger {
    ($start_line:expr, $start_col:expr, $end_line:expr, $end_col:expr) => {
        use fcl::call_log_infra::THREAD_LOGGER; // TODO: Consider moving to top of the file as a separate macro call.
        // use fcl::call_log_infra::CALL_LOG_INFRA;    // TODO: Consider moving to top of the file as a separate macro call.
        let mut _logger = None;
        THREAD_LOGGER.with(|logger| {
            if logger.borrow_mut().is_on() {
                _logger = Some(ClosureLogger::new($start_line, $start_col, $end_line, $end_col))
            }
        });
        // CALL_LOG_INFRA.with(|infra| {
        //     if infra.borrow_mut().is_on() {
        //         _logger = Some(ClosureLogger::new($start_line, $start_col, $end_line, $end_col))
        //     }
        // })
    }
}

pub struct FunctionLogger; // TODO: -> FunctionLogger (as opposed to ClosureLogger)

impl FunctionLogger {
    pub fn new(func_name: &'static str) -> Self {
        THREAD_LOGGER.with(|logger| {
            logger
                .borrow_mut()
                .log_call(&CalleeName::Function(func_name))
        });
        // CALL_LOG_INFRA.with(|infra| {
        //     infra
        //         .borrow_mut()
        //         .log_call(&CalleeName::Function(func_name))
        // });
        Self
    }
}

// TODO: Single `impl Drop` for a struct common for FunctionLogger and ClosureLogger.
impl Drop for FunctionLogger {
    fn drop(&mut self) {
        THREAD_LOGGER.with(|logger| logger.borrow_mut().log_ret());

        // CALL_LOG_INFRA.with(|infra| infra.borrow_mut().log_ret());
    }
}

pub struct ClosureLogger;

impl ClosureLogger {
    pub fn new(start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> Self {
        THREAD_LOGGER.with(|logger| {
            logger
                .borrow_mut()
                .log_call(&CalleeName::Closure(ClosureInfo {
                    start_line,
                    start_column,
                    end_line,
                    end_column,
                }))
        });
        // CALL_LOG_INFRA.with(|infra| {
        //     infra
        //         .borrow_mut()
        //         .log_call(&CalleeName::Closure(ClosureInfo {
        //             start_line,
        //             start_column,
        //             end_line,
        //             end_column,
        //         }))
        // });
        Self
    }
}
impl Drop for ClosureLogger {
    fn drop(&mut self) {
        THREAD_LOGGER.with(|logger| logger.borrow_mut().log_ret());
        // CALL_LOG_INFRA.with(|infra| infra.borrow_mut().log_ret());
    }
}
