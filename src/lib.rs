use libc::{c_char, c_int};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub const FILENAME_INFERENCE_LIMIT: usize = 5000;

#[cfg(feature = "pre_bash_4_4")]
use ctor::ctor;

/// Writes formatted arguments to stdout and flushes immediately.
#[macro_export]
macro_rules! flush_stdout {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let mut stdout = std::io::stdout();
        write!(stdout, $($arg)*).and_then(|_| stdout.flush())
    }};
}

#[macro_use]
pub(crate) mod perf;
mod active_suggestions;
mod agent_mode;
mod app;
mod bash_funcs;
mod bash_symbols;
mod changelog;
mod cli;
mod command_acceptance;
mod content_builder;
mod content_utils;
mod cursor;
mod dparser;
mod globbing;
mod history;
pub mod hostnames;
mod i18n;
mod iter_first_last;
mod kill_on_drop_child;
mod logging;
mod mouse_state;
mod palette;
mod prompt_manager;
mod settings;
mod shell_integration;
mod snake_animation;
mod stateful_sliding_window;
mod tab_completion_context;
mod table;
mod text_buffer;
pub(crate) mod threads;
mod tutorial;
pub mod unicode_helpers;
mod users;

// Global state for our custom input stream
static FLYLINE_INSTANCE_PTR: Mutex<Option<Box<Flyline>>> = Mutex::new(None);

// Number of flyline library frames currently on the call stack (the input
// stream getter and the `flyline` builtin entry point).
//
// `enable -d flyline` unloads the builtin *and* dlclose()s the library.  When
// the unload is requested from inside our own code (e.g. a `runBashCommand`
// key binding that executes `enable -d flyline`), bash would dlclose() the
// library while its code is still running, which segfaults the shell.  In that
// situation we keep the library mapped and defer the real teardown until the
// next flyline_get_char() call reaches a safe point.
static FLYLINE_CALL_DEPTH: AtomicUsize = AtomicUsize::new(0);
static FLYLINE_UNLOAD_PENDING: AtomicBool = AtomicBool::new(false);
/// After `enable -d flyline`, if bash keeps calling this getter because the
/// input stream could not be restored, pass raw bytes straight through from
/// stdin instead of touching any flyline state.  This is the final safety net
/// for login shells whose saved-stream stack cannot be rebuilt safely.
static FLYLINE_PASS_THROUGH: AtomicBool = AtomicBool::new(false);

// When `enable -d flyline` runs from inside our own code, we add an extra
// dlopen() reference so bash's dlclose() does not unmap the library while we
// are still executing.  The reference is intentionally kept until the library
// is loaded again (see release_kept_library_reference), since dlclose()ing
// ourselves from inside our own code would unmap the function we are returning
// from.
struct DlHandle(*mut libc::c_void);
unsafe impl Send for DlHandle {}
unsafe impl Sync for DlHandle {}

static KEPT_LIBRARY_HANDLE: Mutex<Option<DlHandle>> = Mutex::new(None);

struct CallDepthGuard;

impl CallDepthGuard {
    fn enter() -> Self {
        FLYLINE_CALL_DEPTH.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for CallDepthGuard {
    fn drop(&mut self) {
        FLYLINE_CALL_DEPTH.fetch_sub(1, Ordering::SeqCst);
    }
}

fn catch_unwind_safe<T>(f: impl FnOnce() -> T) -> Result<T, ()> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|_| ())
}

fn report_stderr_no_panic(message: &str) {
    let _ = catch_unwind_safe(|| {
        eprintln!("{message}");
    });
}

fn report_error_no_panic(message: &str) {
    let _ = catch_unwind_safe(|| {
        log::error!("{message}");
    });
}

