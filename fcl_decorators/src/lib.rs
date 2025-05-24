// // code_run_decorator

// /*
// TODO:
// Consider splitting OldCodeRunDecorator into
//     pub trait CodeRunDecorator
//     implementors
//         pub CallLikeDecorator, which is already done
//         pub TreeLikeDecorator, practice for a reader
// */
use std::{
    cell::RefCell,
    // collections::{HashMap, HashSet},
    io::{Write, stdout},
    // ptr::null,
    // rc::Rc,
    sync::Arc,
    thread,
};

use fcl_traits::{
    CalleeName, CodeRunDecorator, CoderunNotifiable, CoderunThreadSpecificNotifyable,
    Flushable, /*, Flusher */
    ThreadSpecifics,
};
use parking_lot::ReentrantMutex;

// use fcl_traits::{CalleeName, CoderunNotifiable};

macro_rules! CLOSURE_NAME_FORMAT {
    () => {
        "closure{{{},{}:{},{}}}"
    }; // "closure{112,9:116,34}"
}

pub type ThreadAwareWriterType = Arc<ReentrantMutex<RefCell<ThreadAwareWriter>>>;
// pub type ThreadAwareWriterType<'a> = Arc<ReentrantMutex<RefCell<ThreadAwareWriter<'a>>>>;

pub struct ThreadAwareWriter /*<'a>*/ {
    writer: Box<dyn Write>,
    last_flushable: Option<(thread::ThreadId, *mut dyn Flushable)>,
    // last_flushable: Option<(thread::ThreadId, &'static mut dyn FnMut())>,
    // last_flushable: Option<&'a mut dyn Flushable>,
    // last_flushable: *const usize,
    // last_flushable: *const dyn Flushable,
    // last_output_thread_id: Option<thread::ThreadId>,
    // flushables: HashSet<*mut dyn Flushable>,
    // // flushables: HashSet<Rc<RefCell<dyn Flushable>>>,
    // // flushables: HashMap<thread::ThreadId, Rc<RefCell<dyn Flushable>>>,
    flush_in_progress: bool,
}

impl ThreadAwareWriter {
    // impl<'a> ThreadAwareWriter<'a> {
    pub fn new(writer: Option<Box<dyn Write>>) -> Self {
        Self {
            writer: writer.unwrap_or(Box::new(stdout())),
            last_flushable: None,
            // last_flushable: Option<FnMut>,
            // last_flushable: None,
            // last_flushable: null(), //(null::<usize>()) as *const dyn Flushable,
            // last_output_thread_id: None,
            // flushables: HashSet::new(),
            // // flushables: HashMap::new(),
            flush_in_progress: false,
        }
    }
    // pub fn register_flushable(
    //     &mut self,
    //     _thread_id: thread::ThreadId,    // TODO: Make sure still is applicable.
    //     flushable: &dyn Flushable
    //     // flushable: Rc<RefCell<dyn Flushable>>,
    // ) {
    //     let was_inserted = self.flushables.insert(flushable as *const dyn Flushable as *mut dyn Flushable);
    //     // let result = self.flushables.insert(thread_id, flushable);
    //     debug_assert!(
    //         was_inserted,
    //         // result.is_none(),
    //         "Thread registers multiple times which is unexpected"
    //     );
    // }

    pub fn write(&mut self, flushable: &dyn Flushable, output: &str) {
        // pub fn write(&mut self, flushable: &'static mut dyn FnMut(), output: &str) {
        // pub fn write(&mut self, flushable: &mut dyn Flushable, output: &str) {
        // pub fn write(&mut self, thread_id: thread::ThreadId, output: &str) {
        if !self.flush_in_progress {
            if let Some(last_flushable) = self.last_flushable.as_mut()
                && last_flushable.0 != thread::current().id()
            {
                // let flushable_ptr = flushable as *const dyn Flushable as *const usize;
                // if flushable_ptr != self.last_flushable {
                // if let Some(last_thread_id) = self.last_output_thread_id``
                //     && last_thread_id != thread_id
                self.flush_in_progress = true;
                unsafe { (*last_flushable.1).flush() }; // Flush
                // (self.last_flushable as *mut usize as *mut dyn Flushable as &mut dyn Flushable).flush();
                self.flush_in_progress = false;
                // // The thread context has switched from the last_thread_id to thread_id.
                // // Flush the last_thread_id output cache:
                // if let Some(flushable) = self.flushables.get(&last_thread_id) {
                //     self.flush_in_progress = true;
                //     flushable.borrow_mut().flush(); // This will cause the re-enter to `self.write()`.
                //     self.flush_in_progress = false;
                // } else {
                //     debug_assert!(
                //         false,
                //         "Internal Error: Flushable for the last thread ID is not found"
                //     )
                // }
            } // else (the first-most write or the thread context has not switched) just output.

            self.last_flushable = Some((
                thread::current().id(),
                flushable as *const dyn Flushable as *mut dyn Flushable,
            ));
            // self.last_flushable = flushable_ptr;
            // self.last_output_thread_id = Some(thread_id);
        } // else (flush_in_progress) just output.

        // Output:
        if let Err(error) = self.writer.write_all(output.as_bytes()) {
            panic!(
                "Logging write failed unexpectedly (Output socket is closed? Output file system is full?). Error: '{}'.\nPanicing voluntarily.",
                error
            );
        }
    }
}

