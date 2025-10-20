use std::{
    cell::RefCell,
    io::{Write, stdout},
    rc::Rc,
};

use code_commons::{CoderunNotifiable, ItemKind, RepeatCountCategory};

/// Trait to be implemented by the instances that handle any thread specifics.
/// For example, the thread's output can be shifted by half of the console.
pub trait ThreadSpecific {
    /// Sets the thread's output indentation. E.g. if there are 2 threads,
    /// one thread's output can be logged in the left half of the console,
    /// and the other thread's output can be logged in the right half,
    /// or _indented_ by half of the console width.
    fn set_thread_indent(&mut self, thread_indent: String);
}

pub trait WriterPossessor {
    fn set_writer(&mut self, _writer: Rc<RefCell<dyn Write>>) {}
}

pub trait LogDecorator: CoderunNotifiable + ThreadSpecific + WriterPossessor {}

enum Writer {
    Actual(Box<dyn Write>),
    Substitute(Rc<RefCell<dyn Write>>),
}
struct CommonDecorator {
    writer: Writer,
    thread_indent: String,
}
impl CommonDecorator {
    fn new(writer: Option<Box<dyn Write>>) -> Self {
        Self {
            writer: Writer::Actual(writer.unwrap_or(Box::new(stdout()))),
            thread_indent: String::from(""),
        }
    }
    fn set_thread_indent(&mut self, thread_indent: String) {
        self.thread_indent = thread_indent;
    }

    fn get_thread_indent(&self) -> String {
        self.thread_indent.clone()
    }

    fn set_writer(&mut self, writer: Rc<RefCell<dyn Write>>) {
        self.writer = Writer::Substitute(writer);
    }
}

// By example of `println!()`.
macro_rules! decorator_write {  // TODO: Condsider renaming to `writer_write()`.
    ($self:ident, $($arg:tt)*) => {{
        let writer = match &mut $self.common.writer {
            Writer::Actual(writer) => &mut **writer,
            Writer::Substitute(writer) => &mut *writer.borrow_mut(),
        };
        let _ignore_result = write!(writer, $($arg)*);  // TODO: Comment why ignore.
    }};
}

pub struct CodeLikeDecorator {
    common: CommonDecorator,
    indent_step: &'static str,
    line_end_pending: bool, // '\n' pending after "f() {" before printing a nested call.
}

impl CodeLikeDecorator {
    pub fn new(writer: Option<Box<dyn Write>>, indent_step: Option<&'static str>) -> Self {
        Self {
            common: CommonDecorator::new(writer),
            indent_step: indent_step.unwrap_or(&"  "),
            line_end_pending: false,
        }
    }
    fn get_indent_string(&self, call_depth: usize) -> String {
        let mut indent_string = String::with_capacity(8);
        for _ in 0..call_depth {
            indent_string.push_str(self.indent_step);
        }
        indent_string
    }
    fn get_indents(&self, call_depth: usize) -> (String, String) {
        (
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
        )
    }
}

const LOOPBODY_NAME: &str = &"Loop body";

impl CoderunNotifiable for CodeLikeDecorator {
    fn notify_flush(&mut self) {
        if self.line_end_pending {
            decorator_write!(self, "\n"); // '\n' after "parent() {" before an output of another thread.
            self.line_end_pending = false;
        }
    }
    fn notify_call(&mut self, call_depth: usize, name: &str, param_vals: &Option<String>) {
        if self.line_end_pending {
            decorator_write!(self, "\n"); // '\n' after "parent() {" before printing a nested call.
        }
        let indents = self.get_indents(call_depth);
        decorator_write!(
            self,
            "{}{}{}({}) {{",
            indents.0,
            indents.1,
            name,
            param_vals
                .as_ref()
                .map(|string| string.as_str())
                .unwrap_or(&""),
        ); // E.g. "<thread_indent><indent>sibling() {"
        self.line_end_pending = true; // '\n' pending. Won't be printed if there will be no nested calls (immediate "}\n").
    }
    fn notify_return(
        &mut self,
        call_depth: usize,
        name: &str,
        has_nested_calls: bool,
        ret_val: &Option<String>,
    ) {
        let ret_val_str = ret_val.as_ref().map_or_else(
            || "".to_string(), // None -> "".
            |output| format!(" -> {}", output),
        ); // Some() -> "-> Value".
        if !has_nested_calls && self.line_end_pending {
            decorator_write!(self, "}}{}\n", ret_val_str); // "}\n"
        } else {
            let indents = self.get_indents(call_depth);
            decorator_write!(
                self,
                "{}{}}}{} // {}().\n", // E.g. "<thread_indent><indent>} -> RetVal // sibling().\n".
                indents.0, 
                indents.1,
                ret_val_str,
                name
            );
        }
        self.line_end_pending = false;
    }
    fn notify_repeat_count(
        &mut self,
        call_depth: usize,
        kind: &ItemKind,
        count: RepeatCountCategory,
    ) {
        let item_name = match kind {
            ItemKind::Call { name, .. } => format!("{}()", name),
            ItemKind::Loopbody { .. } => String::from(LOOPBODY_NAME), //"Loop body"),
        };
        let indents = self.get_indents(call_depth);
        decorator_write!(
            self,
            "{}{}// {} repeats {} time(s).\n", // E.g. "<thread_indent><indent>// sibling() repeats 8 time(s).\n"
            indents.0, 
            indents.1,
            item_name,
            count.to_string()
        );
    }
    fn notify_loopbody_start(&mut self, call_depth: usize) {
        if self.line_end_pending {
            decorator_write!(self, "\n"); // '\n' after "parent() {" before printing a nested call.
        }
        let indents = self.get_indents(call_depth);
        decorator_write!(
            self,
            "{}{}{{ // {} start.\n",
            indents.0, 
            indents.1,
            LOOPBODY_NAME,
        ); // E.g. "<thread_indent><indent>{ // Loop body start."
        self.line_end_pending = false;
    }
    fn notify_loopbody_end(&mut self, call_depth: usize) {
        let indents = self.get_indents(call_depth);
        decorator_write!(
            self,
            "{}{}}} // {} end.\n", // E.g. "<thread_indent><indent>} // Loop body end.\n".
            indents.0, 
            indents.1,
            LOOPBODY_NAME,
        );
        self.line_end_pending = false;
    }
}

