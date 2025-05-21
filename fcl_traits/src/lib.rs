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

// TODO: Consider removing the default behavior.
pub trait CoderunNotifiable {
    // Non-cached call happened:
    fn notify_call(&mut self, _call_depth: usize, _name: &CalleeName) {}
    // Non-cached return happened:
    fn notify_return(&mut self, _call_depth: usize, _name: &CalleeName, _has_nested_calls: bool) {}
    // Repeat count has stopped being cached:
    fn notify_repeat_count(&mut self, _call_depth: usize, _name: &CalleeName, _count: usize) {}
}

macro_rules! CLOSURE_NAME_FORMAT {
    () => {
        "closure{{{},{}:{},{}}}" // "closure{112,9:116,34}"
    };
}

pub trait CodeRunDecorator {
    fn get_indent_string(&self, call_depth: usize) -> String;
    fn get_callee_name_string(name: &CalleeName) -> String {
        match name {
            CalleeName::Function(slice) => String::from(*slice),
            CalleeName::Closure(info) => String::from(format!(
                CLOSURE_NAME_FORMAT!(),
                info.start_line, info.start_column, info.end_line, info.end_column
            )),
        }
    }
}