// C-compatible getter function that bash will call
extern "C" fn flyline_get_char() -> c_int {
    let _depth = CallDepthGuard::enter();
    // An unload requested from a nested input source (eval/source/command
    // substitution) is deferred until bash is back on flyline's stream.  At
    // that point finish the teardown before the instance is touched, then
    // delegate one read to the restored stream's getter so bash seamlessly
    // continues on readline.  A synthetic newline would make bash process an
    // extra empty command, and EOF would make a login shell log out.
    if FLYLINE_UNLOAD_PENDING.swap(false, Ordering::SeqCst) {
        unsafe {
            finish_deferred_unload();
            if let Some(getter) = bash_symbols::bash_input.getter {
                return getter();
            }
        }
        return b'\n' as c_int;
    }
    // Passthrough is the safety net for getter calls that still reach flyline
    // after a non-deferred unload.  It must run after the pending check above
    // so deferred teardown can restore the readline stream first.
    if FLYLINE_PASS_THROUGH.load(Ordering::Relaxed) {
        let mut byte: u8 = 0;
        loop {
            let n = unsafe {
                libc::read(
                    0,
                    &mut byte as *mut u8 as *mut libc::c_void,
                    1,
                )
            };
            if n > 0 {
                return byte as c_int;
            }
            if n == 0 {
                return bash_symbols::EOF;
            }
            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::Interrupted {
                return bash_symbols::EOF;
            }
        }
    }
    let result = if let Some(boxed) = FLYLINE_INSTANCE_PTR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_mut()
    {
        match catch_unwind_safe(|| boxed.get()) {
            Ok(c) => c,
            Err(_) => {
                // writing to stderr can panic if master pty side has been closed.
                report_stderr_no_panic(
                    "flyline: app panicked; recovering with EOF. Please create an issue with the steps to reproduce at https://github.com/HalFrgrd/flyline/issues.",
                );
                report_error_no_panic("app panicked; recovering with EOF");

                std::thread::sleep(std::time::Duration::from_millis(1000));
                bash_symbols::EOF
            }
        }
    } else {
        // The instance has already been torn down (e.g. `enable -d flyline`
        // ran from a nested login shell), but bash's input stream still points
        // at this getter.  Restore the readline stdin stream so bash does not
        // keep calling into an unloaded/uninitialized flyline, then signal EOF
        // so the current read ends cleanly.
        unsafe {
            if is_flyline_stream(&raw const bash_symbols::bash_input) {
                report_stderr_no_panic(
                    "flyline_get_char: FLYLINE_INSTANCE_PTR is None; restoring readline stdin",
                );
                bash_symbols::with_input_from_stdin();
                // Signal the end of the current (empty) line instead of EOF:
                // returning EOF makes a login shell treat the unload as the
                // end of input and log out.  The next get_char call will use
                // the restored stdin/readline stream.
                return b'\n' as c_int;
            } else {
                report_stderr_no_panic("flyline_get_char: FLYLINE_INSTANCE_PTR is None");
            }
        }
        bash_symbols::EOF
    };

    result
}

// C-compatible ungetter function that bash will call
extern "C" fn flyline_unget_char(c: c_int) -> c_int {
    if let Some(boxed) = FLYLINE_INSTANCE_PTR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_mut()
    {
        return match catch_unwind_safe(|| boxed.unget(c)) {
            Ok(unget_char) => unget_char,
            Err(_) => {
                report_stderr_no_panic("flyline: unget handler panicked; ignoring.");
                report_error_no_panic("flyline_unget_char panicked; returning original character");
                c
            }
        };
    }
    report_stderr_no_panic("flyline_unget_char: FLYLINE_INSTANCE_PTR is None");
    c
}

extern "C" fn flyline_call_command(words: *const bash_symbols::WordList) -> c_int {
    let _depth = CallDepthGuard::enter();
    let result = catch_unwind_safe(|| {
        if let Some(boxed) = FLYLINE_INSTANCE_PTR
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
        {
            return boxed.call(words);
        }
        report_stderr_no_panic("flyline_call_command: FLYLINE_INSTANCE_PTR is None");
        0
    });
    match result {
        Ok(code) => code,
        Err(_) => {
            report_stderr_no_panic("flyline: command handler panicked; ignoring.");
            report_error_no_panic("flyline_call_command panicked; returning failure");
            bash_symbols::BuiltinExitCode::Usage as c_int
        }
    }
}

#[derive(Debug)]
pub(crate) struct Flyline {
    content: Vec<u8>,
    position: usize,
    settings: settings::Settings,
}

impl Flyline {
    fn new() -> Self {
        Self {
            content: vec![],
            position: 0,
            settings: settings::Settings::default(),
        }
    }

