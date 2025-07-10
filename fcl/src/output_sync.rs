use filedescriptor::{FileDescriptor, StdioDescriptor};
use std::fs::File;
use std::io::{self, Read, Write};
use tempfile::NamedTempFile;

pub struct StdOutputRedirector {
    original_std_output_fd: FileDescriptor,
    stdio: StdioDescriptor,
    tmpfile_for_fcl_to_read_from: File,
}

impl StdOutputRedirector {
    fn make(stdio: StdioDescriptor) -> io::Result<Self> {
        let tempfile = NamedTempFile::new()?;
        let tmpfile_for_os_to_write_to = tempfile.reopen()?; // The tmp file handle the OS will be redirecting the std output to.
        let tmpfile_for_fcl_to_read_from = tempfile.reopen()?; // The handle of the same tmp file the FCL will be reading {the OS-redirected output} from.

        let original_std_output_fd =
            FileDescriptor::redirect_stdio(&tmpfile_for_os_to_write_to, stdio)
                .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;

        // Dropping this will redirect std output back to original_std_output_fd
        Ok(StdOutputRedirector {
            original_std_output_fd,
            stdio,
            tmpfile_for_fcl_to_read_from,
        })
    }
    pub fn new_stdout() -> io::Result<Self> {
        Self::make(StdioDescriptor::Stdout)
    }
    pub fn new_stderr() -> io::Result<Self> {
        Self::make(StdioDescriptor::Stderr)
    }
    pub fn get_original_writer(&mut self) -> &mut dyn Write {
        &mut self.original_std_output_fd
    }
    pub fn clone_original_writer(&self) -> filedescriptor::Result<File> {
        self.original_std_output_fd.as_file()
    }
    // pub fn get_original_writer(&mut self) -> &mut dyn Write {
    //     &mut self.original_std_output_fd
    // }
    pub fn get_buffer_reader(&mut self) -> &mut dyn Read {
        &mut self.tmpfile_for_fcl_to_read_from
    }
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
        // Report any flush error to the other std output stream
        // (i.e. report stderr error to stdout, and vice versa):
        if let Some(error) = flush_error {
            let error_report_destination: &mut dyn Write = if self.stdio == StdioDescriptor::Stderr
            {
                &mut std::io::stdout()
            } else {
                &mut std::io::stderr()
            };
            let _ignore_another_error = write!(
                error_report_destination,
                "Warning: Failed to flush the buffered std output: '{}'",
                error
            );
        }

        // Cancel the std output redirection (recover the original std output handle):
        let _ignore_error =
            FileDescriptor::redirect_stdio(&self.original_std_output_fd, self.stdio);
    }
}
