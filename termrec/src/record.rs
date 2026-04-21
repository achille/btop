use anyhow::{Context, Result, bail};
use nix::errno::Errno;
use nix::fcntl::{OFlag, fcntl, FcntlArg};
use nix::libc;
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::pty::openpty;
use nix::sys::signal::{self, Signal};
use nix::sys::wait::{WaitPidFlag, waitpid};
use nix::unistd::{ForkResult, Pid, dup2, execvp, fork, read, setsid, write};
use std::ffi::CString;
use std::io::{self, BufWriter, Write};
use std::os::fd::{AsRawFd, BorrowedFd, OwnedFd};
use std::path::Path;
use std::time::Instant;

use crate::cast::{CastEvent, CastHeader, CastWriter};

/// Record a terminal session to an asciicast v2 file.
pub fn record(command: &[String], output_path: &Path) -> Result<()> {
    // Get current terminal size.
    let (cols, rows) = crossterm::terminal::size().context("failed to get terminal size")?;

    let win_size = nix::pty::Winsize {
        ws_row: rows,
        ws_col: cols,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };

    // Open PTY pair.
    let pty = openpty(Some(&win_size), None).context("openpty failed")?;
    let master_fd = pty.master;
    let slave_fd = pty.slave;

    // Fork.
    match unsafe { fork() }.context("fork failed")? {
        ForkResult::Child => {
            // Child: set up slave PTY as stdin/stdout/stderr, exec command.
            drop(master_fd);
            child_exec(slave_fd, command);
        }
        ForkResult::Parent { child } => {
            // Parent: close slave, relay I/O.
            drop(slave_fd);
            parent_record(master_fd, child, output_path, cols as u32, rows as u32)?;
        }
    }

    Ok(())
}

/// Child process: set up PTY slave and exec command.
fn child_exec(slave_fd: OwnedFd, command: &[String]) -> ! {
    // Create new session.
    setsid().ok();

    // Set controlling terminal.
    unsafe {
        libc::ioctl(slave_fd.as_raw_fd(), libc::TIOCSCTTY, 0);
    }

    // Dup slave to stdin/stdout/stderr.
    let raw = slave_fd.as_raw_fd();
    dup2(raw, 0).ok();
    dup2(raw, 1).ok();
    dup2(raw, 2).ok();
    if raw > 2 {
        drop(slave_fd);
    }

    // Exec.
    let cmd = if command.is_empty() {
        vec!["btop".to_string()]
    } else {
        command.to_vec()
    };

    let c_cmd: Vec<CString> = cmd
        .iter()
        .map(|s| CString::new(s.as_str()).unwrap())
        .collect();

    execvp(&c_cmd[0], &c_cmd).expect("execvp failed");
    unreachable!()
}

/// Parent process: relay stdin↔master, capture master→cast file.
fn parent_record(
    master_fd: OwnedFd,
    child: Pid,
    output_path: &Path,
    cols: u32,
    rows: u32,
) -> Result<()> {
    let file = std::fs::File::create(output_path)
        .with_context(|| format!("failed to create {}", output_path.display()))?;
    let buf_writer = BufWriter::new(file);

    let header = CastHeader::new(cols, rows);
    let mut cast_writer = CastWriter::new(buf_writer, &header)?;

    // Put terminal in raw mode.
    let _raw_guard = crossterm::terminal::enable_raw_mode();

    // Make master fd non-blocking.
    set_nonblocking(&master_fd)?;

    // Note: we do NOT set stdin non-blocking because on a TTY, stdin and stdout
    // share the same file description. Making stdin non-blocking would also make
    // stdout non-blocking, causing write_all() to fail with EAGAIN.
    // poll() already ensures we only read stdin when data is available.
    let stdin_fd = io::stdin().as_raw_fd();

    let start = Instant::now();
    let mut buf = [0u8; 8192];
    let mut child_exited = false;

    loop {
        let master_raw = master_fd.as_raw_fd();
        let mut pollfds = [
            PollFd::new(unsafe { BorrowedFd::borrow_raw(master_raw) }, PollFlags::POLLIN),
            PollFd::new(unsafe { BorrowedFd::borrow_raw(stdin_fd) }, PollFlags::POLLIN),
        ];

        match poll(&mut pollfds, PollTimeout::from(100u16)) {
            Ok(_) => {}
            Err(Errno::EINTR) => continue,
            Err(e) => bail!("poll error: {}", e),
        }

        // Read from master (child output).
        if pollfds[0]
            .revents()
            .is_some_and(|r| r.contains(PollFlags::POLLIN))
        {
            match read(master_raw, &mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let elapsed = start.elapsed().as_secs_f64();
                    let data = String::from_utf8_lossy(&buf[..n]).into_owned();
                    cast_writer.write_event(&CastEvent {
                        time: elapsed,
                        event_type: "o".into(),
                        data,
                    })?;
                    // Write to our stdout so user sees the output.
                    io::stdout().write_all(&buf[..n])?;
                    io::stdout().flush()?;
                }
                Err(Errno::EAGAIN | Errno::EIO) => {
                    if child_exited {
                        break;
                    }
                }
                Err(e) => bail!("read from master: {}", e),
            }
        }

        // Read from stdin (user input).
        if pollfds[1]
            .revents()
            .is_some_and(|r| r.contains(PollFlags::POLLIN))
        {
            match read(stdin_fd, &mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    // Relay to master.
                    let _ = write(&master_fd, &buf[..n]);
                }
                Err(Errno::EAGAIN) => {}
                Err(e) => bail!("read from stdin: {}", e),
            }
        }

        // Check if master has hung up.
        if pollfds[0]
            .revents()
            .is_some_and(|r| r.contains(PollFlags::POLLHUP))
        {
            break;
        }

        // Non-blocking waitpid to check if child has exited.
        if !child_exited {
            match waitpid(child, Some(WaitPidFlag::WNOHANG)) {
                Ok(nix::sys::wait::WaitStatus::Exited(_, _))
                | Ok(nix::sys::wait::WaitStatus::Signaled(_, _, _)) => {
                    child_exited = true;
                    // Do one more poll iteration to drain remaining output.
                }
                _ => {}
            }
        } else {
            // Child exited and we've had one more iteration, stop.
            break;
        }
    }

    cast_writer.flush()?;

    // Restore terminal.
    let _ = crossterm::terminal::disable_raw_mode();

    // Clean up child if needed.
    if !child_exited {
        let _ = signal::kill(child, Signal::SIGTERM);
        let _ = waitpid(child, None);
    }

    Ok(())
}

fn set_nonblocking(fd: &OwnedFd) -> Result<()> {
    let raw = fd.as_raw_fd();
    let flags = fcntl(raw, FcntlArg::F_GETFL)?;
    fcntl(
        raw,
        FcntlArg::F_SETFL(OFlag::from_bits_truncate(flags) | OFlag::O_NONBLOCK),
    )?;
    Ok(())
}
