use std::io::{Write, stdout};

use fcl_traits::{
    CalleeName, CoderunDecorator, CoderunNotifiable, CoderunThreadSpecificNotifyable,
    RepeatCountCategory, ThreadSpecifics,
};

macro_rules! CLOSURE_NAME_FORMAT {
    () => {
        "closure{{{},{}:{},{}}}"
    }; // E.g. "closure{112,9:116,34}"
}

struct CommonDecorator {
    writer: Box<dyn Write>,
    thread_indent: &'static str,
}
impl CommonDecorator {
    fn new(writer: Option<Box<dyn Write>>) -> Self {
        Self {
            writer: writer.unwrap_or(Box::new(stdout())),
            thread_indent: &"",
        }
    }
    fn get_callee_name_string(name: &CalleeName) -> String {
        match name {
            CalleeName::Function(name) => name.clone(),
            CalleeName::Closure(info) => String::from(format!(
                CLOSURE_NAME_FORMAT!(),
                info.start_line, info.start_column, info.end_line, info.end_column
            )),
        }
    }
    fn set_thread_indent(&mut self, thread_indent: &'static str) {
        self.thread_indent = thread_indent;
    }

    fn get_thread_indent(&self) -> &'static str {
        self.thread_indent
    }
}

// By example of `println!()`.
macro_rules! decorator_write {
    ($self:ident, $($arg:tt)*) => {{
        let _result = write!($self.common.writer, $($arg)*);
        // $crate::io::_print($crate::format_args_nl!($($arg)*));
    }};
}

pub struct CodeLikeDecorator {
    common: CommonDecorator,
    indent_step: &'static str,
    line_end_pending: bool, // '\n' pending after "f() {" before printing a nested call.
}

impl CodeLikeDecorator {
    pub fn new(
        writer: Option<Box<dyn Write>>,
        indent_step: Option<&'static str>,
    ) -> Self {
        Self {
            common: CommonDecorator::new(writer),
            indent_step: indent_step.unwrap_or(&"  "),  // TODO: Test "    ", "\t".
            line_end_pending: false,
        }
    }
}

impl CoderunDecorator for CodeLikeDecorator {
    fn get_indent_string(&self, call_depth: usize) -> String {
        let mut indent_string = String::with_capacity(8);
        for _ in 0..call_depth {
            indent_string.push_str(self.indent_step);
        }
        indent_string
    }
}

impl CoderunNotifiable for CodeLikeDecorator {
    fn notify_flush(&mut self) {
        if self.line_end_pending {
            decorator_write!(self, "\n"); // '\n' after "parent() {" before an output of another thread.
            self.line_end_pending = false;
        }
    }
    fn notify_call(&mut self, call_depth: usize, name: &CalleeName) {
        if self.line_end_pending {
            decorator_write!(self, "\n"); // '\n' after "parent() {" before printing a nested call.
        }
        decorator_write!(
            self,
            "{}{}{}() {{",
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
            CommonDecorator::get_callee_name_string(name)
        ); // E.g. "<thread_indent><indent>sibling() {"
        self.line_end_pending = true; // '\n' pending. Won't be printed if there will be no nested calls (immediate "}\n").
    }
    fn notify_return(&mut self, call_depth: usize, name: &CalleeName, has_nested_calls: bool) {
        if !has_nested_calls && self.line_end_pending {
            decorator_write!(self, "}}\n"); // "}\n"
        } else {
            decorator_write!(
                self,
                "{}{}}} // {}().\n", // E.g. "<thread_indent><indent>} // sibling().\n".
                self.common.get_thread_indent(),
                self.get_indent_string(call_depth),
                CommonDecorator::get_callee_name_string(name)
            );
        }
        self.line_end_pending = false;
    }
    fn notify_repeat_count(
        &mut self,
        call_depth: usize,
        name: &CalleeName,
        count: RepeatCountCategory,
    ) {
        decorator_write!(
            self,
            "{}{}// {}() repeats {} time(s).\n", // E.g. "<thread_indent><indent>// sibling() repeats 8 time(s).\n"
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
            CommonDecorator::get_callee_name_string(name),
            count.to_string()
        );
    }
}

impl ThreadSpecifics for CodeLikeDecorator {
    fn set_thread_indent(&mut self, thread_indent: &'static str) {
        self.common.set_thread_indent(thread_indent);
    }
}

impl CoderunThreadSpecificNotifyable for CodeLikeDecorator {}

// TreeLikeDecorator Log           Explanation
// -----------------------------------------------
// +-f                               // f() {
// | +-g                             //   g() {}
// | +-h                             //   h() {}
// |   h repeats 99 time(s)          //   // h() repeats 99 time(s).
// | +-i                             //   i() {
// | | +-j                           //     j() {}
// | |   j repeats 9 time(s)         //     // j() repeats 9 time(s).
// | | +-k                           //     k() {}
// | |   k repeats 5 time(s)         //     // k() repeats 5 time(s).
//                                   //   } // i()
// |   i repeats 100 time(s)         //   // i() repeats 100 time(s).
#[rustfmt::skip]
pub struct TreeLikeDecorator {
    common: CommonDecorator,
    indent_step_call   : &'static str,  // "+-"  f
    indent_step_noncall: &'static str,  // "  "  Repeats ..
    indent_step_parent : &'static str,  // "| "  Prepends multiple times those above.
}

impl TreeLikeDecorator {
    #[rustfmt::skip]
    pub fn new(writer: Option<Box<dyn Write>>,
        indent_step_call   : Option<&'static str>,
        indent_step_noncall: Option<&'static str>,
        indent_step_parent : Option<&'static str>,
        // thread_indent: Option<&'static str>
    ) -> Self {
        Self {
            common: CommonDecorator::new(writer/*, thread_indent */),
            indent_step_call   : indent_step_call   .unwrap_or(&"+-"),
            indent_step_noncall: indent_step_noncall.unwrap_or(&"  "),
            indent_step_parent : indent_step_parent .unwrap_or(&"| ")
        }
    }
}

impl CoderunDecorator for TreeLikeDecorator {
    fn get_indent_string(&self, call_depth: usize) -> String {
        let mut indent_string = String::with_capacity(8);
        for _ in 0..call_depth {
            indent_string.push_str(self.indent_step_parent);
        }
        indent_string
    }
}

impl CoderunNotifiable for TreeLikeDecorator {
    fn notify_call(&mut self, call_depth: usize, name: &CalleeName) {
        decorator_write!(
            self,
            "{}{}{}{}\n",
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
            self.indent_step_call,
            CommonDecorator::get_callee_name_string(name)
        ); // E.g."<thread_indent><indent>+-sibling", "| | | | +-sibling"
    }

    // NOTE: Reusing the default behavior of `notify_return()` that does nothing.

    fn notify_repeat_count(
        &mut self,
        call_depth: usize,
        name: &CalleeName,
        count: RepeatCountCategory,
    ) {
        decorator_write!(
            self,
            "{}{}{}{} repeats {} time(s).\n", // E.g. "<thread_indent><indent> sibling repeats 8 time(s).\n"
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
            self.indent_step_noncall,
            CommonDecorator::get_callee_name_string(name),
            count.to_string()
        );
    }
}

impl ThreadSpecifics for TreeLikeDecorator {
    fn set_thread_indent(&mut self, thread_indent: &'static str) {
        self.common.set_thread_indent(thread_indent);
    }
}

impl CoderunThreadSpecificNotifyable for TreeLikeDecorator {}
