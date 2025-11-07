use std::{
    cell::RefCell,
    io::{Write, stdout},
    rc::Rc,
};

use code_commons::{CoderunNotifiable, ItemKind, RepeatCountCategory};

/// Trait to be implemented by the instances that handle any thread specifics, e.g. the thread indent.
/// 
/// The thread's output can be shifted by half of the console width
/// to visually separate the logs of the different threads into different "columns".
pub trait ThreadSpecific {
    /// Sets the thread's output indentation. E.g. if there are 2 threads,
    /// one thread's output can be logged in the left half of the console,
    /// and the other thread's output can be logged in the right half,
    /// or _indented_ by half of the console width.
    fn set_thread_indent(&mut self, thread_indent: String);
}

/// Trait to be implemented by the writer possessing types.
pub trait WriterPossessor {
    /// Replaces the writer.
    /// 
    /// Used by the tests to repace the default writer with the one that enables the analysis
    /// against the expected values.
    /// 
    /// To be used by the users to replace the default writer with the one that writes 
    /// * to a different place, such as a communication inteface (SPI, UART, JTAG, socket, pipe), 
    ///   circular buffer or file, etc.,
    /// * or in a different format, such as HTML, XML, JSON, etc.
    fn set_writer(&mut self, _writer: Rc<RefCell<dyn Write>>) {}
}

/// A supertrait of a log decorator combining a number of other traits.
pub trait LogDecorator: CoderunNotifiable + ThreadSpecific + WriterPossessor {}

/// A writer pointer type (a pointer to an instance implementing `Write`).
///
/// Is an `enum` rather than a single type in order to give a way for the tests and user
/// to substitute the log writer.
enum Writer {
    // TODO: Consider -> LogWriter
    /// The original writer pointer.
    Original(Box<dyn Write>),
    /// The substitute writer pointer.
    Substitute(Rc<RefCell<dyn Write>>),
}

/// Common part of the log decorators.
struct CommonDecorator {
    /// The pointer to a writer.
    writer: Writer,
    /// The thread indent for visual separation of different thread logs into different "columns".
    thread_indent: String,
}
impl CommonDecorator {
    /// Creates a new `CommonDecorator` with the optional writer passed as an argument.
    /// If `None` then `std::io::stdio::stdout()` is used as a writer.
    /// An emtpy string is used as a thread indent.
    fn new(writer: Option<Box<dyn Write>>) -> Self {
        Self {
            writer: Writer::Original(writer.unwrap_or(Box::new(stdout()))), // TODO: Move `stdout()` to a separate file of defaults.
            thread_indent: String::from(""),
        }
    }

    /// Replaces the thread indent with the one passed as an argument.
    fn set_thread_indent(&mut self, thread_indent: String) {
        self.thread_indent = thread_indent;
    }
    /// Returns the thread indent.
    fn get_thread_indent(&self) -> String {
        self.thread_indent.clone()
    }

    /// Replaces the writer pointer with the one passed as an argument.
    ///
    /// This method is used by the tests to replace the default writer.
    fn set_writer(&mut self, writer: Rc<RefCell<dyn Write>>) {
        self.writer = Writer::Substitute(writer);
    }
}

/// The decorators' "member macro" that writes to the writer of the `common: CommonDecorator` member.
/// # Parameters.
/// * `self` followed by
/// * the same arguments as for the `println!()`.
// By example of `println!()`.
macro_rules! decorator_write {  // TODO: Condsider renaming to `writer_write()`.
    ($self:ident, $($arg:tt)*) => {{
        let writer = match &mut $self.common.writer {
            Writer::Original(writer) => &mut **writer,
            Writer::Substitute(writer) => &mut *writer.borrow_mut(),
        };
        let _ignore_result = write!(writer, $($arg)*);  // TODO: Comment why ignore.
    }};
}

