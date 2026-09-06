//! agy-retry: Terminal wrapper for `agy` with error auto-retry.
//!
//! Monitors live PTY output for failure messages and automatically resumes
//! the session by sending '.' + Enter.  Also supports `-c`/`--conversation`
//! flags that are forwarded to `agy`.
//!
//! Config env-vars:
//!   AGY_BIN                 – path to the agy binary  (default: ~/.local/bin/agy)
//!   AGY_AUTO_RETRY_DELAY    – seconds before retry     (default: 1.0)
//!   AGY_AUTO_COOLDOWN       – seconds to ignore redraws after retry (default: 4.0)
//!   AGY_AUTO_EXTRA_PATTERNS – pipe-separated extra error patterns

use std::env;
use std::ffi::CString;

use std::os::unix::io::RawFd;
use std::time::{Duration, Instant};

use libc::{
    c_int, fd_set, select, timeval, winsize,
    STDIN_FILENO, STDOUT_FILENO,
    SIGWINCH, TIOCGWINSZ, TIOCSWINSZ,
    FD_ISSET, FD_SET, FD_ZERO,
};
use regex::Regex;

// ─── helpers ──────────────────────────────────────────────────────────────────

fn env_str(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_string())
}
fn env_f64(key: &str, default: f64) -> f64 {
    env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Write bytes to a raw fd, ignoring EINTR.
fn write_all_fd(fd: RawFd, buf: &[u8]) {
    let mut written = 0;
    while written < buf.len() {
        let n = unsafe {
            libc::write(fd, buf[written..].as_ptr() as *const libc::c_void, buf.len() - written)
        };
        if n <= 0 { break; }
        written += n as usize;
    }
}

/// Read up to `cap` bytes from fd; returns None on EOF/error.
fn read_fd(fd: RawFd, cap: usize) -> Option<Vec<u8>> {
    let mut buf = vec![0u8; cap];
    let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, cap) };
    if n <= 0 { None } else { buf.truncate(n as usize); Some(buf) }
}

/// Sync window size from src terminal → dst pty master.
fn sync_winsize(src: RawFd, dst: RawFd) {
    unsafe {
        let mut ws: winsize = std::mem::zeroed();
        if libc::ioctl(src, TIOCGWINSZ, &mut ws) == 0 {
            libc::ioctl(dst, TIOCSWINSZ, &ws);
        }
    }
}

/// Strip ANSI escape sequences from text.
fn strip_ansi(s: &str) -> String {
    // \x1b followed by various escape sequence forms
    let re = Regex::new(r"\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])").unwrap();
    re.replace_all(s, "").into_owned()
}

/// Build the list of error-detection regexes.
fn build_patterns() -> Vec<Regex> {
    let mut pats: Vec<Regex> = vec![
        Regex::new(r"(?i)There was a network issue connecting to the server").unwrap(),
        Regex::new(r"(?i)Agent execution terminated due to error").unwrap(),
    ];
    if let Ok(extra) = env::var("AGY_AUTO_EXTRA_PATTERNS") {
        for part in extra.split('|') {
            let p = part.trim();
            if !p.is_empty() {
                if let Ok(re) = Regex::new(&format!("(?i){}", regex::escape(p))) {
                    pats.push(re);
                }
            }
        }
    }
    pats
}

fn check_patterns(text: &str, patterns: &[Regex]) -> Option<String> {
    for re in patterns {
        if let Some(m) = re.find(text) {
            return Some(m.as_str().to_string());
        }
    }
    None
}

// ─── arg parsing ──────────────────────────────────────────────────────────────

/// Parse our own args, returning (agy_args_to_forward).
/// We normalise:
///   -c ID  | --conversation ID  | -c=ID | --conversation=ID
/// into:  ["--conversation", ID]  which agy understands.
fn parse_args(args: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        // short form: -c VALUE  or  -cVALUE
        if a == "-c" {
            i += 1;
            if i < args.len() {
                out.push("--conversation".to_string());
                out.push(args[i].clone());
            } else {
                eprintln!("[agy-retry] -c requires an argument");
            }
        } else if a.starts_with("-c") && a.len() > 2 && !a.starts_with("--") {
            // -cSOMETHING
            out.push("--conversation".to_string());
            out.push(a[2..].to_string());
        }
        // long form: --conversation VALUE  or  --conversation=VALUE
        else if a == "--conversation" || a == "-conversation" {
            i += 1;
            if i < args.len() {
                out.push("--conversation".to_string());
                out.push(args[i].clone());
            } else {
                eprintln!("[agy-retry] --conversation requires an argument");
            }
        } else if a.starts_with("--conversation=") {
            out.push("--conversation".to_string());
            out.push(a["--conversation=".len()..].to_string());
        } else if a.starts_with("-conversation=") {
            out.push("--conversation".to_string());
            out.push(a["-conversation=".len()..].to_string());
        } else {
            out.push(a.clone());
        }
        i += 1;
    }
    out
}

// ─── signal plumbing ─────────────────────────────────────────────────────────

