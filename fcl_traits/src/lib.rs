use code_commons::{CalleeName, CoderunNotifiable};
/// Trait to be implemented by the instances that handle any thread specifics.
pub trait ThreadSpecifics {
    /// Sets the thread code run output indentation. E.g. if there are 2 threads, 
    /// one thread's output can be logged in the left half of the console,
    /// and the other thread's output can be logged in the right half, 
    /// or _indented_ by half of the console width.
    fn set_thread_indent(&mut self, thread_indent: &'static str);
}

pub trait CoderunThreadSpecificNotifyable: CoderunNotifiable + ThreadSpecifics {}

pub trait CoderunDecorator {
    fn get_indent_string(&self, call_depth: usize) -> String;
}

pub trait CallLogger {
    fn push_logging_is_on(&mut self, is_on: bool);
    fn pop_logging_is_on(&mut self);
    fn logging_is_on(&self) -> bool;
    fn set_logging_is_on(&mut self, is_on: bool);

    fn set_thread_indent(&mut self, thread_indent: &'static str);

    fn log_call(&mut self, name: &CalleeName);
    fn log_ret(&mut self);
    fn flush(&mut self) {}
}
