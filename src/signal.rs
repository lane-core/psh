//! Signal handling for psh.
//!
//! rc heritage: signal handlers are functions named after their signals.
//! `fn sigint { }` handles SIGINT; deleting the function restores the
//! default disposition. `sigexit` is an artificial signal that fires
//! when the shell exits.
//!
//! Uses `signals_receipts` for async-signal-safe atomic counters.
//! The signal handler increments a counter; the main loop polls for
//! pending signals and dispatches to named functions in the function
//! table.

use signals_receipts::Premade;

// Declare the premade signal receipts for each signal we handle.
// The delegates here just take the receipt count — actual dispatch
// to shell functions happens in `check_signals` which polls the
// counters directly.
signals_receipts::premade! {
    SIGINT    => |_receipt| {};
    SIGCHLD   => |_receipt| {};
    SIGTSTP   => |_receipt| {};
    SIGCONT   => |_receipt| {};
    SIGWINCH  => |_receipt| {};
    SIGHUP    => |_receipt| {};
    SIGTERM   => |_receipt| {};
}

use signals_receipts_premade::SignalsReceipts;

/// Install signal handlers for all signals psh cares about.
///
/// `mask = true`: block other signals during handler execution.
/// `restart = true`: restart interrupted syscalls (SA_RESTART).
pub fn install_handlers() {
    SignalsReceipts::install_all_handlers();
}

/// Uninstall all signal handlers, restoring defaults.
pub fn uninstall_handlers() {
    SignalsReceipts::uninstall_all_handlers();
}

/// Signal identifiers that map to rc-style function names.
///
/// Each variant has a signal number and the rc function name
/// (e.g., "sigint" for SIGINT → `fn sigint { ... }`).
#[derive(Debug, Clone, Copy)]
pub struct SignalInfo {
    pub signum: i32,
    pub name: &'static str,
}

/// All signals psh handles, in check order.
pub const HANDLED_SIGNALS: &[SignalInfo] = &[
    SignalInfo {
        signum: libc::SIGCHLD,
        name: "sigchld",
    },
    SignalInfo {
        signum: libc::SIGINT,
        name: "sigint",
    },
    SignalInfo {
        signum: libc::SIGTSTP,
        name: "sigtstp",
    },
    SignalInfo {
        signum: libc::SIGCONT,
        name: "sigcont",
    },
    SignalInfo {
        signum: libc::SIGWINCH,
        name: "sigwinch",
    },
    SignalInfo {
        signum: libc::SIGHUP,
        name: "sighup",
    },
    SignalInfo {
        signum: libc::SIGTERM,
        name: "sigterm",
    },
];

/// Check which signals have been received since last check.
///
/// Returns a list of (signal_name, count) for each signal that fired
/// at least once. Consumes the counts atomically.
pub fn take_pending() -> Vec<(&'static str, u64)> {
    use signals_receipts::SignalReceipt;

    let mut pending = Vec::new();

    macro_rules! check_signal {
        ($sig:ident, $name:expr) => {
            let count = <SignalsReceipts as SignalReceipt<{ libc::$sig }>>::take_count();
            if count > 0 {
                pending.push(($name, count));
            }
        };
    }

    // SIGCHLD first — reaping children before other signal dispatch
    // prevents zombies from accumulating.
    check_signal!(SIGCHLD, "sigchld");
    check_signal!(SIGINT, "sigint");
    check_signal!(SIGTSTP, "sigtstp");
    check_signal!(SIGCONT, "sigcont");
    check_signal!(SIGWINCH, "sigwinch");
    check_signal!(SIGHUP, "sighup");
    check_signal!(SIGTERM, "sigterm");

    pending
}

/// Ignore a signal by setting its disposition to SIG_IGN.
///
/// Used for signals the shell itself should not receive (e.g.,
/// the shell ignores SIGTSTP — only the foreground job's process
/// group receives it).
pub fn ignore_signal(signum: i32) {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = libc::SIG_IGN;
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(signum, &action, std::ptr::null_mut());
    }
}
