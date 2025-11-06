#![cfg(not(feature = "minimal_writer"))]

use filedescriptor::{FileDescriptor, StdioDescriptor};
use std::fs::File;
use std::io::{self, Read, Write};
use tempfile::NamedTempFile;

/// The standard output redirector - the container of the resources necessary for
/// * redirecting the std output (`stdout` or `stderr`) to a temporary file,
/// * reading the data that has been output to that file,
/// * and recovering the state that was before the redirection.
pub struct StdOutputRedirector {
    /// Stores the original std output file descriptor.
    original_std_output_fd: FileDescriptor,
    /// The std output specifier. Either the `stdout` or `stderr` is expected.
    stdio: StdioDescriptor,
    /// The file descriptor of the temporary file where the std output is redirected to,
    /// and the FCL can read the redirected output from for subsequent flushing
    /// at the right moment of the call log.
    tmpfile_for_fcl_to_read_from: File,
}

impl StdOutputRedirector {
    /// Creates an instance of the standard output redirector for the standard output (`stdout` or `stderr`)
    /// specified by the passed argument.
    /// 
    /// Returns the `std::io::Result<StdOutputRedirector>` with 
    /// * the redirector instance (in `Ok()`) 
    /// * or the redirection error `std::io::error::Error` (in `Err()`).
    fn make(stdio: StdioDescriptor) -> io::Result<Self> {
        // Create the temporary file:
        let tempfile = NamedTempFile::new()?;
        // Create the tmp file handle the OS will be redirecting the std output to.
        let tmpfile_for_os_to_write_to = tempfile.reopen()?;
        // Create the handle of the same tmp file the FCL will be reading (the OS-redirected output) from.
        let tmpfile_for_fcl_to_read_from = tempfile.reopen()?;

        // Tell the OS to redirect the specified std output (`stdout` or `stderr`)
        // to a tmp file and save the original std output file descriptor for subsequent recovery.
        let original_std_output_fd =
            FileDescriptor::redirect_stdio(&tmpfile_for_os_to_write_to, stdio)
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;

        // Create and return the instance of the standard output redirector.
        // Dropping it will redirect the std output back to `StdOutputRedirector::original_std_output_fd`.
        Ok(StdOutputRedirector {
            original_std_output_fd,
            stdio,
            tmpfile_for_fcl_to_read_from,
        })
    }
    /// Creates the `stdout` output redirector.
    /// 
    /// Returns the `std::io::Result<StdOutputRedirector>` with 
    /// * the redirector instance (in `Ok()`) 
    /// * or the redirection error `std::io::error::Error` (in `Err()`).
    pub fn new_stdout() -> io::Result<Self> {
        Self::make(StdioDescriptor::Stdout)
    }
    /// Creates the `stderr` output redirector.
    /// 
    /// Returns the `std::io::Result<StdOutputRedirector>` with 
    /// * the redirector instance (in `Ok()`) 
    /// * or the redirection error `std::io::error::Error` (in `Err()`).
    pub fn new_stderr() -> io::Result<Self> {
        Self::make(StdioDescriptor::Stderr)
    }
    /// Returns the reference to the `Write` trait of the original std output file descriptor.
    pub fn get_original_writer(&mut self) -> &mut dyn Write {
        &mut self.original_std_output_fd
    }
    /// Returns the creation result of the wrapper containting
    /// the cloned original std output file descriptor.
    pub fn clone_original_writer(&self) -> filedescriptor::Result<File> {
        self.original_std_output_fd.as_file()
    }
    /// Returns the reference to the `Read` trait of the descriptor for a temporary file
    /// used for std output redirection.
    pub fn get_buffer_reader(&mut self) -> &mut dyn Read {
        &mut self.tmpfile_for_fcl_to_read_from
    }
    /// Reads the content of the temporary file (the redirected user's std output) since last read,
    /// and writes, if any, to the original std output file descriptor.
    pub fn flush(&mut self) {
        let mut buf_content = String::new();
        let read_result = self.get_buffer_reader().read_to_string(&mut buf_content);
        if let Ok(size) = read_result
            && size != 0
        {
            let _ignore_error = self // redirector
                .get_original_writer()
                .write_all(buf_content.as_bytes());
        }
    }
}

impl Drop for StdOutputRedirector {
    /// Flushes the data from the temporary file to the original std output file descriptor
    /// and recovers the std output to a state before the redirection.
    fn drop(&mut self) {
        // Flush the buffered data:
        let mut buf_content = String::new();
        let read_result = self.get_buffer_reader().read_to_string(&mut buf_content);
        let flush_error = match read_result {
            Ok(size) => {
                if size != 0 {
                    let write_result = self.get_original_writer().write_all(buf_content.as_bytes());
                    if let Err(e) = write_result {
                        Some(e)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            Err(e) => Some(e),
        };

        // Prepare the error-reporting std output stream
        // (for reporting any error to the other std output stream,
        // i.e. report `stderr` flush error to `stdout`, and vice versa):
        let error_report_destination: &mut dyn Write = if self.stdio == StdioDescriptor::Stderr {
            &mut std::io::stdout()
        } else {
            &mut std::io::stderr()
        };

        // Report the flush error, if any:
        if let Some(error) = flush_error {
            let _ignore_another_error = write!(
                error_report_destination,
                "Warning: Failed to flush the buffered `{}` data: '{}'",
                if self.stdio == StdioDescriptor::Stdout {
                    "stderr"
                } else {
                    "stdout"
                },
                error
            );
        }

        // Cancel the std output redirection (recover the original std output handle).
        // Report the error, if any:
        // let _ignore_error =
        //     FileDescriptor::redirect_stdio(&self.original_std_output_fd, self.stdio);
        // // TODO: Consider reporting the error.
        if let Err(error) = FileDescriptor::redirect_stdio(&self.original_std_output_fd, self.stdio)
        {
            let _ignore_another_error = write!(
                error_report_destination,
                "Warning: Failed to revert the `{}` redirection: '{}'",
                if self.stdio == StdioDescriptor::Stdout {
                    "stderr"
                } else {
                    "stdout"
                },
                error
            );
        }
    }
}
