use std::io::{self, Error, Read, Write};
use gag::BufferRedirect;
// use crate::BufferRedirect;

/// Hold output until dropped or read. On drop, the held output is sent to the stdout/stderr.
///
/// Note: This will ignore IO errors when printing held output.
pub struct Hold {
    buf_redir: Option<BufferRedirect>,
    is_stdout: bool,
}

impl Hold {
    /// Hold stderr output.
    pub fn stderr() -> io::Result<Hold> {
        Ok(Hold {
            buf_redir: Some(BufferRedirect::stderr()?),
            is_stdout: false,
        })
    }

    /// Hold stdout output.
    pub fn stdout() -> io::Result<Hold> {
        Ok(Hold {
            buf_redir: Some(BufferRedirect::stdout()?),
            is_stdout: true,
        })
    }

    // pub fn read_byte(&mut self) -> std::io::Result<Option<u8>> {
    //     if let Some(from) = self.buf_redir.as_mut() {
    //         let mut buf = [0u8];
    //         let result = from.read(&mut buf);
    //         match result {
    //             Ok(len) => 
    //                 if len == 1 {
    //                     return Ok(Some(buf[0]))
    //                 } else {
    //                     return Ok(None)
    //                 },
    //             Err(e) => return Err(e)
    //         }
    //     } else {
    //         debug_assert!(false, "Internal Error: Unexpected lack of redirection");
    //         Err(Error::from(io::ErrorKind::NotFound))
    //     }
    // }
    pub fn take(&mut self, to_str: &mut String) -> std::io::Result<usize> { // TODO: -> read_to_string
        // if let Some(from) = self.buf_redir.as_mut() {
        //     let mut buf = [0u8];
        //     let result = from.read(&mut buf);
        //     match result {
        //         Ok(len) => if len != 0 {

        //         }
        //     }
        // }

        if let Some(from) = self.buf_redir.as_mut() {
        // if let Some(from) = self.buf_redir.take() {
            from.read_to_string(to_str)
            // from.into_inner().read_to_string(to_str)
        } else {
            debug_assert!(false, "Internal Error: Unexpected lack of redirection");
            Err(Error::from(io::ErrorKind::NotFound))
        }
    }
}

impl Drop for Hold {
    fn drop(&mut self) {
        if let Some(from) = self.buf_redir.take() {
            let from = from.into_inner();

            fn read_into<R: Read, W: Write>(mut from: R, mut to: W) {
                // TODO: use sendfile?
                let mut buf = [0u8; 4096];
                loop {
                    // Ignore errors
                    match from.read(&mut buf) {
                        Ok(0) => break,
                        Ok(size) => {
                            if to.write_all(&buf[..size]).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                // Just in case...
                let _ = to.flush();
            }

            // let from = self.buf_redir.take().unwrap().into_inner();
            // Ignore errors.
            if self.is_stdout {
                let stdout = io::stdout();
                read_into(from, stdout.lock());
            } else {
                let stderr = io::stderr();
                read_into(from, stderr.lock());
            }
        }
    }
}