// TODO: Move privates down, publics up.
struct CommonDecorator {
//struct CommonDecorator<'a> {
    writer: ThreadAwareWriterType,
    // writer: Box<dyn Write>,
    thread_indent: &'static str,
    flushable: Option<*mut dyn Flushable>,
    // reused_lock: Option<parking_lot::lock_api::ReentrantMutexGuard<
    //     'a,
    //     parking_lot::RawMutex,
    //     parking_lot::RawThreadId,
    //     RefCell<ThreadAwareWriter>,
    // >>,
    // reused_borrow: Option<RefMut<'a, ThreadAwareWriter>>
}

// impl Flusher for CommonDecorator {
//     fn register_flushable(&mut self, flushable: &dyn Flushable) {
//         self.writer.lock().borrow_mut().register_flushable(thread::current().id(), flushable);
//     }
// }

impl CommonDecorator {
// impl<'a> CommonDecorator<'a> {
    // fn get_flusher(&mut self) -> &mut dyn Flusher {
    //     self
    // }

    fn new(writer: ThreadAwareWriterType) -> Self {
        // fn new(writer: Option<Box<dyn Write>> /* , thread_indent: Option<&'static str>*/) -> Self {
        // TODO: Consider `indent_step: Option<&'static str>`.
        Self {
            writer,
            // writer: writer.unwrap_or(Box::new(stdout())),
            thread_indent: &"",
            // thread_indent: thread_indent.unwrap_or(&""),
            flushable: None,
            // reused_borrow: None,
        }
    }

    fn write(&mut self, output: &str) {
        self.writer.lock().borrow_mut().write(self, output);
    }

    // fn write<'b>(&'a mut self, output: &str)
    // where
    //     'a: 'b,
    //     'b: 'a,
    // {
    //     let lock: parking_lot::lock_api::ReentrantMutexGuard<
    //         '_,
    //         parking_lot::RawMutex,
    //         parking_lot::RawThreadId,
    //         RefCell<ThreadAwareWriter>,
    //     > = self.writer.lock();
    //     let borrow_mut: Result<RefMut<'b, _>, std::cell::BorrowMutError> = lock.try_borrow_mut();
    //     if let Ok(acquired_borrow) = borrow_mut {
    //         self.reused_borrow = Some(acquired_borrow);
    //         self.reused_borrow.as_mut().unwrap().write(self, &output);
    //         self.reused_borrow = None;
    //     } else {
    //         self.reused_borrow.as_mut().unwrap().write(self, &output);
    //     }
    // }

    fn set_flushable(&mut self, flushable: &dyn Flushable) {
        self.flushable = Some(flushable as *const dyn Flushable as *mut dyn Flushable);
    }

    // fn flush(&mut self) {
    //     // println!("!!! flush() !!!");
    //     if let Some(flushable) = self.flushable {
    //         unsafe {(&mut *flushable).flush() }
    //         // unsafe { (*flushable).flush() }
    //     }
    // }

