use std::{
    cell::RefCell,
    io::{Write, stderr, stdout},
    sync::Arc,
};

pub enum FclWriter {
    Stdout,
    Stderr,
    Other(Box<dyn Write>),
}

#[derive(Clone, Copy, PartialEq)]
pub enum WriterKind {
    Stdout,
    Stderr,
    Other,
}
pub struct ThreadSharedWriter {
    writer_kind: WriterKind,
    writer: Box<dyn Write>,
    override_writer: Option<std::fs::File>,
    // writer: std::rc::Rc<dyn Write>,
    // initial_writer: Writer
}

impl ThreadSharedWriter {
    pub fn new(fcl_writer: Option<FclWriter>) -> Self {
        let (writer, writer_kind): (Box<dyn Write>, WriterKind) = match fcl_writer {
            None => (Box::new(stdout()), WriterKind::Stdout),
            Some(writer) => match writer {
                FclWriter::Stdout => (Box::new(stdout()), WriterKind::Stdout),
                FclWriter::Stderr => (Box::new(stderr()), WriterKind::Stderr),
                FclWriter::Other(non_std_writer) => (non_std_writer, WriterKind::Other),
            },
        };
        Self {
            writer_kind,
            writer,
            override_writer: None,
        }
    }
    // pub fn new(writer: Option<Box<dyn Write>>) -> Self {
    //     // if writer == Some(Box::new(std::io::stdout())) {
    //         let initial_writer = match writer {
    //             Some(writer) => {
    //                 let writer_typeid = writer.type_id();
    //                 let stdout_typeid = (Box::new(stdout()) as Box<dyn Write>).type_id();
    //                 let stderr_typeid = (Box::new(stderr()) as Box<dyn Write>).type_id();
    //                 if writer_typeid == stdout_typeid { // *std::ptr::addr_eq(raw_writer, raw_stdout) {*/ raw_writer == raw_stdout {
    //                     InitialOutput::Stdout
    //                 } else if writer_typeid == stderr_typeid { // std::ptr::addr_eq(raw_writer, raw_stderr) { // raw_writer == raw_stderr {
    //                 // } else if std::ptr::addr_eq(writer, &stderr()/* as &dyn Write */ as *const dyn Write) { // writer == &stderr()/* as &dyn Write */ as *const dyn Write {
    //                     InitialOutput::Stderr
    //                 } else {
    //                     InitialOutput::Other
    //                 }
    //                 // let a = if writer_typeid == stdout_typeid {
    //                 //     true
    //                 // } else {
    //                 //     false
    //                 // };

    //                 // let raw_writer = Box::into_raw(writer);
    //                 // let raw_stdout = Box::into_raw(Box::new(stdout()) as Box<dyn Write>);
    //                 // let raw_stderr = Box::into_raw(Box::new(stderr()) as Box<dyn Write>);
    //                 // // let writer = &**writer as *const dyn Write;
    //                 // // if &**writer == &stdout() as &dyn Write {//&stdout() as &dyn Write as *const dyn Write {
    //                 // // if std::ptr::addr_eq(writer, &stdout() as &dyn Write as *const dyn Write) {// writer == &stdout() as &dyn Write as *const dyn Write {
    //                 // if /*std::ptr::addr_eq(raw_writer, raw_stdout) {*/ raw_writer == raw_stdout {
    //                 //     InitialOutput::Stdout
    //                 // } else if std::ptr::addr_eq(raw_writer, raw_stderr) { // raw_writer == raw_stderr {
    //                 // // } else if std::ptr::addr_eq(writer, &stderr()/* as &dyn Write */ as *const dyn Write) { // writer == &stderr()/* as &dyn Write */ as *const dyn Write {
    //                 //     InitialOutput::Stderr
    //                 // } else {
    //                 //     InitialOutput::Other
    //                 // }
    //             }
    //             None => InitialOutput::Stdout
    //         };
    //     // }
    //     Self {
    //         writer: Box::new(stdout()), // writer.unwrap_or(Box::new(stdout())),
    //         initial_writer
    //     }
    // }
    pub fn get_writer_kind(&self) -> WriterKind {
        self.writer_kind
    }
    pub fn set_writer(&mut self, file: std::fs::File) {
        self.override_writer = Some(file);
        // self.writer = std::rc::Rc::new(self.other.as_ref().unwrap())
        // self.writer = Box::new(self.other.as_ref().unwrap())
    }
    // pub fn set_writer(&mut self, writer: Box<dyn Write>) {
    //     self.writer = writer;
    // }
}

impl Write for ThreadSharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(override_writer) = &mut self.override_writer {
            override_writer.write(buf)
        } else {
            self.writer.write(buf)
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(override_writer) = &mut self.override_writer {
            override_writer.flush()
        } else {
            self.writer.flush()
        }
    }
}

pub type ThreadSharedWriterPtr = Arc<RefCell<ThreadSharedWriter>>;
pub struct WriterAdapter {
    writer: ThreadSharedWriterPtr,
}

impl WriterAdapter {
    pub fn new(writer: ThreadSharedWriterPtr) -> Self {
        Self { writer }
    }
}

impl Write for WriterAdapter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.borrow_mut().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.borrow_mut().flush()
    }
}