    fn get(&mut self) -> c_int {
        // This is meant to mimic yy_readline_get.
        if self.content.is_empty() || self.position >= self.content.len() {
            log::info!("---------------------- Starting app ------------------------");

            unsafe {
                if bash_symbols::job_control != 0 {
                    bash_symbols::give_terminal_to(bash_symbols::shell_pgrp, 0);
                }
            }

            // In yy_readline_get, Bash has some SIGINT handling.
            // But we put the terminal in raw mode so we're unlikely to receive SIGINTs.
            // So I don't bother with this logic.

            // I haven't bothered replicating this line either:
            //   sh_unset_nodelay_mode (fileno (rl_instream));	/* just in case */
            // Reset SIGCHLD to SIG_DFL so child process spawning works without ECHILD;
            // SigchldGuard restores Bash's original handler upon drop.

            let _sigchld_guard = SigchldGuard::new();

            let result = app::get_command(&mut self.settings);

            self.settings.last_app_closed_at = Some(std::time::Instant::now());

            // Join the background cache warming thread before returning control to Bash.
            // This ensures that no background Rust threads are running or calling Bash FFI
            // functions while Bash is executing command execution C code (which is single-threaded
            // and has no locking of its own).
            crate::threads::join_bash_func_threads();

            // unsafe {
            //     // This doesn't seem to be strictly necessary but yy_readline_get does it here.
            //     // I think something upstream will handle it if we don't run this here.
            //     let sig = bash_symbols::terminating_signal;
            //     if sig != 0 {
            //         log::info!(
            //             "Terminating signal {} received, exiting immediately",
            //             app::signal_to_str(sig)
            //         );
            //         bash_symbols::termsig_handler(sig);
            //     }
            // }

            self.content = match result {
                app::ExitState::WithCommand(cmd) => {
                    if self.settings.tutorial_step.is_active() && cmd.trim().is_empty() {
                        self.settings.tutorial_step.next();
                        log::info!(
                            "Tutorial step advanced to {:?}",
                            self.settings.tutorial_step
                        );
                        if !self.settings.tutorial_step.is_active() {
                            self.settings.run_tutorial = false;
                        }
                    }

                    cmd.into_bytes()
                }
                app::ExitState::EOF => {
                    log::info!("App signaled EOF");
                    return bash_symbols::EOF;
                }
                app::ExitState::WithoutCommand => vec![],
            };
            log::info!("---------------------- App finished ------------------------");
            self.content.push(b'\n');
            self.position = 0;
        }

        if let Some(byte) = self.content.get(self.position) {
            self.position += 1;
            *byte as c_int
        } else {
            log::info!("End of input stream reached, returning EOF");
            bash_symbols::EOF
        }
    }

    fn unget(&mut self, _c: c_int) -> c_int {
        if self.position > 0 {
            self.position -= 1;
            self.content[self.position] as c_int
        } else {
            _c
        }
    }
}

struct SyncPtrs([*const c_char; 4]);
unsafe impl Sync for SyncPtrs {}

static FLYLINE_LONG_DOC: SyncPtrs = SyncPtrs([
    c"Advanced command line editing for Bash.\n".as_ptr(),
    c"Refer to `flyline --help` for more help.\n".as_ptr(),
    std::ptr::null(),
    std::ptr::null(),
]);

/* Exported builtin struct */
#[unsafe(no_mangle)]
pub static mut flyline_struct: bash_symbols::BashBuiltin = bash_symbols::BashBuiltin {
    name: c"flyline".as_ptr(),
    function: Some(flyline_call_command),
    flags: bash_symbols::BUILTIN_ENABLED,
    long_doc: FLYLINE_LONG_DOC.0.as_ptr(),
    short_doc: c"flyline [option] ... [subcommand]".as_ptr(),
    handle: std::ptr::null(),
};

// On pre-bash-4.4 builds, register a shared-library constructor so that flyline
// is initialised as soon as the library is loaded via `enable -f`.
// On newer versions of bash `flyline_builtin_load` is called automatically by bash during enable.
#[cfg(all(feature = "pre_bash_4_4", not(test)))]
#[ctor(unsafe)]
fn flyline_builtin_load_ctor() {
    let _ = flyline_load_common();
}