    fn get_callee_name_string(name: &CalleeName) -> String {
        match name {
            CalleeName::Function(slice) => String::from(*slice),
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

    //     // fn write(&mut self, output: Arguments) -> Result<()> {
    //     //     write!(self.writer, "{:.*}", 2, 1.234567)?;
    //     //     self.writer.write_fmt(output)?;
    //     //     Ok(())
    //     // }
}

impl Flushable for CommonDecorator {
// impl Flushable for CommonDecorator<'_> {
    fn flush(&mut self) {
        // println!("!!! flush() !!!");
        if let Some(flushable) = self.flushable {
            unsafe { (&mut *flushable).flush() }
            // unsafe { (*flushable).flush() }
        }
    }
}

// // By example of `println!()`.
// macro_rules! decorator_write {
//     ($self:ident, $($arg:tt)*) => {{
//         let _result = write!($self.common.writer, $($arg)*);
//         // $crate::io::_print($crate::format_args_nl!($($arg)*));
//     }};
// }

pub struct CodeLikeDecorator {
    common: CommonDecorator,
    indent_step: &'static str,
    line_end_pending: bool, // '\n' pending after "f() {" before printing a child call.
}

impl CodeLikeDecorator {
    pub fn new(
        writer: ThreadAwareWriterType,
        // writer: Option<Box<dyn Write>>,
        indent_step: Option<&'static str>,
        // thread_indent: Option<&'static str>,
    ) -> Self {
        // TODO: Consider `indent_step: Option<&'static str>`.
        Self {
            common: CommonDecorator::new(writer /*, thread_indent */),
            indent_step: indent_step.unwrap_or(&"  "),
            // indent_step, // TODO: Consider `= indent_step.unwrap_or(&"  ")`.
            line_end_pending: false,
        }
    }
    // fn flush(&mut self) {}
}

impl CodeRunDecorator for CodeLikeDecorator {
    fn get_indent_string(&self, call_depth: usize) -> String {
        let mut indent_string = String::with_capacity(8);
        for _ in 0..call_depth {
            indent_string.push_str(self.indent_step);
        }
        indent_string
    }
}

// impl Flushable for CodeLikeDecorator {
//     fn flush(&mut self) {
//         // println!("!!! flush() !!!");
//         self.common.flush();
//     }
// }

impl CoderunNotifiable for CodeLikeDecorator {
    fn set_flushable(&mut self, flushable: &dyn Flushable) {
        self.common.set_flushable(flushable);
    }

    // fn get_flusher(&self) -> &mut dyn fcl_traits::Flusher {
    //     self.common.get_flusher()
    // }

    fn notify_call(&mut self, call_depth: usize, name: &CalleeName) {
        // '\n' after "parent() {" before printing the first sibling call.
        if self.line_end_pending {
            self.common.write("\n");
            // self.common.writer.lock().borrow_mut().write(self, "\n");
            // .write(&mut CodeLikeDecorator::flush, "\n");
            // .write(thread::current().id(), "\n");
            // decorator_write!(self, "\n");
        }

        // "<thread_indent><indent>sibling() {"
        let output_string = format!(
            "{}{}{}() {{",
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
            CommonDecorator::get_callee_name_string(name)
        );

        self.common.write(&output_string);
        // self.common
        //     .writer
        //     .lock()
        //     .borrow_mut()
        //     .write(self, &output_string);
        // .write(CodeLikeDecorator::flush, &output_string);
        // .write(thread::current().id(), &output_string);

        // decorator_write!(
        //     self,
        //     "{}{}{}() {{",
        //     self.common.get_thread_indent(),
        //     self.get_indent_string(call_depth),
        //     CommonDecorator::get_callee_name_string(name)
        // ); // "<thread_indent><indent>sibling() {"

        // TODO: Consider `self.line_end_pending` during flushing:
        self.line_end_pending = true; // '\n' pending. Won't be printed if there will be no child calls (immediate "}\n" will follow).
    }

    fn notify_return(&mut self, call_depth: usize, name: &CalleeName, has_nested_calls: bool) {
        if !has_nested_calls {
            // "}\n"
            self.common.write("}\n");
            // self.common.writer.lock().borrow_mut().write(self, "}\n");
            // .write(CodeLikeDecorator::flush, "}}\n");
            // .write(thread::current().id(), "}}\n");
            // decorator_write!(self, "}}\n");
        } else {
            // "<thread_indent><indent>} // sibling().\n".
            let output_string = format!(
                "{}{}}} // {}().\n",
                self.common.get_thread_indent(),
                self.get_indent_string(call_depth),
                CommonDecorator::get_callee_name_string(name)
            );
            self.common.write(&output_string);
            // self.common
            //     .writer
            //     .lock()
            //     .borrow_mut()
            //     .write(self, &output_string);
            // .write(CodeLikeDecorator::flush, &output_string);
            // .write(thread::current().id(), &output_string);
            // decorator_write!(
            //     self,
            //     "{}{}}} // {}().\n",
            //     self.common.get_thread_indent(),
            //     self.get_indent_string(call_depth),
            //     CommonDecorator::get_callee_name_string(name)
            // );
        }
        self.line_end_pending = false;
    }

    fn notify_repeat_count(&mut self, call_depth: usize, name: &CalleeName, count: usize) {
        // "<thread_indent><indent>// sibling() repeats 8 time(s).\n"
        let output_string = format!(
            "{}{}// {}() repeats {} time(s).\n",
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
            CommonDecorator::get_callee_name_string(name),
            count
        );
        self.common.write(&output_string);
        // self.common
        //     .writer
        //     .lock()
        //     .borrow_mut()
        //     .write(self, &output_string);
        // .write(CodeLikeDecorator::flush, &output_string);
        // .write(thread::current().id(), &output_string);

        // decorator_write!(
        //     self,
        //     "{}{}// {}() repeats {} time(s).\n",
        //     self.common.get_thread_indent(),
        //     self.get_indent_string(call_depth),
        //     CommonDecorator::get_callee_name_string(name),
        //     count
        // );
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
// | | h repeats 99 time(s)          //   // h() repeats 99 time(s).
// | +-i                             //   i() {
// | | +-j                           //     j() {}
// | | | j repeats 9 time(s)         //     // j() repeats 9 time(s).
// | | +-k                           //     k() {}
// | | | k repeats 5 time(s)         //     // k() repeats 5 time(s).
// |                                 //   } // i()
// | | i repeats 100 time(s)         //   // i() repeats 100 time(s).
#[rustfmt::skip]
pub struct TreeLikeDecorator {
    common: CommonDecorator,
    indent_step_call   : &'static str,  // "+-"  f() {}
    indent_step_noncall: &'static str,  // "  "  Repeats ..
    indent_step_parent : &'static str,  // "| "  Prepends multiple times those above.
}

impl TreeLikeDecorator {
    #[rustfmt::skip]
    pub fn new(
        writer: ThreadAwareWriterType,
        // writer: Option<Box<dyn Write>>,
        indent_step_call   : Option<&'static str>,
        indent_step_noncall: Option<&'static str>,
        indent_step_parent : Option<&'static str>,
        // thread_indent: Option<&'static str>
    ) -> Self 
    {
        Self {
            common: CommonDecorator::new(writer/*, thread_indent */),
            indent_step_call   : indent_step_call   .unwrap_or(&"+-"),
            indent_step_noncall: indent_step_noncall.unwrap_or(&"  "),
            indent_step_parent : indent_step_parent .unwrap_or(&"| ")
        }
    }
}

// TODO: Consider moving to the parent class.
impl CodeRunDecorator for TreeLikeDecorator {
    fn get_indent_string(&self, call_depth: usize) -> String {
        let mut indent_string = String::with_capacity(8);
        for _ in 0..call_depth {
            indent_string.push_str(self.indent_step_parent);
        }
        indent_string
    }
}

// impl Flushable for TreeLikeDecorator {
//     fn flush(&mut self) {
//         self.common.flush();
//     }
// }

impl CoderunNotifiable for TreeLikeDecorator {
    fn set_flushable(&mut self, flushable: &dyn Flushable) {
        self.common.set_flushable(flushable);
    }

    fn notify_call(&mut self, call_depth: usize, name: &CalleeName) {
        // "<indent>+-sibling", "| | | | +-sibling"
        let output_string = format!(
            "{}{}{}{}\n",
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
            self.indent_step_call,
            CommonDecorator::get_callee_name_string(name)
        );

        self.common.write(&output_string);
        // self.common
        //     .writer
        //     .lock()
        //     .borrow_mut()
        //     .write(self, &output_string);
        // .write(thread::current().id(), &output_string);

        // decorator_write!(
        //     self,
        //     "{}{}{}{}\n",
        //     self.common.get_thread_indent(),
        //     self.get_indent_string(call_depth),
        //     self.indent_step_call,
        //     CommonDecorator::get_callee_name_string(name)
        // );
    }

    // NOTE: Reusing the default behavior of `notify_return()` that does nothing.

    fn notify_repeat_count(&mut self, call_depth: usize, name: &CalleeName, count: usize) {
        // "<indent> sibling repeats 8 time(s).\n"
        let output_string = format!(
            "{}{}{}{} repeats {} time(s).\n",
            self.common.get_thread_indent(),
            self.get_indent_string(call_depth),
            self.indent_step_noncall,
            CommonDecorator::get_callee_name_string(name),
            count
        );
        self.common.write(&output_string);
        // self.common
        //     .writer
        //     .lock()
        //     .borrow_mut()
        //     .write(self, &output_string);
        // .write(thread::current().id(), &output_string);

        // decorator_write!(
        //     self,
        //     "{}{}{}{} repeats {} time(s).\n",
        //     self.common.get_thread_indent(),
        //     self.get_indent_string(call_depth),
        //     self.indent_step_noncall,
        //     CommonDecorator::get_callee_name_string(name),
        //     count
        // );
    }
}

impl ThreadSpecifics for TreeLikeDecorator {
    fn set_thread_indent(&mut self, thread_indent: &'static str) {
        self.common.set_thread_indent(thread_indent);
    }
}

impl CoderunThreadSpecificNotifyable for TreeLikeDecorator {}
