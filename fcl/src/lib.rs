pub mod call_log_infra;
pub mod writer;

use call_log_infra::THREAD_LOGGER;
use code_commons::{CalleeName, ClosureInfo};

#[macro_export]
macro_rules! closure_logger {
    ($start_line:expr, $start_col:expr, $end_line:expr, $end_col:expr) => {
        let mut _logger = None;
        fcl::call_log_infra::THREAD_LOGGER.with(|logger| {
            if logger.borrow_mut().logging_is_on() {
                _logger = Some(ClosureLogger::new($start_line, $start_col, $end_line, $end_col))
            }
        });
    }
}

struct CalleeLogger;
impl Drop for CalleeLogger {
    fn drop(&mut self) {
        THREAD_LOGGER.with(|logger| logger.borrow_mut().log_ret());
    }
}

pub struct FunctionLogger {
    _dropper: CalleeLogger
}

impl FunctionLogger {
    pub fn new(func_name: &str) -> Self {
        THREAD_LOGGER.with(|logger| {
            logger
                .borrow_mut()
                .log_call(&CalleeName::Function(String::from(func_name)))
        });
        Self { _dropper: CalleeLogger }
    }
}

pub struct ClosureLogger {
    _dropper: CalleeLogger
}

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
        Self { _dropper: CalleeLogger }
    }
}