#[cfg(not(feature = "pre_bash_4_4"))]
#[unsafe(no_mangle)]
pub extern "C" fn flyline_builtin_load(_arg: *const c_char) -> c_int {
    flyline_load_common()
}

const FLYLINE_ENV_VAR_NAME: &str = "FLYLINE_VERSION";
const FLYLINE_ENV_VAR_VALUE: &str = env!("CARGO_PKG_VERSION");

fn flyline_load_common() -> c_int {
    log::info!("flyline_builtin_load called, initializing flyline");
    release_kept_library_reference();
    FLYLINE_PASS_THROUGH.store(false, Ordering::SeqCst);
    FLYLINE_UNLOAD_PENDING.store(false, Ordering::SeqCst);
    // Returning 0 means the load fails
    const SUCCESS: c_int = 1;
    const FAILURE: c_int = 0;

    let already_initialized = FLYLINE_INSTANCE_PTR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .is_some();
    if already_initialized {
        log::info!("flyline_builtin_load: already initialized, skipping");
        return SUCCESS;
    }

    logging::init().unwrap_or_else(|e| {
        eprintln!("Flyline failed to setup logging: {}", e);
    });

    // When do we want to set up flyline's input stream?
    // shell.c:main:792:set_bash_input: sets up readline if interactive && no_line_editing

    // unsafe {
    //     log::trace!(
    //         "interactive: {}, interactive_shell: {}, no_line_editing: {}",
    //         bash_symbols::interactive,
    //         bash_symbols::interactive_shell,
    //         bash_symbols::no_line_editing
    //     );
    // }

    unsafe {
        if bash_symbols::interactive_shell == 0 || bash_symbols::no_line_editing != 0 {
            log::warn!("Not an interactive shell, flyline will not be loaded");
            log::info!(
                "To avoid loading flyline in non-interactive shells, add the following to your .bashrc before the flyline enable line:\nif [[ $- != *i* ]]; then return; fi"
            );
            logging::print_logs_stderr();
            return FAILURE;
        }
    }

    // This is how we ensure that our custom input stream is used by bash instead of readline.
    // This code is run during `run_startup_files` so we can't modify bash_input directly.
    // `bash_input` is being used to read the rc files at this point. set_bash_input() has yet to be called.
    // `stream_list` contains only a sentinel input stream at this point.
    // Normally when it is popped off the list after rc files are read, readline stdin is added since
    // `with_input_from_stdin` sees that the current bash_input is of type st_stdin.
    // So we modify the sentinel node before that happens so that in set_bash_input,
    // with_input_from_stdin will see that the current bash_input is fit for purpose and not add readline stdin.

    let setup_bash_input = |bash_input: *mut bash_symbols::BashInput| {
        let old_name = unsafe { (*bash_input).name };
        // Bash expects name to be heap allocated so it can free it later
        let name = c"flyline";
        let name_ptr = unsafe { bash_symbols::locked_xmalloc_cstr(name) };
        unsafe {
            (*bash_input).stream_type = bash_symbols::StreamType::Stdin;
            (*bash_input).name = name_ptr;
            (*bash_input).getter = Some(flyline_get_char);
            (*bash_input).ungetter = Some(flyline_unget_char);
            if !old_name.is_null() {
                bash_symbols::locked_xfree(old_name as *mut libc::c_void);
            }
        }

        // Store the Arc globally so C callbacks can access it
        *FLYLINE_INSTANCE_PTR
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(Box::new(Flyline::new()));

        bash_funcs::export_env_var(FLYLINE_ENV_VAR_NAME, FLYLINE_ENV_VAR_VALUE).unwrap_or_else(
            |e| {
                log::error!(
                    "Failed to export environment variable '{}': {}",
                    FLYLINE_ENV_VAR_NAME,
                    e
                );
            },
        );

        let load_dir_var = "FLYLINE_LOAD_DIR";
        let is_load_dir_set = unsafe {
            let name_cstr = std::ffi::CString::new(load_dir_var).unwrap();
            let var = bash_symbols::find_variable(name_cstr.as_ptr());
            !var.is_null()
        };

        if !is_load_dir_set {
            if let Some(path) = get_library_directory() {
                let path_str = if let Ok(abs_path) = std::fs::canonicalize(&path) {
                    abs_path.to_string_lossy().into_owned()
                } else {
                    path.to_string_lossy().into_owned()
                };
                if let Err(e) = bash_funcs::export_env_var(load_dir_var, &path_str) {
                    log::error!(
                        "Failed to export environment variable '{}': {}",
                        load_dir_var,
                        e
                    );
                } else {
                    log::info!("Exported {} to '{}'", load_dir_var, path_str);
                }
            }
        }
    };

    unsafe {
        if !bash_symbols::bash_input.name.is_null() {
            let current_input_name =
                std::ffi::CStr::from_ptr(bash_symbols::bash_input.name).to_string_lossy();

            if current_input_name.starts_with("readline") {
                log::trace!("current bash input is readline, replacing it with flyline input");
                bash_symbols::push_stream(0);
                setup_bash_input(&raw mut bash_symbols::bash_input);
                log::set_max_level(log::LevelFilter::Info);
                return SUCCESS;
            } else if current_input_name.starts_with("flyline") {
                log::trace!("current bash input is already flyline, overriding callbacks");
                setup_bash_input(&raw mut bash_symbols::bash_input);
                log::set_max_level(log::LevelFilter::Info);
                return SUCCESS;
            } else {
                log::trace!("current bash input is {}", current_input_name);
            }
        }

        if !bash_symbols::stream_list.is_null() {
            // iterate through the list
            // if we find a stream of type StStdin or StNone that is already flyline, override callbacks
            // if we find a stream of type StStdin or StNone that is not flyline, replace it with flyline
            let mut current = bash_symbols::stream_list;
            let mut idx = 0;
            while !current.is_null() {
                let stream = &*current;
                let name = if stream.bash_input.name.is_null() {
                    "?".to_string()
                } else {
                    std::ffi::CStr::from_ptr(stream.bash_input.name)
                        .to_string_lossy()
                        .into_owned()
                };
                log::trace!(
                    "stream_list[{}]: name: {}, type: {:?}",
                    idx,
                    name,
                    stream.bash_input.stream_type
                );
                if stream.bash_input.stream_type == bash_symbols::StreamType::Stdin
                    || stream.bash_input.stream_type == bash_symbols::StreamType::None
                {
                    if name.starts_with("flyline") {
                        log::trace!(
                            "Found existing flyline input stream in stream_list, overriding callbacks"
                        );
                        setup_bash_input(&raw mut (*current).bash_input);
                        log::set_max_level(log::LevelFilter::Info);
                        return SUCCESS;
                    }
                    // Replace it with flyline
                    log::trace!(
                        "Found stream_list entry with type {:?}, setting flyline input stream on this node",
                        stream.bash_input.stream_type
                    );
                    setup_bash_input(&raw mut (*current).bash_input);
                    log::set_max_level(log::LevelFilter::Info);
                    return SUCCESS;
                }

                current = stream.next;
                idx += 1;
            }
            log::error!("Could not setup flyline");
            logging::print_logs_stderr();
            return FAILURE;
        }
    }

    log::set_max_level(log::LevelFilter::Info);
    SUCCESS
}

