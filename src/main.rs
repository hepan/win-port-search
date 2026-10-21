#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("win-port-search builds for Windows targets only");

#[cfg(windows)]
mod data;
#[cfg(windows)]
mod gui;

#[cfg(windows)]
fn main() {
    if let Err(message) = gui::run() {
        let text: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
        let title: Vec<u16> = "端口查看器"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // The GUI subsystem has no console, so startup errors need a visible message.
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
            );
        }
    }
}
