use anyhow::{Context, Result};
use std::process::Command;

/// A sort of `exec` implementation. Windows does not really have `exec` so we
/// are wrapping the executable to run and returning it's exit code (passing all
/// signals into the child process).
pub fn exec(mut command: Command) -> Result<i32> {
    imp::wrap_exec(&mut command).context(format!("Executing command: {:?}", &command.get_program()))
}

// The idea for this wrapping of executable was taken from cargo-utils
#[cfg(windows)]
pub(super) mod imp {
    use anyhow::{anyhow, Result};
    use std::process::Command;
    use winapi::shared::minwindef::{BOOL, DWORD, FALSE, TRUE};
    use winapi::um::consoleapi::SetConsoleCtrlHandler;

    unsafe extern "system" fn ctrlc_handler(_: DWORD) -> BOOL {
        // Do nothing, pass everything to child process
        TRUE
    }

    pub(super) fn wrap_exec(command: &mut Command) -> Result<i32> {
        unsafe {
            if SetConsoleCtrlHandler(Some(ctrlc_handler), TRUE) == FALSE {
                return Err(anyhow!("Could not set Ctrl-C handler."));
            }
        }

        // Acts under the (possibly false) assumption that if status returns an
        // error it means that it didn't run. If the process has ran it will
        // return status.
        match command.status() {
            Err(err) => Err(anyhow!(err)),
            Ok(status) => {
                if status.success() {
                    Ok(0)
                } else {
                    Ok(status.code().unwrap_or(-1))
                }
            }
        }
    }
}
