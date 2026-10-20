use super::history::record_history;
use super::scanner::DiscoveredPackage;
use crate::ui::{clack_outro, colors};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static ACTIVE_CHILD_PID: AtomicU32 = AtomicU32::new(0);
static CTRL_C_INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Execute a script/target from a discovered project using the detected (or overridden) toolchain
pub fn run_package_script(
    pkg: &DiscoveredPackage,
    script: &str,
    pm_override: Option<&str>,
    extra_args: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    record_history(pkg, script);

    let pm = pm_override.unwrap_or(&pkg.package_manager);
    let display_name = pkg.name.as_deref().unwrap_or(&pkg.id);

    // Resolve raw command description from pkg.scripts if available
    let raw_cmd = pkg
        .scripts
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(script))
        .map(|(_, c)| c.as_str())
        .unwrap_or(script);

    // Check if Java process-scoped JDK switching via Scoop applies
    let scoop_jdk = detect_required_jdk_from_framework(pkg.framework.as_deref())
        .and_then(|ver| resolve_scoop_jdk_home(&ver));

    let jdk_note = match scoop_jdk {
        Some((ref ver, ref home)) => format!(
            " {MUTED}[JDK {ver} → {home}]{RESET}",
            MUTED = colors::MUTED,
            RESET = colors::RESET,
            ver = ver,
            home = home.display()
        ),
        None => String::new(),
    };

    clack_outro(&format!(
        "Launching {BOLD}{CYAN}{pm} ({script}){RESET} in {BOLD_WHITE}{display_name}{RESET} {MUTED}({dir}){RESET}{jdk_note}",
        BOLD = colors::BOLD,
        CYAN = colors::CYAN,
        BOLD_WHITE = colors::BOLD_WHITE,
        MUTED = colors::MUTED,
        RESET = colors::RESET,
        dir = pkg.dir.display(),
    ));

    let mut child_cmd = build_toolchain_command(pm, script, raw_cmd, extra_args);

    if let Some((_, ref java_home)) = scoop_jdk {
        let java_bin = java_home.join("bin");
        let current_path = std::env::var_os("PATH").unwrap_or_default();
        let mut paths = vec![java_bin];
        paths.extend(std::env::split_paths(&current_path));
        if let Ok(joined) = std::env::join_paths(paths) {
            child_cmd.env("PATH", joined);
        }
        child_cmd.env("JAVA_HOME", java_home);
    }

    #[cfg(windows)]
    win_process_guard::ensure_console_ctrl_c_enabled();

    let mut child = child_cmd
        .current_dir(&pkg.dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| {
            format!(
                "Failed to spawn '{} ({})' in {}: {}",
                pm,
                script,
                pkg.dir.display(),
                e
            )
        })?;

    let child_pid = child.id();
    CTRL_C_INTERRUPTED.store(false, Ordering::SeqCst);
    ACTIVE_CHILD_PID.store(child_pid, Ordering::SeqCst);

    #[cfg(windows)]
    let _guard = win_process_guard::ChildTreeGuard::attach(&child);

    let wait_res = child.wait();

    // Ensure any remaining child/grandchild processes are terminated when the command exits
    let remaining_pid = ACTIVE_CHILD_PID.swap(0, Ordering::SeqCst);
    let interrupted = CTRL_C_INTERRUPTED.swap(false, Ordering::SeqCst);

    #[cfg(windows)]
    drop(_guard);

    if interrupted && remaining_pid != 0 {
        kill_process_tree(remaining_pid);
    }

    let status = wait_res.map_err(|e| {
        format!(
            "Error while waiting for '{} ({})' (PID {}): {}",
            pm, script, child_pid, e
        )
    })?;

    if interrupted {
        println!();
        clack_outro(&format!(
            "{AMBER}■{RESET} Stopped {BOLD}{pm} ({script}){RESET} {MUTED}— terminated process tree (PID {child_pid}){RESET}",
            AMBER = colors::AMBER,
            BOLD = colors::BOLD,
            MUTED = colors::MUTED,
            RESET = colors::RESET,
        ));
        return Ok(());
    }

    if !status.success() {
        let code = status.code().unwrap_or(1);
        // Ignore standard Ctrl+C exit code on Windows (-1073741510 / 0xc000013a or 130)
        if code == 130 || code == -1073741510 {
            println!();
            clack_outro(&format!(
                "{AMBER}■{RESET} Stopped {BOLD}{pm} ({script}){RESET} {MUTED}— terminated process tree (PID {child_pid}){RESET}",
                AMBER = colors::AMBER,
                BOLD = colors::BOLD,
                MUTED = colors::MUTED,
                RESET = colors::RESET,
            ));
            return Ok(());
        }
        return Err(format!(
            "Command '{} ({})' exited with status code {}",
            pm, script, code
        )
        .into());
    }

    Ok(())
}

