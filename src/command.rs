use std::io::Write;
use std::process::Command;
use std::{io, process, thread};
use std::{mem, panic};

pub(crate) fn output_and_write_streams<OW: Write + Send, EW: Write + Send>(
    command: &mut Command,
    stdout_write: OW,
    stderr_write: EW,
) -> io::Result<process::Output> {
    let mut stdout_buffer = Vec::new();
    let mut stderr_buffer = Vec::new();

    let mut stdout = tee(&mut stdout_buffer, stdout_write);
    let mut stderr = tee(&mut stderr_buffer, stderr_write);

    let mut child = command
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .spawn()?;

    thread::scope(|scope| {
        let stdout_thread = mem::take(&mut child.stdout).map(|mut child_stdout| {
            scope.spawn(move || std::io::copy(&mut child_stdout, &mut stdout))
        });
        let stderr_thread = mem::take(&mut child.stdout).map(|mut child_stderr| {
            scope.spawn(move || std::io::copy(&mut child_stderr, &mut stderr))
        });

        stdout_thread
            .map_or_else(
                || Ok(0),
                |handle| match handle.join() {
                    Ok(value) => value,
                    Err(err) => panic::resume_unwind(err),
                },
            )
            .and({
                stderr_thread.map_or_else(
                    || Ok(0),
                    |handle| match handle.join() {
                        Ok(value) => value,
                        Err(err) => panic::resume_unwind(err),
                    },
                )
            })
            .and_then(|_| child.wait())
    })
    .map(|status| process::Output {
        status,
        stdout: stdout_buffer,
        stderr: stderr_buffer,
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use std::process::Command;

    #[test]
    #[cfg(unix)]
    fn test_output_and_write_streams() {
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();

        let mut cmd = Command::new("echo");
        cmd.args(["-n", "Hello World!"]);

        let output = output_and_write_streams(&mut cmd, &mut stdout_buf, &mut stderr_buf).unwrap();

        assert_eq!(stdout_buf, "Hello World!".as_bytes());
        assert_eq!(stderr_buf, Vec::<u8>::new());

        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, "Hello World!".as_bytes());
        assert_eq!(output.stderr, Vec::<u8>::new());
    }
}

/// Constructs a writer that writes to two other writers. Similar to the UNIX `tee` command.
pub(crate) fn tee<A: io::Write, B: io::Write>(a: A, b: B) -> TeeWrite<A, B> {
    TeeWrite {
        inner_a: a,
        inner_b: b,
    }
}

/// A tee writer that was created with the [`tee`] function.
#[derive(Debug, Clone)]
pub(crate) struct TeeWrite<A: io::Write, B: io::Write> {
    inner_a: A,
    inner_b: B,
}

impl<A: io::Write, B: io::Write> io::Write for TeeWrite<A, B> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner_a.write_all(buf)?;
        self.inner_b.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner_a.flush()?;
        self.inner_b.flush()
    }
}
