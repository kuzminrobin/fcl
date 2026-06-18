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
// TODO: Consider Thread[Shared]Writer -> LogWriter (since not only threads write/log to it but also the user code's std output)
// TODO: Consider reviewing FclWriter, WriterKind, and ThreadSharedWriter[::set_writer()] 
// against unifying/deduping, 
// preserving invariants (see below), and choosing a more optimal place for Box<dyn Write>.
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

pub type ThreadSharedWriterPtr = Arc<RefCell<ThreadSharedWriter>>; // TODO: Consider -> Arc<RefCell<dyn Write>>.

/// The adapter for the writer.
/// 
/// Is used per-thread in the environements with the writer access sinchronization, 
/// in particular the environments multi-threaded and/or having 
/// the user code's `stdout` and `stderr` output. 
pub struct WriterAdapter {
    /// The writer used by the adapter.
    writer: ThreadSharedWriterPtr,
}

impl WriterAdapter {
    /// Creates new `WriterAdapter` with the writer passed as an argument.
    pub fn new(writer: ThreadSharedWriterPtr) -> Self {
        Self { writer }
    }
}

impl Write for WriterAdapter {
    /// Forwards the call to the writer's `Write::write()`.
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.borrow_mut().write(buf)
    }
    /// Forwards the call to the writer's `Write::flush()`.`
    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.borrow_mut().flush()
    }
}

/// The global writer shared by all the threads. It participates in synchronization 
/// with the user code's `stdout`/`stderr` output.
// TODO: Consider THREAD_SHARED_WRITER -> COMMON_WRITER since it is shared not only by the threads 
// but also by the user code's stdout and astderr output.
pub static mut THREAD_SHARED_WRITER: LazyLock<ThreadSharedWriterPtr> = LazyLock::new(|| {
    Arc::new(RefCell::new(ThreadSharedWriter::new(Some(
        FclWriter::Stdout, // TODO: Consider either `None` or 
        // fully creating the writer outside of ThreadSharedWriter and passing to ThreadSharedWriter::new().
        // Such that the ThreadSharedWriter works with whatever `dyn Write` provided from outside.
    ))))
});