pub fn kill_process_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn build_toolchain_command(
    pm: &str,
    script: &str,
    raw_cmd: &str,
    extra_args: &[String],
) -> Command {
    match pm {
        "cargo" => {
            let mut c = spawn_shell_cmd("cargo");
            let sub_args: Vec<&str> = raw_cmd
                .strip_prefix("cargo ")
                .unwrap_or(script)
                .split_whitespace()
                .collect();
            c.args(&sub_args);
            if !extra_args.is_empty() {
                c.arg("--").args(extra_args);
            }
            c
        }
        "mvn" | "mvnw" => {
            let exe = if cfg!(windows) && pm == "mvnw" {
                ".\\mvnw.cmd"
            } else if pm == "mvnw" {
                "./mvnw"
            } else {
                "mvn"
            };
            let mut c = spawn_shell_cmd(exe);
            let sub_args: Vec<&str> = raw_cmd.split_whitespace().collect();
            c.args(&sub_args);
            c.args(extra_args);
            c
        }
        "gradle" | "gradlew" => {
            let exe = if cfg!(windows) && pm == "gradlew" {
                ".\\gradlew.bat"
            } else if pm == "gradlew" {
                "./gradlew"
            } else {
                "gradle"
            };
            let mut c = spawn_shell_cmd(exe);
            let sub_args: Vec<&str> = raw_cmd.split_whitespace().collect();
            c.args(&sub_args);
            c.args(extra_args);
            c
        }
        "docker" => {
            let mut c = spawn_shell_cmd("docker");
            let sub_args: Vec<&str> = raw_cmd
                .strip_prefix("docker ")
                .unwrap_or(raw_cmd)
                .split_whitespace()
                .collect();
            c.args(&sub_args);
            c.args(extra_args);
            c
        }
        _ => {
            let mut c = spawn_shell_cmd(pm);
            c.arg("run").arg(script);
            if !extra_args.is_empty() {
                c.arg("--").args(extra_args);
            }
            c
        }
    }
}

fn spawn_shell_cmd(program: &str) -> Command {
    #[cfg(target_os = "windows")]
    {
        // Prefer direct .exe execution (e.g. pnpm.exe, cargo.exe, bun.exe, docker.exe)
        // so cmd.exe never intercepts Ctrl+C with "Terminate batch job (Y/N)?"
        if let Some(exe_path) = resolve_windows_direct_exe(program) {
            return Command::new(exe_path);
        }
        let mut c = Command::new("cmd");
        c.arg("/C").arg(program);
        c
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new(program)
    }
}

#[cfg(target_os = "windows")]
fn resolve_windows_direct_exe(program: &str) -> Option<PathBuf> {
    if program.contains('\\')
        || program.contains('/')
        || program.ends_with(".cmd")
        || program.ends_with(".bat")
    {
        return None;
    }
    let exe_name = if program.to_lowercase().ends_with(".exe") {
        program.to_string()
    } else {
        format!("{}.exe", program)
    };
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(&exe_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn detect_required_jdk_from_framework(framework: Option<&str>) -> Option<String> {
    let fw = framework?;
    let idx = fw.find("JDK ")?;
    let rest = &fw[idx + 4..];
    let ver: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if ver.is_empty() { None } else { Some(ver) }
}

/// Locate a matching JDK installation inside `~/scoop/apps/` without modifying global environment variables
pub fn resolve_scoop_jdk_home(version: &str) -> Option<(String, PathBuf)> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .ok()?;
    let scoop_apps = Path::new(&home).join("scoop").join("apps");
    if !scoop_apps.is_dir() {
        return None;
    }

    let entries = std::fs::read_dir(&scoop_apps).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if (name.contains("jdk")
            || name.contains("temurin")
            || name.contains("zulu")
            || name.contains("graalvm"))
            && name.contains(version)
        {
            let current_dir = entry.path().join("current");
            if current_dir.join("bin").is_dir() {
                return Some((version.to_string(), current_dir));
            }
        }
    }
    None
}

#[cfg(windows)]
#[allow(non_snake_case, clippy::upper_case_acronyms)]
mod win_process_guard {
    use super::{ACTIVE_CHILD_PID, CTRL_C_INTERRUPTED, kill_process_tree};
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ACTIVE_JOB_HANDLE: AtomicUsize = AtomicUsize::new(0);

    type HANDLE = *mut c_void;
    type BOOL = i32;
    type DWORD = u32;

    const STD_INPUT_HANDLE: DWORD = 0xFFFFFFF6;
    const ENABLE_PROCESSED_INPUT: DWORD = 0x0001;
    const ENABLE_LINE_INPUT: DWORD = 0x0002;
    const ENABLE_ECHO_INPUT: DWORD = 0x0004;

    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: u32 = 9;
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x00002000;