// Its easier to just not unload on older bash versions
// Maybe I could use a fini_array function to unload, but I doubt its worth the effort.
#[cfg(not(feature = "pre_bash_4_4"))]
#[unsafe(no_mangle)]
pub extern "C" fn flyline_builtin_unload() {
    log::info!("flyline_builtin_unload called, unloading flyline");
    // From this point on, any getter call that still reaches flyline must be
    // answered from raw stdin rather than from flyline state.
    FLYLINE_PASS_THROUGH.store(true, Ordering::SeqCst);
    // Keep the library mapped: bash will dlclose() it after this returns, but
    // the input stream may still point at flyline_get_char in login shells.
    keep_library_loaded();
    crate::threads::join_all_before_unload();

    bash_funcs::unset_env_var(FLYLINE_ENV_VAR_NAME).unwrap_or_else(|e| {
        log::error!(
            "Failed to unset environment variable '{}': {}",
            FLYLINE_ENV_VAR_NAME,
            e
        );
    });

    // If the unload was requested while flyline code is still on the call
    // stack, or while bash is reading from a nested input source (eval,
    // source, command substitution, ...), bash is about to dlclose() this
    // library under our feet.  Keep the library mapped so the in-flight call
    // can finish, and defer the real teardown to the next flyline_get_char()
    // call, when bash is back on flyline's top-level stream.  The extra
    // dlopen() reference is released on the next load.
    unsafe {
        let nested_input = !is_flyline_stream(&raw const bash_symbols::bash_input);
        if FLYLINE_CALL_DEPTH.load(Ordering::SeqCst) > 0 || nested_input {
            log::warn!(
                "flyline unload requested from inside flyline code or a nested input source; deferring teardown to the next input read"
            );
            FLYLINE_UNLOAD_PENDING.store(true, Ordering::SeqCst);
            return;
        }
    }

    let had_instance = FLYLINE_INSTANCE_PTR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .is_some();

    if !had_instance {
        return;
    }

    unsafe {
        // Restore the readline input stream.  Do not pop or rewrite the saved
        // stream stack: in login shells it is unreliable, and passthrough mode
        // keeps any residual flyline getter call safe.  In an interactive
        // shell with_input_from_stdin() reattaches yy_readline_get, which is
        // what makes bash display PS1 again.
        clear_flyline_streams_from_stack();
        bash_symbols::bash_input.stream_type = bash_symbols::StreamType::None;
        bash_symbols::with_input_from_stdin();
    }
}

