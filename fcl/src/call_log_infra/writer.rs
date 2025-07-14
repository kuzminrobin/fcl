use std::{
    cell::RefCell,
    io::{stderr, stdout, Write},
    sync::{Arc, LazyLock},
};

pub enum FclWriter {
    Stdout,
    Stderr,
    Other(Box<dyn Write>),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WriterKind {
    Stdout,
    Stderr,
    Other,
}
pub struct ThreadSharedWriter {
    writer_kind: WriterKind,
    writer: Box<dyn Write>,
    override_writer: Option<std::fs::File>,
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
    pub fn get_writer_kind(&self) -> WriterKind {
        self.writer_kind
    }
    pub fn set_writer(&mut self, file: std::fs::File) {
        self.override_writer = Some(file);
    }
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

// Global data shared by all the threads:
pub static mut THREAD_SHARED_WRITER: LazyLock<ThreadSharedWriterPtr> = LazyLock::new(|| {
    Arc::new(RefCell::new(ThreadSharedWriter::new(Some(
        /*crate::writer::*/FclWriter::Stdout,
    ))))
});
