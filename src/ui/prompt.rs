use std::io::{self, IsTerminal, Write};

/// Prompt user for standard text input with an optional default value
pub fn prompt_input(label: &str, default: Option<&str>) -> io::Result<String> {
    if let Some(def) = default {
        print!("  \x1b[1;36m?\x1b[0m {} \x1b[90m[{}]\x1b[0m: ", label, def);
    } else {
        print!("  \x1b[1;36m?\x1b[0m {}: ", label);
    }
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let trimmed = line.trim().to_string();

    if trimmed.is_empty() && let Some(def) = default {
        Ok(def.to_string())
    } else {
        Ok(trimmed)
    }
}

/// Prompt user for password input with terminal echoing disabled
pub fn prompt_password(label: &str) -> io::Result<String> {
    print!("  \x1b[1;36m?\x1b[0m {}: ", label);
    io::stdout().flush()?;

    if !io::stdin().is_terminal() {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        return Ok(line.trim().to_string());
    }

    let password = platform_impl::read_hidden()?;
    println!();
    Ok(password.trim().to_string())
}

/// Display a numbered choice list and prompt user to select an option
pub fn prompt_select(label: &str, options: &[&str], default_idx: usize) -> io::Result<usize> {
    println!("\n  \x1b[1;36m?\x1b[0m {}", label);
    for (i, opt) in options.iter().enumerate() {
        let marker = if i == default_idx { "\x1b[32m>\x1b[0m" } else { " " };
        println!("   {} \x1b[1m{})\x1b[0m {}", marker, i + 1, opt);
    }

    loop {
        print!("  Select [1-{}, default {}]: ", options.len(), default_idx + 1);
        io::stdout().flush()?;

        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return Ok(default_idx);
        }

        if let Ok(choice) = trimmed.parse::<usize>()
            && choice >= 1 && choice <= options.len() {
                return Ok(choice - 1);
            }

        println!("  \x1b[31mInvalid selection, please enter a number between 1 and {}.\x1b[0m", options.len());
    }
}

#[cfg(windows)]
#[allow(clippy::upper_case_acronyms)]
mod platform_impl {
    use std::io::{self, BufRead};

    type HANDLE = *mut std::ffi::c_void;
    type BOOL = i32;
    type DWORD = u32;

    const STD_INPUT_HANDLE: DWORD = 0xFFFFFFF6; // ((DWORD)-10)
    const ENABLE_ECHO_INPUT: DWORD = 0x0004;

    unsafe extern "system" {
        fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;
        fn GetConsoleMode(hConsoleHandle: HANDLE, lpMode: *mut DWORD) -> BOOL;
        fn SetConsoleMode(hConsoleHandle: HANDLE, dwMode: DWORD) -> BOOL;
    }

    pub fn read_hidden() -> io::Result<String> {
        unsafe {
            let handle = GetStdHandle(STD_INPUT_HANDLE);
            if handle.is_null() || handle as isize == -1 {
                let mut line = String::new();
                io::stdin().read_line(&mut line)?;
                return Ok(line);
            }

            let mut original_mode: DWORD = 0;
            if GetConsoleMode(handle, &mut original_mode) == 0 {
                let mut line = String::new();
                io::stdin().read_line(&mut line)?;
                return Ok(line);
            }

            // Disable echo input
            let new_mode = original_mode & !ENABLE_ECHO_INPUT;
            SetConsoleMode(handle, new_mode);

            let mut line = String::new();
            let stdin = io::stdin();
            let mut handle_reader = stdin.lock();
            let res = handle_reader.read_line(&mut line);

            // Restore original console mode
            SetConsoleMode(handle, original_mode);

            res.map(|_| line)
        }
    }
}

#[cfg(not(windows))]
mod platform_impl {
    use std::io::{self, BufRead};

    pub fn read_hidden() -> io::Result<String> {
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        Ok(line)
    }
}
