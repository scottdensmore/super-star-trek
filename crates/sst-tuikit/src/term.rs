//! `github.com/charmbracelet/x/term` (and the bits of `golang.org/x/sys/unix`
//! that termenv/bubbletea/theme detection use): TTY checks, raw mode, window
//! size, and readiness polling.
//!
//! The only `unsafe` code of the crate lives here: thin libc calls
//! (`isatty`, `tcgetattr`/`tcsetattr`, `ioctl`, `select`, `read`) that have
//! no safe std equivalent.

use std::io;
use std::time::Duration;

/// A raw file descriptor (Go `uintptr` fd).
pub type Fd = i32;

/// Standard descriptors.
pub const STDIN: Fd = 0;
pub const STDOUT: Fd = 1;

/// Saved terminal state (`term.State`).
#[derive(Clone)]
pub struct State {
    #[cfg(unix)]
    termios: libc::termios,
}

/// `term.IsTerminal`.
pub fn is_terminal(fd: Fd) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: isatty only inspects the descriptor.
        unsafe { libc::isatty(fd) == 1 }
    }
    #[cfg(not(unix))]
    {
        use std::io::IsTerminal;
        match fd {
            0 => io::stdin().is_terminal(),
            1 => io::stdout().is_terminal(),
            2 => io::stderr().is_terminal(),
            _ => false,
        }
    }
}

#[cfg(unix)]
fn get_termios(fd: Fd) -> io::Result<libc::termios> {
    // SAFETY: termios is plain old data; tcgetattr fills it.
    unsafe {
        let mut t: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(fd, &mut t) != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(t)
    }
}