    #[repr(C)]
    #[derive(Default)]
    struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
        PerProcessUserTimeLimit: i64,
        PerJobUserTimeLimit: i64,
        LimitFlags: DWORD,
        MinimumWorkingSetSize: usize,
        MaximumWorkingSetSize: usize,
        ActiveProcessLimit: DWORD,
        Affinity: usize,
        PriorityClass: DWORD,
        SchedulingClass: DWORD,
    }

    #[repr(C)]
    #[derive(Default)]
    struct IO_COUNTERS {
        ReadOperationCount: u64,
        WriteOperationCount: u64,
        OtherOperationCount: u64,
        ReadTransferCount: u64,
        WriteTransferCount: u64,
        OtherTransferCount: u64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
        BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION,
        IoInfo: IO_COUNTERS,
        ProcessMemoryLimit: usize,
        JobMemoryLimit: usize,
        PeakProcessMemoryUsed: usize,
        PeakJobMemoryUsed: usize,
    }

    unsafe extern "system" {
        fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;
        fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *mut DWORD) -> BOOL;
        fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: DWORD) -> BOOL;
        fn SetConsoleCtrlHandler(
            HandlerRoutine: Option<unsafe extern "system" fn(DWORD) -> BOOL>,
            Add: BOOL,
        ) -> BOOL;
        fn CreateJobObjectW(lpJobAttributes: *const c_void, lpName: *const u16) -> HANDLE;
        fn SetInformationJobObject(
            hJob: HANDLE,
            JobObjectInfoClass: u32,
            lpJobObjectInfo: *const c_void,
            cbJobObjectInfoLength: DWORD,
        ) -> BOOL;
        fn AssignProcessToJobObject(hJob: HANDLE, hProcess: HANDLE) -> BOOL;
        fn TerminateJobObject(hJob: HANDLE, uExitCode: u32) -> BOOL;
        fn CloseHandle(hObject: HANDLE) -> BOOL;
    }

    pub fn ensure_console_ctrl_c_enabled() {
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE);
            if !handle.is_null() && handle as isize != -1 {
                let mut mode: DWORD = 0;
                if GetConsoleMode(handle, &mut mode) != 0 {
                    SetConsoleMode(
                        handle,
                        mode | ENABLE_PROCESSED_INPUT | ENABLE_LINE_INPUT | ENABLE_ECHO_INPUT,
                    );
                }
            }
        }
    }

    unsafe extern "system" fn console_ctrl_handler(ctrl_type: DWORD) -> BOOL {
        // 0 = CTRL_C_EVENT, 1 = CTRL_BREAK_EVENT, 2 = CTRL_CLOSE_EVENT
        if ctrl_type <= 2 {
            CTRL_C_INTERRUPTED.store(true, Ordering::SeqCst);

            let mut killed_by_job = false;
            let job_raw = ACTIVE_JOB_HANDLE.swap(0, Ordering::SeqCst);
            if job_raw != 0 {
                unsafe {
                    let job = job_raw as HANDLE;
                    if TerminateJobObject(job, 130) != 0 {
                        killed_by_job = true;
                    }
                    CloseHandle(job);
                }
            }

            let pid = ACTIVE_CHILD_PID.swap(0, Ordering::SeqCst);
            if !killed_by_job && pid != 0 {
                kill_process_tree(pid);
            }
            return 1; // Handled: do not terminate tq.exe before child cleanup finishes
        }
        0
    }

    pub struct ChildTreeGuard {
        handler_registered: bool,
    }

    impl ChildTreeGuard {
        pub fn attach(child: &Child) -> Self {
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if !job.is_null() {
                    let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                    info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                    let ok = SetInformationJobObject(
                        job,
                        JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                        &info as *const _ as *const c_void,
                        std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as DWORD,
                    );
                    if ok != 0
                        && AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) != 0
                    {
                        ACTIVE_JOB_HANDLE.store(job as usize, Ordering::SeqCst);
                    } else {
                        CloseHandle(job);
                    }
                }

                let registered = SetConsoleCtrlHandler(Some(console_ctrl_handler), 1) != 0;
                Self {
                    handler_registered: registered,
                }
            }
        }
    }

    impl Drop for ChildTreeGuard {
        fn drop(&mut self) {
            unsafe {
                if self.handler_registered {
                    SetConsoleCtrlHandler(Some(console_ctrl_handler), 0);
                }
                let job_raw = ACTIVE_JOB_HANDLE.swap(0, Ordering::SeqCst);
                if job_raw != 0 {
                    let job = job_raw as HANDLE;
                    TerminateJobObject(job, 130);
                    CloseHandle(job);
                }
            }
            ensure_console_ctrl_c_enabled();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_required_jdk_from_framework() {
        assert_eq!(
            detect_required_jdk_from_framework(Some("Spring Boot (JDK 17)")),
            Some("17".to_string())
        );
        assert_eq!(
            detect_required_jdk_from_framework(Some("Maven (JDK 21)")),
            Some("21".to_string())
        );
        assert_eq!(detect_required_jdk_from_framework(Some("Vite")), None);
        assert_eq!(detect_required_jdk_from_framework(None), None);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_job_object_terminates_child_tree() {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;

        let mut child = Command::new("cmd")
            .args(["/C", "ping 127.0.0.1 -n 30 > nul"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .expect("spawn ping");

        let guard = win_process_guard::ChildTreeGuard::attach(&child);
        // Dropping the guard must immediately terminate the entire job tree
        drop(guard);

        let status = child.wait().expect("wait after job drop");
        assert!(
            !status.success(),
            "Child process in job should have been terminated"
        );
    }
}