/// We use a self-pipe trick: SIGWINCH handler writes a byte; the main loop
/// reads it and syncs the window size.
static mut SIGWINCH_PIPE_WRITE: RawFd = -1;

extern "C" fn sigwinch_handler(_: c_int) {
    unsafe { libc::write(SIGWINCH_PIPE_WRITE, b"\x00".as_ptr() as *const _, 1); }
}

// ─── PTY fork + exec ─────────────────────────────────────────────────────────

/// Fork a PTY child, exec `prog` with `argv`.  Returns (child_pid, master_fd).
fn pty_fork_exec(prog: &str, argv: &[String]) -> (libc::pid_t, RawFd) {
    let mut master: RawFd = -1;
    let child = unsafe {
        let slave_name_buf = [0 as libc::c_char; 256];
        // openpty
        let mut slave: RawFd = -1;
        let rc = libc::openpty(
            &mut master,
            &mut slave,
            slave_name_buf.as_ptr() as *mut _,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(rc, 0, "openpty failed");

        let pid = libc::fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // ── child ──
            libc::close(master);
            // create new session and set slave as controlling terminal
            libc::setsid();
            libc::ioctl(slave, libc::TIOCSCTTY as _, 0i32);
            libc::dup2(slave, STDIN_FILENO);
            libc::dup2(slave, STDOUT_FILENO);
            libc::dup2(slave, libc::STDERR_FILENO);
            if slave > 2 { libc::close(slave); }

            // build C argv
            let prog_c = CString::new(prog).unwrap();
            let mut cargv: Vec<*const libc::c_char> = Vec::new();
            let prog_ptr = prog_c.as_ptr();
            cargv.push(prog_ptr);
            let cstrings: Vec<CString> = argv.iter()
                .map(|a| CString::new(a.as_str()).unwrap())
                .collect();
            for cs in &cstrings { cargv.push(cs.as_ptr()); }
            cargv.push(std::ptr::null());

            libc::execv(prog_c.as_ptr(), cargv.as_ptr());
            // exec failed
            libc::_exit(127);
        }
        libc::close(slave);
        pid
    };
    (child, master)
}

// ─── raw terminal ─────────────────────────────────────────────────────────────

fn set_raw(fd: RawFd) -> libc::termios {
    unsafe {
        let mut old: libc::termios = std::mem::zeroed();
        libc::tcgetattr(fd, &mut old);
        let mut raw = old;
        libc::cfmakeraw(&mut raw);
        libc::tcsetattr(fd, libc::TCSANOW, &raw);
        old
    }
}

fn restore_termios(fd: RawFd, saved: &libc::termios) {
    unsafe { libc::tcsetattr(fd, libc::TCSADRAIN, saved); }
}

// ─── select helper ────────────────────────────────────────────────────────────

fn do_select(fds: &[RawFd], timeout_ms: u64) -> Vec<RawFd> {
    let mut ready = Vec::new();
    if fds.is_empty() { return ready; }
    let max_fd = fds.iter().copied().max().unwrap() + 1;
    let mut set: fd_set = unsafe { std::mem::zeroed() };
    unsafe { FD_ZERO(&mut set); }
    for &fd in fds { unsafe { FD_SET(fd, &mut set); } }
    let sec = (timeout_ms / 1000) as i64;
    let usec = ((timeout_ms % 1000) * 1000) as i64;
    let mut tv = timeval { tv_sec: sec, tv_usec: usec };
    let rc = unsafe { select(max_fd, &mut set, std::ptr::null_mut(), std::ptr::null_mut(), &mut tv) };
    if rc <= 0 { return ready; }
    for &fd in fds {
        if unsafe { FD_ISSET(fd, &set) } { ready.push(fd); }
    }
    ready
}

// ─── main ─────────────────────────────────────────────────────────────────────

