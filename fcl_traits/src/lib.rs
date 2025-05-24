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

pub trait Flushable {
    fn flush(&mut self);
}

// pub trait Flusher {
//     fn register_flushable(&mut self, flushable: &dyn Flushable);
// }

pub trait CoderunNotifiable {
    // Non-cached call happened:
    fn notify_call(&mut self, _call_depth: usize, _name: &CalleeName) {}
    // Non-cached return happened:
    fn notify_return(&mut self, _call_depth: usize, _name: &CalleeName, _has_nested_calls: bool) {}
    // Repeat count has stopped being cached:
    fn notify_repeat_count(&mut self, _call_depth: usize, _name: &CalleeName, _count: usize) {}

    // fn get_flusher(&self) -> &mut dyn Flusher;
    fn set_flushable(&mut self, flushable: &dyn Flushable);
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

// Consider removing, is used internally only, not called from outside of the implementor.
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

