#[derive(PartialEq, Clone)]
pub struct ClosureInfo {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(PartialEq, Clone)]
pub enum CalleeName {   // TODO: Consider -> CalleeID
    Function(&'static str),
    Closure(ClosureInfo),
}

/// Function call repeat count data type.
type RepeatCountType = usize;
/// The maximum value (saturation value) for the function call repeat count data type.
/// The function can be called in a loop endlessly, such that the function call repeat count can potentially overflow.
/// The algorithm increments that count to the maximum (saturation) value and then stops incrementing.
const REPEAT_COUNT_MAX: RepeatCountType = RepeatCountType::MAX;

pub enum RepeatCountCategory {
    Exact(RepeatCountType),     // Repeats 6 time(s). // None is REPEAT_COUNT_MAX.
    AtLeast(RepeatCountType),   // Repeats 6+ time(s). // The `overall` is REPEAT_COUNT_MAX.
    Unknown // Repeats ? time(s). // Both are REPEAT_COUNT_MAX.
}
impl RepeatCountCategory {
    pub fn to_string(&self) -> String {
        match self {
            RepeatCountCategory::Exact(exact) => exact.to_string(),
            RepeatCountCategory::AtLeast(at_least) => at_least.to_string() + "+",
            RepeatCountCategory::Unknown => "?".to_string()
        }
    }
}

/// The function call repeat count. Consists of the two parts.
/// * Actual repeat count. Stops incrementing upon reaching `REPEAT_COUNT_MAX` (saturates).
/// * The flushed part of the actual repeat count. Value less than or equal to the actual repeat count.
#[derive(PartialEq)]
pub struct RepeatCount {
    overall: RepeatCountType,
    flushed: RepeatCountType    // flushed <= overall
}
impl RepeatCount {
    pub fn new() -> Self {
        Self { overall: 0, flushed: 0 }
    }
    pub fn non_flushed(&self) -> RepeatCountCategory {
        if self.overall < REPEAT_COUNT_MAX {
            return RepeatCountCategory::Exact(self.overall - self.flushed)
        } else if self.flushed < REPEAT_COUNT_MAX {
            return RepeatCountCategory::AtLeast(self.overall - self.flushed)
        }
        RepeatCountCategory::Unknown
    }
    pub fn non_flushed_is_empty(&self) -> bool {
        // Equal but not both are saturated:
        self.overall == self.flushed && self.flushed < REPEAT_COUNT_MAX
    }
    pub fn inc(&mut self) {
        if self.overall < REPEAT_COUNT_MAX {
            self.overall += 1
        }
    }
    pub fn mark_flushed(&mut self) {
        self.flushed = self.overall
    }
}

/// Trait to be implemented by the instances that need to be notified about the code run events 
/// (such as function or closure calls, returns, etc.).
pub trait CoderunNotifiable {
    /// Non-cached call happened.
    fn notify_call(&mut self, _call_depth: usize, _name: &CalleeName) {}
    /// Non-cached return happened.
    fn notify_return(&mut self, _call_depth: usize, _name: &CalleeName, _has_nested_calls: bool) {}
    /// Repeat count has stopped being cached.
    fn notify_repeat_count(&mut self, _call_depth: usize, _name: &CalleeName, _count: RepeatCountCategory) {}
    // fn notify_repeat_count(&mut self, _call_depth: usize, _name: &CalleeName, _count: usize) {}

    /// Flush needed (any output cached by this trait implementor needs to be flushed).
    fn notify_flush(&mut self) {}
}

/// Trait to be implemented by the instances that handle any thread specifics.
pub trait ThreadSpecifics {
    /// Sets the thread code run output indentation. E.g. if there are 2 threads, 
    /// one thread's output can be logged in the left half of the console,
    /// and the other thread's output can be logged in the right half, 
    /// or _indented_ by half of the console width.
    fn set_thread_indent(&mut self, thread_indent: &'static str);
}

pub trait CoderunThreadSpecificNotifyable: CoderunNotifiable + ThreadSpecifics {}

pub trait CodeRunDecorator {    // TODO: CodeRunDecorator -> CoderunDecorator, code_run -> coderun
    fn get_indent_string(&self, call_depth: usize) -> String;
}

pub trait CallLogger {
    fn push_is_on(&mut self, is_on: bool);
    fn pop_is_on(&mut self);
    fn is_on(&self) -> bool;
    fn set_is_on(&mut self, is_on: bool);

    fn set_thread_indent(&mut self, thread_indent: &'static str);

    fn log_call(&mut self, name: &CalleeName);
    fn log_ret(&mut self);
    fn flush(&mut self) {}
}
