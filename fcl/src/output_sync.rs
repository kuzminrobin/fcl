use filedescriptor::{/*AsRawFileDescriptor,*/ FileDescriptor, StdioDescriptor};
use std::fs::File;
// use gag::BufferRedirect;
use std::io::{self /*, Error*/, Read, Write};
use tempfile::NamedTempFile;

pub struct StdOutputRedirector {
    original_std_output_fd: FileDescriptor,
    stdio: StdioDescriptor,
    tmpfile_for_fcl_to_read_from: File,
}

impl StdOutputRedirector {
    fn make(
        // file: &F,
        stdio: StdioDescriptor,
    ) -> io::Result<Self> {
        // if REDIRECT_FLAGS[stdio as usize].fetch_or(true, Ordering::Relaxed) {
        //     return Err(io::Error::new(
        //         io::ErrorKind::AlreadyExists,
        //         "Redirect already exists.",
        //     ));
        // }

        let tempfile = NamedTempFile::new()?;
        let tmpfile_for_os_to_write_to = tempfile.reopen()?; // The tmp file the OS will be redirecting the std output to.
        let tmpfile_for_fcl_to_read_from = tempfile.reopen()?; // The same tmp file the FCL will be reading {the OS-redirected output} from.

        // /// Buffer stderr.
        // pub fn stderr() -> io::Result<BufferRedirect> {
        //     let tempfile = NamedTempFile::new()?;
        //     let inner = tempfile.reopen()?;
        //     let outer = tempfile.reopen()?;
        //     let redir = Redirect::stderr(inner)?;
        //     Ok(BufferRedirect { redir, outer })
        // }

        let original_std_output_fd =
            FileDescriptor::redirect_stdio(&tmpfile_for_os_to_write_to, stdio)
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;

        // Dropping this will redirect stdio back to original std_fd
        Ok(StdOutputRedirector {
            original_std_output_fd,
            stdio,
            tmpfile_for_fcl_to_read_from,
        })
    }
    pub fn new_stdout() -> io::Result<Self> {
        Self::make(StdioDescriptor::Stderr)
    }
    pub fn new_stderr() -> io::Result<Self> {
        Self::make(StdioDescriptor::Stderr)
    }
    pub fn get_original_writer(&mut self) -> &mut dyn Write {
        &mut self.original_std_output_fd
    }
    pub fn get_buffer_reader(&mut self) -> &mut dyn Read {
        &mut self.tmpfile_for_fcl_to_read_from
    }
}

impl Drop for StdOutputRedirector {
    fn drop(&mut self) {
        // Flush the buffered data:
        let mut buf_content = String::new();
        let read_result = self.get_buffer_reader().read_to_string(&mut buf_content);
        let flush_error = match read_result {
            Ok(_size) => {
                let write_result = self.get_original_writer().write_all(buf_content.as_bytes());
                if let Err(e) = write_result {
                    Some(e)
                } else {
                    None
                }
            }
            Err(e) => Some(e),
        };
        // Report any flush error to the other std output stream
        // (i.e. report stderr flushing error to stdout, and vice versa):
        if let Some(error) = flush_error {
            let error_report_destination: &mut dyn Write = if self.stdio == StdioDescriptor::Stderr
            {
                &mut std::io::stdout()
            } else {
                &mut std::io::stderr()
            };
            let _ignore_error = write!(
                error_report_destination,
                "Warning: Failed to flush the buffered std output: '{}'",
                error
            );
        }

        // Cancel the std output redirection:
        let _ = FileDescriptor::redirect_stdio(&self.original_std_output_fd, self.stdio);
    }
}

// /// Hold output until dropped or read. On drop, the held output is sent to the stdout/stderr.
// ///
// /// Note: This will ignore IO errors when printing held output.
// pub struct Hold {
//     buf_redir: Option<BufferRedirect>,
//     is_stdout: bool,
// }

// impl Hold {
//     /// Hold stderr output.
//     pub fn stderr() -> io::Result<Hold> {
//         Ok(Hold {
//             buf_redir: Some(BufferRedirect::stderr()?),
//             is_stdout: false,
//         })
//     }

//     /// Hold stdout output.
//     pub fn stdout() -> io::Result<Hold> {
//         Ok(Hold {
//             buf_redir: Some(BufferRedirect::stdout()?),
//             is_stdout: true,
//         })
//     }

//     pub fn read_to_string(&mut self, to_str: &mut String) -> std::io::Result<usize> {
//         if let Some(from) = self.buf_redir.as_mut() {
//             from.read_to_string(to_str)
//         } else {
//             debug_assert!(false, "Internal Error: Unexpected lack of redirection");
//             Err(Error::from(io::ErrorKind::NotFound))
//         }
//     }
// }

// impl Drop for Hold {
//     fn drop(&mut self) {
//         if let Some(from) = self.buf_redir.take() {
//             let from = from.into_inner();

//             fn read_into<R: Read, W: Write>(mut from: R, mut to: W) {
//                 // TODO: use sendfile?
//                 let mut buf = [0u8; 4096];
//                 loop {
//                     // Ignore errors
//                     match from.read(&mut buf) {
//                         Ok(0) => break,
//                         Ok(size) => {
//                             if to.write_all(&buf[..size]).is_err() {
//                                 break;
//                             }
//                         }
//                         Err(_) => break,
//                     }
//                 }
//                 // Just in case...
//                 let _ = to.flush();
//             }

//             // let from = self.buf_redir.take().unwrap().into_inner();
//             // Ignore errors.
//             if self.is_stdout {
//                 let stdout = io::stdout();
//                 read_into(from, stdout.lock());
//             } else {
//                 let stderr = io::stderr();
//                 read_into(from, stderr.lock());
//             }
//         }
//     }
// }