#[cfg(not(feature = "pre_bash_4_4"))]
unsafe fn is_flyline_stream(input: *const bash_symbols::BashInput) -> bool {
    unsafe {
        let input = &*input;
        if !input.name.is_null() {
            let name = std::ffi::CStr::from_ptr(input.name).to_string_lossy();
            if name.starts_with("flyline") {
                return true;
            }
        }
        false
    }
}

/// Replace every saved flyline stream on the stream stack with a plain stdin
/// stream.  The current input stream is handled separately (via
/// `with_input_from_stdin`, which restores readline), but any nested entries
/// pushed earlier would otherwise still point at this library after unload —
/// and would confuse a later `enable -f flyline` reload.
#[cfg(not(feature = "pre_bash_4_4"))]
unsafe fn clear_flyline_streams_from_stack() {
    unsafe {
        let mut current = bash_symbols::stream_list;
        let mut cleared = 0;
        while !current.is_null() {
            let node = &mut *current;
            if is_flyline_stream(&raw const node.bash_input) {
                if !node.bash_input.name.is_null() {
                    bash_symbols::locked_xfree(node.bash_input.name as *mut libc::c_void);
                }
                node.bash_input.stream_type = bash_symbols::StreamType::Stdin;
                node.bash_input.getter = None;
                node.bash_input.ungetter = None;
                node.bash_input.location = std::mem::zeroed();
                node.bash_input.name = bash_symbols::locked_xmalloc_cstr(c"readline stdin");
                cleared += 1;
            }
            current = node.next;
        }
        if cleared > 0 {
            log::info!("Cleared {cleared} saved flyline input stream(s)");
        }
    }
}

/// Finish a teardown that was deferred because `enable -d flyline` ran while
/// flyline code was on the call stack.  Called from flyline_get_char() after
/// the instance borrow has ended, so dropping the instance is safe.
#[cfg(not(feature = "pre_bash_4_4"))]
unsafe fn finish_deferred_unload() {
    log::info!("finishing deferred flyline unload");
    let had_instance = FLYLINE_INSTANCE_PTR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
        .is_some();
    if !had_instance {
        return;
    }

    // Reattach the readline input stream so bash displays PS1 again and
    // handles editing normally.  Any residual flyline getter calls are covered
    // by passthrough mode.
    unsafe {
        clear_flyline_streams_from_stack();
        bash_symbols::bash_input.stream_type = bash_symbols::StreamType::None;
        bash_symbols::with_input_from_stdin();
    }
}