/// The decorator that logs the function calls in a code-like manner (as opposed to a tree-like manner).
///
/// For example,
/// ```txt
/// f() {
///     g() {}
///     // g() repeats 3 time(s).
///     h() {}
/// }
/// ```
pub struct CodeLikeDecorator {
    /// The part (of the decorator) common for multiple decorators.
    common: CommonDecorator,
    /// The indent step used for indenting functions with different call depth.
    ///
    /// Typically consists of a single Tab character (`'\t'`) or multiple spaces,
    /// but can be any `'static` string slice.
    indent_step: &'static str, // TODO (func_indent): Consider -> func_indent_step (to clearly distinguish from the thread_indent).
    /// Tells that the line end `'\n'` is pending after `f() {` before
    /// * loggign a nested call,
    /// * or a different thread's output,
    /// * or an `stdout` and/or `stderr` output by a user's code or panic hook.
    line_end_pending: bool, // '\n' pending after "f() {" before printing a nested call.
}

impl CodeLikeDecorator {
    /// Creates a new `CodeLikeDecorator` with the optional writer and optional indent step.
    /// * If the writer is `None` then uses the one in the `CommonDecorator`.
    /// * If the indent step is `None` then uses 2 spaces.
    pub fn new(writer: Option<Box<dyn Write>>, indent_step: Option<&'static str>) -> Self {
        Self {
            common: CommonDecorator::new(writer),
            indent_step: indent_step.unwrap_or(&"  "), // TODO: Move the default (&"  ") to a separate file of defaults.
            line_end_pending: false,
        }
    }
    /// Returns the indent string for the specified call depth. In other words, a string containing
    /// `self.indent_step` `call_depth` times.
    fn get_indent_string(&self, call_depth: usize) -> String {
        // TODO (func_indent): Consider -> get_func_indent_string.
        let mut indent_string = String::with_capacity(8);
        for _ in 0..call_depth {
            indent_string.push_str(self.indent_step);
        }
        indent_string
    }
    /// Returns a tuple of
    /// * the thread indent
    /// * and function indent string (that reflects the call depth).
    ///
    /// Those combined provide an overall indent for logging a line by the current thread.
    fn get_indents(&self, call_depth: usize) -> (String, String) {
        (
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
        )
    }
}

