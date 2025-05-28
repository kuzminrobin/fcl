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

type RepeatCountType = usize;

pub enum RepeatCountCategory {
    Exact(RepeatCountType),     // Repeats 6 time(s). // None is RepeatCountType::MAX.
    AtLeast(RepeatCountType),   // Repeats 6+ time(s). // The `overall` is RepeatCountType::MAX.
    Unknown // Repeats ? time(s). // Both are RepeatCountType::MAX.
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
        if self.overall < RepeatCountType::MAX {
            return RepeatCountCategory::Exact(self.overall - self.flushed)
        } else if self.flushed < RepeatCountType::MAX {
            return RepeatCountCategory::AtLeast(self.overall - self.flushed)
        }
        RepeatCountCategory::Unknown
    }
    pub fn non_flushed_is_empty(&self) -> bool {
        // Equal but not both are saturated:
        self.overall == self.flushed && self.flushed < RepeatCountType::MAX
    }
    pub fn inc(&mut self) {
        if self.overall < RepeatCountType::MAX {
            self.overall += 1
        }
    }
    pub fn mark_flushed(&mut self) {
        self.flushed = self.overall
    }
}
// TODO: Consider removing the default behavior.
pub trait CoderunNotifiable {
    // Non-cached call happened:
    fn notify_call(&mut self, _call_depth: usize, _name: &CalleeName) {}
    // Non-cached return happened:
    fn notify_return(&mut self, _call_depth: usize, _name: &CalleeName, _has_nested_calls: bool) {}
    // Repeat count has stopped being cached:
    fn notify_repeat_count(&mut self, _call_depth: usize, _name: &CalleeName, _count: RepeatCountCategory) {}
    // fn notify_repeat_count(&mut self, _call_depth: usize, _name: &CalleeName, _count: usize) {}

    fn notify_flush(&mut self) {}
}

pub trait ThreadSpecifics {
    fn set_thread_indent(&mut self, thread_indent: &'static str);
}

pub trait CoderunThreadSpecificNotifyable: CoderunNotifiable + ThreadSpecifics {}

// macro_rules! CLOSURE_NAME_FORMAT {
//     () => {
//         "closure{{{},{}:{},{}}}" // "closure{112,9:116,34}"
//     };
// }

pub trait CodeRunDecorator {    // TODO: CodeRunDecorator -> CoderunDecorator, code_run -> coderun
    fn get_indent_string(&self, call_depth: usize) -> String;
    // fn get_callee_name_string(name: &CalleeName) -> String {
    //     match name {
    //         CalleeName::Function(slice) => String::from(*slice),
    //         CalleeName::Closure(info) => String::from(format!(
    //             CLOSURE_NAME_FORMAT!(),
    //             info.start_line, info.start_column, info.end_line, info.end_column
    //         )),
    //     }
    // }
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
