use std::ptr;

#[cfg(windows)]
pub fn is_admin() -> bool {
    use std::mem;
    use std::ptr;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use winapi::um::processthreadsapi::OpenProcessToken;
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};

    let mut token_handle = ptr::null_mut();
    unsafe {
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle) == 0 {
            return false;
        }
    }

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut return_length = 0;
    let elevation_size = mem::size_of::<TOKEN_ELEVATION>() as u32;

    let success = unsafe {
        GetTokenInformation(
            token_handle,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            elevation_size,
            &mut return_length,
        ) != 0
    };

    unsafe {
        CloseHandle(token_handle);
    }

    success && elevation.TokenIsElevated != 0
}

#[cfg(windows)]
pub fn relaunch_as_admin() -> anyhow::Result<()> {
    use std::env;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::process;
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_SHOW;

    let exe = env::current_exe()?;
    let exe_str: Vec<u16> = OsStr::new(&exe).encode_wide().chain(Some(0)).collect();
    let params_str: Vec<u16> = OsStr::new("").encode_wide().chain(Some(0)).collect();
    let verb_str: Vec<u16> = OsStr::new("runas").encode_wide().chain(Some(0)).collect();

    let result = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            verb_str.as_ptr(),
            exe_str.as_ptr(),
            params_str.as_ptr(),
            ptr::null_mut(),
            SW_SHOW,
        )
    };

    if result as usize > 32 {
        process::exit(0);
    } else {
        return Err(anyhow::anyhow!("Failed to relaunch as admin"));
    }
}