impl ThreadSpecific for CodeLikeDecorator {
    fn set_thread_indent(&mut self, thread_indent: String) {
        self.common.set_thread_indent(thread_indent);
    }
}

impl WriterPossessor for CodeLikeDecorator {
    fn set_writer(&mut self, writer: Rc<RefCell<dyn Write>>) {
        self.common.set_writer(writer);
    }
}

impl LogDecorator for CodeLikeDecorator {}

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
    ) -> Self {
        Self {
            common: CommonDecorator::new(writer),
            indent_step_call   : indent_step_call   .unwrap_or(&"+-"),
            indent_step_noncall: indent_step_noncall.unwrap_or(&"  "),
            indent_step_parent : indent_step_parent .unwrap_or(&"| ")
        }
    }
    fn get_indent_string(&self, call_depth: usize) -> String {
        let mut indent_string = String::with_capacity(8);
        for _ in 0..call_depth {
            indent_string.push_str(self.indent_step_parent);
        }
        indent_string
    }
    fn get_indents(&self, call_depth: usize) -> (String, String) {
        (
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
        )
    }
}

impl CoderunNotifiable for TreeLikeDecorator {
    fn notify_call(&mut self, call_depth: usize, name: &str, param_vals: &Option<String>) {
        let indents = self.get_indents(call_depth);
        decorator_write!(
            self,
            "{}{}{}{}({})\n",
            indents.0,
            indents.1,
            self.indent_step_call,
            name,
            param_vals
                .as_ref()
                .map(|string| string.as_str())
                .unwrap_or(&"")
        ); // E.g."<thread_indent><indent>+-sibling", "| | | | +-sibling"
    }

    // NOTE: Reusing the default behavior of `notify_return()` that does nothing.

    fn notify_repeat_count(
        &mut self,
        call_depth: usize,
        kind: &ItemKind,
        count: RepeatCountCategory,
    ) {
        let item_name = match kind {
            ItemKind::Call { name, .. } => name.clone(),
            ItemKind::Loopbody { .. } => String::from(LOOPBODY_NAME), //"Loop body"),
        };
        let indents = self.get_indents(call_depth);
        decorator_write!(
            self,
            "{}{}{}{} repeats {} time(s).\n", // E.g. "<thread_indent><indent> sibling repeats 8 time(s).\n"
            indents.0,
            indents.1,
            self.indent_step_noncall,
            item_name,
            count.to_string()
        );
    }
    fn notify_loopbody_start(&mut self, call_depth: usize) {
        let indents = self.get_indents(call_depth);
        decorator_write!(
            self,
            "{}{}{}{}\n",
            indents.0,
            indents.1,
            self.indent_step_call,
            LOOPBODY_NAME,
        ); // E.g."<thread_indent><indent>+-Loop body", "| | | | +-Loop body"
    }

    // NOTE: Reusing the default implementation of `notify_loopbody_end()` that does nothing.
}

impl ThreadSpecific for TreeLikeDecorator {
    fn set_thread_indent(&mut self, thread_indent: String) {
        self.common.set_thread_indent(thread_indent);
    }
}

impl WriterPossessor for TreeLikeDecorator {
    fn set_writer(&mut self, writer: Rc<RefCell<dyn Write>>) {
        self.common.set_writer(writer);
    }
}

impl LogDecorator for TreeLikeDecorator {}
