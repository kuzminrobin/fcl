use std::{
    cell::RefCell,
    io::{Write, stderr, stdout},
    sync::{Arc, LazyLock},
};

/// Specifies the instance used by the FCL for logging.
pub enum FclWriter {
    /// The FCL uses `stdout` for logging.
    Stdout,
    /// The FCL uses `stderr` for logging.
    Stderr,
    /// The FCL uses for logging some other instance implementing `Write`.
    Other(Box<dyn Write>),
}

// TODO: Somewhat duplicates the FclWriter. Consider deduping.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WriterKind {
    Stdout,
    Stderr,
    Other,
}

// TODO: Consider removing `Shared` from the type name. There is nothing sharing-specific. Nothing
// prevents different threads from having separate instances of this Writer.
// TODO: Consider reviewing FclWriter, WriterKind, and ThreadSharedWriter[::set_writer()] against unifying/deduping, 
// preserving invariants, and choosing a more optimal place for Box<dyn Write>.
pub struct ThreadSharedWriter {
    writer_kind: WriterKind,
    writer: Box<dyn Write>,
    override_writer: Option<std::fs::File>, // TODO: Consider Option<dyn Write> if there's nothing std::fs::File-specific.
    // TODO: Consider merging the writer and override_writer. I don't see why they both have to live at the same time.
}

impl ThreadSharedWriter {
    /// Creates the new `ThreadSharedWriter` with the writer passed as an argument.
    /// If the argument is `None` then the `std::io::stdio::stdout()` is used.
    pub fn new(fcl_writer: Option<FclWriter>) -> Self {
        let (writer, writer_kind): (Box<dyn Write>, WriterKind) = match fcl_writer {
            None => (Box::new(stdout()), WriterKind::Stdout),   // TODO: Consider extracting `stdout()` to a file of defaults.
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
        self.override_writer = Some(file); // TODO: What about updating `writer_kind`? Isn't that update 
        // required to preserve the invariant of `writer_kind: WriterKind::Other`?
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

pub type ThreadSharedWriterPtr = Arc<RefCell<ThreadSharedWriter>>; // TODO: Consider -> Arc<RefCell<Write>>.

/// Threads' personal adapter for the thread-shared writer.
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
        FclWriter::Stdout, // TODO: Consider either `None` or 
        // fully creating the writer outside of ThreadSharedWriter and passing to ThreadSharedWriter::new().
        // Such that the ThreadSharedWriter works with whatever `Write` provided from outside.
    ))))
});