fn get_library_path() -> Option<std::ffi::CString> {
    unsafe {
        let mut info = std::mem::zeroed::<Dl_info>();
        let addr = flyline_load_common as *const libc::c_void;
        if dladdr(addr, &mut info) != 0 && !info.dli_fname.is_null() {
            let path = std::ffi::CStr::from_ptr(info.dli_fname);
            return Some(path.to_owned());
        }
    }
    None
}

/// Add a reference to the currently loaded flyline shared object so bash's
/// `enable -d` dlclose() does not unmap the library while our code is running.
#[cfg(not(feature = "pre_bash_4_4"))]
fn keep_library_loaded() {
    let Some(path) = get_library_path() else {
        log::error!("flyline unload: could not determine library path to keep it loaded");
        return;
    };
    unsafe {
        let handle = libc::dlopen(path.as_ptr(), libc::RTLD_NOW | libc::RTLD_NOLOAD);
        if handle.is_null() {
            let err = std::ffi::CStr::from_ptr(libc::dlerror()).to_string_lossy();
            log::error!("flyline unload: failed to keep library loaded: {err}");
        } else {
            log::info!("Kept flyline library loaded while deferring unload");
            *KEPT_LIBRARY_HANDLE
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(DlHandle(handle));
        }
    }
}

/// Release the extra dlopen() reference added by keep_library_loaded().
/// Called when the library is loaded again, at which point bash already holds
/// a fresh reference, so the library stays mapped.
fn release_kept_library_reference() {
    if let Some(DlHandle(handle)) = KEPT_LIBRARY_HANDLE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
    {
        unsafe {
            libc::dlclose(handle);
        }
        log::info!("Released kept flyline library reference on reload");
    }
}

#[repr(C)]
struct Dl_info {
    dli_fname: *const libc::c_char,
    dli_fbase: *mut libc::c_void,
    dli_sname: *const libc::c_char,
    dli_saddr: *mut libc::c_void,
}

unsafe extern "C" {
    fn dladdr(addr: *const libc::c_void, info: *mut Dl_info) -> libc::c_int;
}

fn get_library_directory() -> Option<std::path::PathBuf> {
    unsafe {
        let mut info = std::mem::zeroed::<Dl_info>();
        let addr = flyline_load_common as *const libc::c_void;
        if dladdr(addr, &mut info) != 0 && !info.dli_fname.is_null() {
            let path_str = std::ffi::CStr::from_ptr(info.dli_fname).to_string_lossy();
            let path = std::path::Path::new(path_str.as_ref());
            if let Some(parent) = path.parent() {
                return Some(parent.to_path_buf());
            }
        }
    }
    None
}

/// Resets `SIGCHLD` disposition to `SIG_DFL` (default action) using `sigaction(2)`.
///
/// Bash frequently installs its own `SIGCHLD` handler (e.g. during prompt expansion
/// or command substitution execution), which interferes with process spawning in Rust
/// (`std::process::Command`), causing child wait calls to fail with `ECHILD`.
///
/// Using `sigaction` with `SIG_DFL` ensures `SA_NOCLDWAIT` and custom signal handlers are cleared.
pub fn reset_sigchld() {
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = libc::SIG_DFL as usize;
        libc::sigaction(libc::SIGCHLD, &action, std::ptr::null_mut());
    }
}

/// An RAII guard that sets `SIGCHLD` to `SIG_DFL` upon creation, and restores
/// the previous signal disposition when dropped (even on panic or early return).
#[must_use]
pub struct SigchldGuard {
    prev_action: libc::sigaction,
}

impl SigchldGuard {
    /// Resets `SIGCHLD` to `SIG_DFL` and returns a guard that will restore
    /// the previous `SIGCHLD` disposition when dropped.
    pub fn new() -> Self {
        unsafe {
            let mut prev_action: libc::sigaction = std::mem::zeroed();
            let mut new_action: libc::sigaction = std::mem::zeroed();
            new_action.sa_sigaction = libc::SIG_DFL as usize;
            libc::sigaction(libc::SIGCHLD, &new_action, &mut prev_action);
            Self { prev_action }
        }
    }
}

impl Default for SigchldGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SigchldGuard {
    fn drop(&mut self) {
        unsafe {
            libc::sigaction(libc::SIGCHLD, &self.prev_action, std::ptr::null_mut());
        }
    }
}