#[cfg(unix)]
fn set_termios(fd: Fd, t: &libc::termios) -> io::Result<()> {
    // SAFETY: passes a valid termios pointer.
    unsafe {
        if libc::tcsetattr(fd, libc::TCSANOW, t) != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// `term.GetState`.
pub fn get_state(fd: Fd) -> io::Result<State> {
    #[cfg(unix)]
    {
        Ok(State {
            termios: get_termios(fd)?,
        })
    }
    #[cfg(not(unix))]
    {
        let _ = fd;
        Ok(State {})
    }
}

/// `term.MakeRaw`: puts the terminal into raw mode, returning the previous
/// state (golang.org/x/term flag set).
pub fn make_raw(fd: Fd) -> io::Result<State> {
    #[cfg(unix)]
    {
        let old = get_termios(fd)?;
        let mut t = old;
        t.c_iflag &= !(libc::IGNBRK
            | libc::BRKINT
            | libc::PARMRK
            | libc::ISTRIP
            | libc::INLCR
            | libc::IGNCR
            | libc::ICRNL
            | libc::IXON);
        t.c_oflag &= !libc::OPOST;
        t.c_lflag &= !(libc::ECHO | libc::ECHONL | libc::ICANON | libc::ISIG | libc::IEXTEN);
        t.c_cflag &= !(libc::CSIZE | libc::PARENB);
        t.c_cflag |= libc::CS8;
        t.c_cc[libc::VMIN] = 1;
        t.c_cc[libc::VTIME] = 0;
        set_termios(fd, &t)?;
        Ok(State { termios: old })
    }
    #[cfg(not(unix))]
    {
        let _ = fd;
        crossterm::terminal::enable_raw_mode()?;
        Ok(State {})
    }
}

/// `term.Restore`.
pub fn restore(fd: Fd, state: &State) -> io::Result<()> {
    #[cfg(unix)]
    {
        set_termios(fd, &state.termios)
    }
    #[cfg(not(unix))]
    {
        let _ = (fd, state);
        crossterm::terminal::disable_raw_mode()
    }
}

/// Clears ECHO and ICANON (termenv's status-report mode), returning the old
/// state.
pub fn set_noecho_noncanon(fd: Fd) -> io::Result<State> {
    #[cfg(unix)]
    {
        let old = get_termios(fd)?;
        let mut t = old;
        t.c_lflag &= !libc::ECHO;
        t.c_lflag &= !libc::ICANON;
        set_termios(fd, &t)?;
        Ok(State { termios: old })
    }
    #[cfg(not(unix))]
    {
        let _ = fd;
        Err(io::Error::new(io::ErrorKind::Unsupported, "unsupported"))
    }
}

/// `term.GetSize`: (width, height).
pub fn get_size(fd: Fd) -> io::Result<(i32, i32)> {
    #[cfg(unix)]
    {
        // SAFETY: winsize is plain old data filled by TIOCGWINSZ.
        unsafe {
            let mut ws: libc::winsize = std::mem::zeroed();
            if libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok((ws.ws_col as i32, ws.ws_row as i32))
        }
    }
    #[cfg(not(unix))]
    {
        let _ = fd;
        let (w, h) = crossterm::terminal::size()?;
        Ok((w as i32, h as i32))
    }
}

/// termenv `isForeground`: the terminal's foreground process group is ours.
pub fn is_foreground(fd: Fd) -> bool {
    #[cfg(unix)]
    {
        // SAFETY: plain libc queries.
        unsafe {
            let pgrp = libc::tcgetpgrp(fd);
            pgrp >= 0 && pgrp == libc::getpgrp()
        }
    }
    #[cfg(not(unix))]
    {
        let _ = fd;
        false
    }
}

/// Waits until `fd` is readable or `timeout` passes (`unix.Select`).
/// Returns `Ok(true)` when readable. EINTR is retried like termenv does.
pub fn wait_readable(fd: Fd, timeout: Duration) -> io::Result<bool> {
    #[cfg(unix)]
    {
        loop {
            // SAFETY: fd_set/timeval are POD; select only reads/writes them.
            let n = unsafe {
                let mut set: libc::fd_set = std::mem::zeroed();
                libc::FD_ZERO(&mut set);
                libc::FD_SET(fd, &mut set);
                let mut tv = libc::timeval {
                    tv_sec: timeout.as_secs() as _,
                    tv_usec: timeout.subsec_micros() as _,
                };
                libc::select(
                    fd + 1,
                    &mut set,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut tv,
                )
            };
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
            return Ok(n > 0);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (fd, timeout);
        Ok(false)
    }
}

/// Reads up to `buf.len()` bytes from `fd`.
pub fn read(fd: Fd, buf: &mut [u8]) -> io::Result<usize> {
    #[cfg(unix)]
    {
        // SAFETY: buf is a valid writable slice.
        let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(n as usize)
    }
    #[cfg(not(unix))]
    {
        use std::io::Read;
        let _ = fd;
        io::stdin().read(buf)
    }
}

/// Writes all of `data` to `fd`.
pub fn write_all(fd: Fd, data: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut data = data;
        while !data.is_empty() {
            // SAFETY: data is a valid readable slice.
            let n = unsafe { libc::write(fd, data.as_ptr() as *const libc::c_void, data.len()) };
            if n < 0 {
                let e = io::Error::last_os_error();
                if e.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
            data = &data[n as usize..];
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        use std::io::Write;
        let _ = fd;
        let mut out = io::stdout();
        out.write_all(data)?;
        out.flush()
    }
}

/// Opens `/dev/tty` read-write, returning its descriptor (closed on drop).
pub struct Tty {
    #[cfg(unix)]
    file: std::fs::File,
}

impl Tty {
    /// `os.OpenFile("/dev/tty", os.O_RDWR, 0)`.
    pub fn open() -> io::Result<Tty> {
        #[cfg(unix)]
        {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")?;
            Ok(Tty { file })
        }
        #[cfg(not(unix))]
        {
            Err(io::Error::new(io::ErrorKind::Unsupported, "no /dev/tty"))
        }
    }

    pub fn fd(&self) -> Fd {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            self.file.as_raw_fd()
        }
        #[cfg(not(unix))]
        {
            -1
        }
    }
}