fn main() {
    // Config
    let home = env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let agy_bin = env_str("AGY_BIN", &format!("{}/.local/bin/agy", home));
    let retry_delay = Duration::from_secs_f64(env_f64("AGY_AUTO_RETRY_DELAY", 1.0));
    let cooldown_dur = Duration::from_secs_f64(env_f64("AGY_AUTO_COOLDOWN", 4.0));

    if !std::path::Path::new(&agy_bin).exists() {
        eprintln!("[agy-retry] Error: agy binary not found at {}", agy_bin);
        std::process::exit(1);
    }

    // Parse our CLI args (skip argv[0])
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let agy_args = parse_args(&raw_args);

    // If stdin is not a tty, exec agy directly (no wrapping needed)
    if unsafe { libc::isatty(STDIN_FILENO) } == 0 {
        let prog = CString::new(agy_bin.as_str()).unwrap();
        let mut cargv: Vec<*const libc::c_char> = Vec::new();
        cargv.push(prog.as_ptr());
        let cstrings: Vec<CString> = agy_args.iter()
            .map(|a| CString::new(a.as_str()).unwrap())
            .collect();
        for cs in &cstrings { cargv.push(cs.as_ptr()); }
        cargv.push(std::ptr::null());
        unsafe { libc::execv(prog.as_ptr(), cargv.as_ptr()); }
        eprintln!("[agy-retry] execv failed");
        std::process::exit(1);
    }

    // Self-pipe for SIGWINCH
    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()); }
    let (sigwinch_read, sigwinch_write) = (pipe_fds[0], pipe_fds[1]);
    unsafe {
        SIGWINCH_PIPE_WRITE = sigwinch_write;
        let sa = libc::sigaction {
            sa_sigaction: sigwinch_handler as *const () as usize,
            sa_mask: std::mem::zeroed(),
            sa_flags: libc::SA_RESTART,
            sa_restorer: None,
        };
        libc::sigaction(SIGWINCH, &sa, std::ptr::null_mut());
    }

    // Fork child + exec agy
    let (child_pid, master_fd) = pty_fork_exec(&agy_bin, &agy_args);

    // Sync initial window size
    sync_winsize(STDIN_FILENO, master_fd);

    // Put stdin in raw mode
    let saved_termios = set_raw(STDIN_FILENO);

    let patterns = build_patterns();

    // ── state ──
    let mut stream_buffer = String::new();
    let mut error_detected_this_turn = false;
    let mut last_error = String::new();
    let mut pending_retry = false;
    let mut pending_retry_at = Instant::now();
    let mut cooldown_until: Option<Instant> = None;

    // ── event loop ──
    'main: loop {
        let now = Instant::now();

        // Fire pending retry if timer elapsed
        if pending_retry && now >= pending_retry_at {
            pending_retry = false;
            let msg = format!(
                "\r\n\x1b[1;33m[agy-retry]\x1b[0m Error detected ({last_error}). Resuming...\r\n"
            );
            write_all_fd(STDOUT_FILENO, msg.as_bytes());
            write_all_fd(master_fd, b".");
            std::thread::sleep(Duration::from_millis(50));
            write_all_fd(master_fd, b"\r");
            cooldown_until = Some(Instant::now() + cooldown_dur);
            stream_buffer.clear();
            error_detected_this_turn = false;
        }

        // Compute select timeout
        let timeout_ms: u64 = if pending_retry {
            let now = Instant::now();
            if now < pending_retry_at {
                let remaining = pending_retry_at - now;
                remaining.as_millis().min(100) as u64
            } else {
                0
            }
        } else {
            100
        };

        let watch = [master_fd, STDIN_FILENO, sigwinch_read];
        let ready = do_select(&watch, timeout_ms);

        // ── SIGWINCH ──
        if ready.contains(&sigwinch_read) {
            let mut discard = [0u8; 64];
            unsafe { libc::read(sigwinch_read, discard.as_mut_ptr() as *mut _, 64); }
            sync_winsize(STDIN_FILENO, master_fd);
        }

        // ── Output from agy ──
        if ready.contains(&master_fd) {
            match read_fd(master_fd, 4096) {
                None => break 'main,
                Some(data) => {
                    // Forward to user's terminal
                    write_all_fd(STDOUT_FILENO, &data);

                    let now = Instant::now();
                    // During cooldown: skip error scanning to prevent redraw re-triggers
                    if cooldown_until.map_or(false, |t| now < t) {
                        stream_buffer.clear();
                    } else {
                        let text = String::from_utf8_lossy(&data);
                        let plain = strip_ansi(&text);
                        stream_buffer.push_str(&plain);

                        // Bound buffer — snap to char boundary to avoid panics on
                        // multi-byte UTF-8 sequences.
                        if stream_buffer.len() > 8192 {
                            let trim = stream_buffer.len() - 4096;
                            // Walk forward until we land on a char boundary.
                            let trim = (trim..=stream_buffer.len())
                                .find(|&i| stream_buffer.is_char_boundary(i))
                                .unwrap_or(stream_buffer.len());
                            stream_buffer.drain(..trim);
                        }

                        if !error_detected_this_turn && !pending_retry {
                            if let Some(matched) = check_patterns(&stream_buffer, &patterns) {
                                last_error = matched;
                                error_detected_this_turn = true;
                                pending_retry = true;
                                pending_retry_at = Instant::now() + retry_delay;
                            }
                        }
                    }
                }
            }
        }

        // ── Input from user ──
        if ready.contains(&STDIN_FILENO) {
            match read_fd(STDIN_FILENO, 1024) {
                None => break 'main,
                Some(user_data) => {
                    // User interaction: reset state
                    if user_data.contains(&b'\r') || user_data.contains(&b'\n') {
                        stream_buffer.clear();
                        error_detected_this_turn = false;
                        pending_retry = false;
                    }
                    write_all_fd(master_fd, &user_data);
                }
            }
        }
    }

    // Restore terminal
    restore_termios(STDIN_FILENO, &saved_termios);

    // Wait for child
    let mut status: c_int = 0;
    unsafe {
        libc::waitpid(child_pid, &mut status, 0);
        if libc::WIFEXITED(status) {
            std::process::exit(libc::WEXITSTATUS(status));
        } else if libc::WIFSIGNALED(status) {
            std::process::exit(128 + libc::WTERMSIG(status));
        }
    }
}
