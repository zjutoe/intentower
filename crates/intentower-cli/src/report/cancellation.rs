//! Cancellation handshake for GNU timeout 8.32 on the supported CLI platform.
//!
//! Keep ALRM pending until timeout has registered its monitored PID. Its
//! handler then applies the configured KILL to the whole tool process group.
//! The tool inherits blocked ALRM; our Rossi/cvc5 calls use external deadlines.
//! PDEATHSIG follows the creating thread, which is the synchronous CLI here.

#[cfg(not(all(
    target_os = "linux",
    target_env = "gnu",
    target_arch = "x86_64",
    target_pointer_width = "64"
)))]
compile_error!("tool cancellation requires reviewed x86_64 Linux/glibc and GNU timeout behavior");

use std::ffi::{c_int, c_ulong};
use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

// Linux x86_64 constants and glibc's 1024-bit, unsigned-long-aligned sigset_t.
const SIGALRM: c_int = 14;
const SIG_BLOCK: c_int = 0;
const PR_SET_PDEATHSIG: c_int = 1;
const ESRCH: c_int = 3;

#[repr(C)]
struct SigSet {
    words: [c_ulong; 16],
}

unsafe extern "C" {
    fn sigemptyset(set: *mut SigSet) -> c_int;
    fn sigaddset(set: *mut SigSet, signal: c_int) -> c_int;
    fn sigprocmask(how: c_int, set: *const SigSet, old: *mut SigSet) -> c_int;
    fn prctl(option: c_int, ...) -> c_int;
    fn getppid() -> c_int;
}

pub(super) fn bind_to_cli(command: &mut Command) {
    let parent = std::process::id();
    // SAFETY: after fork, use only these async-signal-safe libc operations and
    // allocation-free OS errors. No locks, formatting or Rust allocation occur.
    // All pointers refer to the correctly aligned local glibc signal set.
    unsafe {
        command.pre_exec(move || {
            let mut signals = SigSet { words: [0; 16] };
            if sigemptyset(&mut signals) != 0
                || sigaddset(&mut signals, SIGALRM) != 0
                || sigprocmask(SIG_BLOCK, &signals, std::ptr::null_mut()) != 0
                || prctl(
                    PR_SET_PDEATHSIG,
                    SIGALRM as c_ulong,
                    0 as c_ulong,
                    0 as c_ulong,
                    0 as c_ulong,
                ) != 0
            {
                return Err(io::Error::last_os_error());
            }
            // A death before registration would otherwise deliver no signal.
            if getppid() as u32 != parent {
                return Err(io::Error::from_raw_os_error(ESRCH));
            }
            Ok(())
        });
    }
}