/// The string used to name loop bodies in the log.
const LOOPBODY_NAME: &str = &"Loop body"; // TODO: Move this deault to a separate file of defaults.

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
        let indents = self.get_indents(call_depth); // TODO: Consider -> `let (thread_indent, func_indent) =`.
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
        ); // Some() -> " -> Value".
        if !has_nested_calls && self.line_end_pending {
            decorator_write!(self, "}}{}\n", ret_val_str); // "}\n" or "} -> RetVal\n".
        } else {
            let indents = self.get_indents(call_depth); // TODO: Consider -> `let (thread_indent, func_indent) =`.
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
        kind: &ItemKind, // TODO: Consider -> item_kind or call_tree_item_kind.
        count: RepeatCountCategory,
    ) {
        let item_name = match kind {
            ItemKind::Call { name, .. } => format!("{}()", name),
            ItemKind::Loopbody { .. } => String::from(LOOPBODY_NAME), // "Loop body".
        };
        let indents = self.get_indents(call_depth); // TODO: Consider -> `let (thread_indent, func_indent) =`.
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
        let indents = self.get_indents(call_depth); // TODO: Consider -> `let (thread_indent, func_indent) =`.
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
        let indents = self.get_indents(call_depth); // TODO: Consider -> `let (thread_indent, func_indent) =`.
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

/// The decorator that logs the function calls in a tree-like manner (as opposed to a code-like manner).
///
/// For example,
/// ```ignore
/// // TreeLikeDecorator Log          Explanation in CodeLikeDecorator manner.
/// // -----------------------------------------------------------------------
/// +-f                               // f() {
/// | +-h                             //   h() {}
/// |   h repeats 99 time(s).         //   // h() repeats 99 time(s).
/// | +-i                             //   i() {
/// | | +-j                           //     j() {}
/// | |   j repeats 9 time(s).        //     // j() repeats 9 time(s).
///                                   //   } // i()
/// |   i repeats 100 time(s).        //   // i() repeats 100 time(s).
///   f repeats 2 time(s).            // } f() repeats 2 time(s).
/// +-Loop body                       // { // Loop body starts.
/// | +-k                             //   k() {}
/// | +-m                             //   m() {}
///                                   // } // Loop body ends.
///   Loop body repeats 3 time(s).    // // Loop body repeats 3 time(s).
/// ```
/// This decorator was added to test and make sure that the `CodeLikeDecorator`'s support 
/// has everything that is required to enable various other decorators.
#[rustfmt::skip]
pub struct TreeLikeDecorator {
    /// The part (of the decorator) common for multiple decorators.
    common: CommonDecorator,

    // TODO: These defaults to a separate file of defaults.
    /// The string that prepends the function call in the log. 
    /// For example, the fragment `+-` in the line `+-f`.
    indent_step_call   : &'static str,  // "+-"  f
    /// The string that prepends the repeat count in the log. 
    /// For example, the fragment of 2 spaces in the line 
    /// ```txt
    /// | |   j repeats 9 time(s)
    /// ```
    indent_step_noncall: &'static str,  // "  "  Repeats ..
    /// The string that shows the call depth, e.g. the multiple fragments `| ` in the line `| | +-j`.
    indent_step_parent : &'static str,  // "| "  Prepends multiple times those above.
}

impl TreeLikeDecorator {
    /// Creates a new `CodeLikeDecorator` with the optional writer and indent steps.
    /// * If the writer is `None` then uses the one in the `CommonDecorator`.
    /// * If the indent steps are `None` then uses
    ///   * `&"+-"` for `indent_step_call`,
    ///   * `&"  "` (2 spaces) for `indent_step_noncall`,
    ///   * `&"| "` for `indent_step_parent`,
    #[rustfmt::skip]
    pub fn new(writer: Option<Box<dyn Write>>,
        indent_step_call   : Option<&'static str>,
        indent_step_noncall: Option<&'static str>,
        indent_step_parent : Option<&'static str>,
    ) -> Self {
        Self {
            common: CommonDecorator::new(writer),

            // TODO: Move the defaults to a separate file of defaults.
            indent_step_call   : indent_step_call   .unwrap_or(&"+-"),
            indent_step_noncall: indent_step_noncall.unwrap_or(&"  "),
            indent_step_parent : indent_step_parent .unwrap_or(&"| ")
        }
    }
    /// Returns the indent string for the specified call depth. In other words, a string containing
    /// `self.indent_step_parent` `call_depth` times.
    fn get_indent_string(&self, call_depth: usize) -> String {
        let mut indent_string = String::with_capacity(8);
        for _ in 0..call_depth {
            indent_string.push_str(self.indent_step_parent);
        }
        indent_string
    }
    /// Returns a tuple of
    /// * the thread indent
    /// * and function indent string (that reflects the call depth).
    ///
    /// Those combined provide an indent for logging a line by the current thread.
    fn get_indents(&self, call_depth: usize) -> (String, String) {
        (
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
        )
    }
}

impl CoderunNotifiable for TreeLikeDecorator {
    fn notify_call(&mut self, call_depth: usize, name: &str, param_vals: &Option<String>) {
        let indents = self.get_indents(call_depth);  // TODO: Consider -> `let (thread_indent, func_indent) =`.
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
            ItemKind::Loopbody { .. } => String::from(LOOPBODY_NAME), // "Loop body",
        };
        let indents = self.get_indents(call_depth); // TODO: Consider -> `let (thread_indent, func_indent) =`.
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
        let indents = self.get_indents(call_depth); // TODO: Consider -> `let (thread_indent, func_indent) =`.
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
