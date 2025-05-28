use std::{cell::RefCell, io::{stdout, Write}, sync::Arc};

pub struct ThreadSharedWriter {
    writer: Box<dyn Write>,
}

impl ThreadSharedWriter {
    pub fn new(writer: Option<Box<dyn Write>>) -> Self {
        Self {
            writer: writer.unwrap_or(Box::new(stdout())),
        }
    }
}

impl Write for ThreadSharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub type ThreadSharedWriterPtr = Arc<RefCell<ThreadSharedWriter>>;
pub struct WriterAdapter {
    writer: ThreadSharedWriterPtr
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

