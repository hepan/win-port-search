use std::iter;
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HWND, RECT, SYSTEMTIME};
use windows_sys::Win32::Graphics::Gdi::{
    COLOR_WINDOW, DEFAULT_GUI_FONT, GetStockObject, GetSysColorBrush, UpdateWindow,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::UI::Controls::{
    ICC_LISTVIEW_CLASSES, INITCOMMONCONTROLSEX, InitCommonControlsEx, LVCF_TEXT, LVCF_WIDTH,
    LVCOLUMNW, LVIF_TEXT, LVITEMW, LVM_DELETEALLITEMS, LVM_INSERTCOLUMNW, LVM_INSERTITEMW,
    LVM_SETCOLUMNWIDTH, LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMW, LVS_EX_DOUBLEBUFFER,
    LVS_EX_FULLROWSELECT, LVS_EX_GRIDLINES, LVS_REPORT, LVS_SHOWSELALWAYS, WC_LISTVIEWW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, CREATESTRUCTW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, ES_AUTOHSCROLL, GWLP_USERDATA, GetClientRect, GetMessageW, GetWindowLongPtrW,
    GetWindowTextLengthW, GetWindowTextW, IDC_ARROW, LoadCursorW, MSG, MoveWindow, PostQuitMessage,
    RegisterClassW, SW_SHOWDEFAULT, SendMessageW, SetWindowLongPtrW, SetWindowTextW, ShowWindow,
    TranslateMessage, UnregisterClassW, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_NCCREATE,
    WM_NCDESTROY, WM_SETFONT, WM_SIZE, WNDCLASSW, WS_CHILD, WS_CLIPCHILDREN, WS_EX_CLIENTEDGE,
    WS_OVERLAPPEDWINDOW, WS_TABSTOP, WS_VISIBLE,
};
use windows_sys::core::w;

const SEARCH_ID: usize = 101;
const REFRESH_ID: usize = 102;
const ALL_ID: usize = 103;
const PORT_EDIT_ID: usize = 104;
const PORT_LABEL_ID: usize = 105;
const COLUMN_WIDTHS: [i32; 6] = [100, 80, 80, 190, 500, 80];

struct State {
    edit: HWND,
    list: HWND,
    status: HWND,
    filter: Option<u16>,
    last_result: String,
    creation_error: Option<String>,
    alive: bool,
}

impl State {
    fn new() -> Self {
        Self {
            edit: null_mut(),
            list: null_mut(),
            status: null_mut(),
            filter: None,
            last_result: "No query yet".to_owned(),
            creation_error: None,
            alive: false,
        }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(iter::once(0)).collect()
}

fn last_error(action: &str) -> String {
    format!("{action}: {}", std::io::Error::last_os_error())
}

fn clock_time() -> String {
    let mut time: SYSTEMTIME = unsafe { zeroed() };
    unsafe { GetLocalTime(&mut time) };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute, time.wSecond
    )
}

fn filter_label(filter: Option<u16>) -> String {
    match filter {
        Some(port) => format!("port {port}"),
        None => "all ports".to_owned(),
    }
}

// The caller owns State on its stack for the entire message loop. Never retain a
// Rust reference to it across a Win32 call: SendMessage and window creation can
// synchronously reenter this window procedure.
unsafe fn set_status(state: *mut State, text: &str) {
    let status = unsafe { (*state).status };
    if !status.is_null() {
        let text = wide(text);
        unsafe { SetWindowTextW(status, text.as_ptr()) };
    }
}

unsafe fn query(state: *mut State) {
    let filter = unsafe { (*state).filter };
    let list = unsafe { (*state).list };
    unsafe { set_status(state, &format!("Querying {}...", filter_label(filter))) };

    // Discard the previous snapshot even on failure: stale rows must not look
    // like a successful result for the newly applied filter.
    let result = crate::data::collect();
    unsafe { SendMessageW(list, LVM_DELETEALLITEMS, 0, 0) };
    let stamp = clock_time();
    let summary = match result {
        Ok(rows) => {
            let mut displayed = 0usize;
            let mut text = Vec::new();
            let mut insertion_error = None;
            for row in rows
                .iter()
                .filter(|row| filter.map_or(true, |port| row.port == port))
            {
                if displayed >= i32::MAX as usize {
                    insertion_error = Some("Too many rows for the table".to_owned());
                    break;
                }
                let port = row.port.to_string();
                let pid = row.pid.to_string();
                let count = row.count.to_string();
                let cells = [row.protocol, &port, &pid, &row.name, &row.path, &count];
                let mut item: LVITEMW = unsafe { zeroed() };
                item.mask = LVIF_TEXT;
                item.iItem = displayed as i32;
                text.extend(cells[0].encode_utf16());
                text.push(0);
                item.pszText = text.as_mut_ptr();
                let inserted =
                    unsafe { SendMessageW(list, LVM_INSERTITEMW, 0, &item as *const _ as isize) };
                text.clear();
                if inserted < 0 {
                    insertion_error = Some("Could not insert table row".to_owned());
                    break;
                }
                for (column, cell) in cells.iter().enumerate().skip(1) {
                    item.iItem = inserted as i32;
                    item.iSubItem = column as i32;
                    text.extend(cell.encode_utf16());
                    text.push(0);
                    item.pszText = text.as_mut_ptr();
                    let updated =
                        unsafe { SendMessageW(list, LVM_SETITEMW, 0, &item as *const _ as isize) };
                    text.clear();
                    if updated == 0 {
                        insertion_error = Some("Could not populate table row".to_owned());
                        break;
                    }
                }
                if insertion_error.is_some() {
                    break;
                }
                displayed += 1;
            }
            match insertion_error {
                Some(error) => {
                    unsafe { SendMessageW(list, LVM_DELETEALLITEMS, 0, 0) };
                    format!(
                        "Query failed at {stamp}: {error}; table cleared ({})",
                        filter_label(filter)
                    )
                }
                None if displayed == 0 && filter.is_some() => {
                    format!(
                        "No matching port | {} | Updated {stamp}",
                        filter_label(filter)
                    )
                }
                None => format!(
                    "{displayed} grouped rows | {} | Updated {stamp}",
                    filter_label(filter)
                ),
            }
        }
        Err(error) => format!(
            "Query failed at {stamp}: {error}; table cleared ({})",
            filter_label(filter)
        ),
    };
    unsafe { set_status(state, &summary) };
    unsafe { (*state).last_result = summary };
}

unsafe fn search(state: *mut State) {
    let edit = unsafe { (*state).edit };
    let capacity = unsafe { GetWindowTextLengthW(edit) }.max(0) as usize + 1;
    let mut buffer = vec![0u16; capacity];
    let length = unsafe { GetWindowTextW(edit, buffer.as_mut_ptr(), capacity as i32) };
    let input = String::from_utf16_lossy(&buffer[..length.max(0) as usize]);
    if input.is_empty() {
        unsafe { (*state).filter = None };
        unsafe { query(state) };
        return;
    }
    let port = if input.bytes().all(|byte| byte.is_ascii_digit()) {
        input.parse::<u16>().ok()
    } else {
        None
    };
    match port {
        Some(port) => {
            unsafe { (*state).filter = Some(port) };
            unsafe { query(state) };
        }
        None => {
            let summary = format!(
                "Enter an exact port (0-65535). Results unchanged: {}",
                unsafe { &(*state).last_result }
            );
            unsafe { set_status(state, &summary) };
        }
    }
}

unsafe fn layout(hwnd: HWND, state: *mut State) {
    let mut rect: RECT = unsafe { zeroed() };
    if unsafe { GetClientRect(hwnd, &mut rect) } == 0 {
        return;
    }
    let width = rect.right.max(0);
    let height = rect.bottom.max(0);
    let edit = unsafe { (*state).edit };
    let list = unsafe { (*state).list };
    let status = unsafe { (*state).status };
    if edit.is_null() || list.is_null() || status.is_null() {
        return;
    }
    // A compact top row leaves the rest of the client area for the report.
    unsafe {
        MoveWindow(
            windows_sys::Win32::UI::WindowsAndMessaging::GetDlgItem(hwnd, PORT_LABEL_ID as i32),
            10,
            14,
            48,
            20,
            1,
        );
        MoveWindow(edit, 64, 10, 104, 25, 1);
        MoveWindow(
            windows_sys::Win32::UI::WindowsAndMessaging::GetDlgItem(hwnd, SEARCH_ID as i32),
            176,
            10,
            76,
            25,
            1,
        );
        MoveWindow(
            windows_sys::Win32::UI::WindowsAndMessaging::GetDlgItem(hwnd, REFRESH_ID as i32),
            260,
            10,
            80,
            25,
            1,
        );
        MoveWindow(
            windows_sys::Win32::UI::WindowsAndMessaging::GetDlgItem(hwnd, ALL_ID as i32),
            348,
            10,
            72,
            25,
            1,
        );
        MoveWindow(list, 10, 44, (width - 20).max(0), (height - 78).max(0), 1);
        MoveWindow(status, 10, (height - 29).max(0), (width - 20).max(0), 20, 1);
        SendMessageW(
            list,
            LVM_SETCOLUMNWIDTH,
            4,
            (width
                - 20
                - COLUMN_WIDTHS[0]
                - COLUMN_WIDTHS[1]
                - COLUMN_WIDTHS[2]
                - COLUMN_WIDTHS[3]
                - COLUMN_WIDTHS[5])
                .max(350) as isize,
        );
    }
}

unsafe fn child(
    parent: HWND,
    instance: windows_sys::Win32::Foundation::HINSTANCE,
    class: *const u16,
    caption: *const u16,
    style: u32,
    ex_style: u32,
    id: usize,
) -> Result<HWND, String> {
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style,
            class,
            caption,
            style | WS_CHILD | WS_VISIBLE,
            0,
            0,
            0,
            0,
            parent,
            id as _,
            instance,
            null(),
        )
    };
    if hwnd.is_null() {
        Err(last_error("Create control"))
    } else {
        let font = unsafe { GetStockObject(DEFAULT_GUI_FONT) };
        unsafe { SendMessageW(hwnd, WM_SETFONT, font as usize, 1) };
        Ok(hwnd)
    }
}

unsafe fn create_controls(hwnd: HWND, state: *mut State) -> Result<(), String> {
    let instance = unsafe { GetModuleHandleW(null()) };
    if instance.is_null() {
        return Err(last_error("GetModuleHandleW"));
    }
    unsafe {
        child(
            hwnd,
            instance,
            w!("STATIC"),
            w!("Port:"),
            0,
            0,
            PORT_LABEL_ID,
        )?;
        let edit = child(
            hwnd,
            instance,
            w!("EDIT"),
            w!(""),
            WS_TABSTOP | ES_AUTOHSCROLL as u32,
            WS_EX_CLIENTEDGE,
            PORT_EDIT_ID,
        )?;
        (*state).edit = edit;
        child(
            hwnd,
            instance,
            w!("BUTTON"),
            w!("Search"),
            WS_TABSTOP,
            0,
            SEARCH_ID,
        )?;
        child(
            hwnd,
            instance,
            w!("BUTTON"),
            w!("Refresh"),
            WS_TABSTOP,
            0,
            REFRESH_ID,
        )?;
        child(
            hwnd,
            instance,
            w!("BUTTON"),
            w!("All"),
            WS_TABSTOP,
            0,
            ALL_ID,
        )?;
        let list = child(
            hwnd,
            instance,
            WC_LISTVIEWW,
            w!(""),
            WS_TABSTOP | LVS_REPORT | LVS_SHOWSELALWAYS,
            WS_EX_CLIENTEDGE,
            0,
        )?;
        (*state).list = list;
        let status = child(hwnd, instance, w!("STATIC"), w!("No query yet"), 0, 0, 0)?;
        (*state).status = status;
        SendMessageW(
            list,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            0,
            (LVS_EX_FULLROWSELECT | LVS_EX_GRIDLINES | LVS_EX_DOUBLEBUFFER) as isize,
        );
        let headers = [
            "Protocol",
            "Port",
            "PID",
            "Process",
            "Executable path",
            "Count",
        ];
        for (index, name) in headers.iter().enumerate() {
            let mut title = wide(name);
            let mut column: LVCOLUMNW = zeroed();
            column.mask = LVCF_TEXT | LVCF_WIDTH;
            column.cx = COLUMN_WIDTHS[index];
            column.pszText = title.as_mut_ptr();
            if SendMessageW(list, LVM_INSERTCOLUMNW, index, &column as *const _ as isize) < 0 {
                return Err("Could not create table columns".to_owned());
            }
        }
        layout(hwnd, state);
    }
    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    if message == WM_NCCREATE {
        let create = unsafe { &*(lparam as *const CREATESTRUCTW) };
        let state = create.lpCreateParams as *mut State;
        if state.is_null() {
            return 0;
        }
        unsafe { (*state).alive = true };
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize) };
        return 1;
    }
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State };
    match message {
        WM_CREATE => {
            if state.is_null() {
                return -1;
            }
            match unsafe { create_controls(hwnd, state) } {
                Ok(()) => 0,
                Err(error) => {
                    unsafe { (*state).creation_error = Some(error) };
                    -1
                }
            }
        }
        WM_SIZE if !state.is_null() => {
            unsafe { layout(hwnd, state) };
            0
        }
        WM_COMMAND if !state.is_null() && (wparam >> 16) == BN_CLICKED as usize => {
            match wparam & 0xffff {
                SEARCH_ID => unsafe { search(state) },
                REFRESH_ID => unsafe { query(state) },
                ALL_ID => {
                    unsafe { (*state).filter = None };
                    let edit = unsafe { (*state).edit };
                    unsafe { SetWindowTextW(edit, w!("")) };
                    unsafe { query(state) };
                }
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            0
        }
        WM_NCDESTROY => {
            // Detach before DefWindowProcW or any late child notification can
            // access the caller's state; the caller remains its sole owner.
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
            if !state.is_null() {
                unsafe { (*state).alive = false };
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

pub fn run() -> Result<(), String> {
    unsafe {
        let controls = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES,
        };
        if InitCommonControlsEx(&controls) == 0 {
            return Err(last_error("InitCommonControlsEx"));
        }
        let instance = GetModuleHandleW(null());
        if instance.is_null() {
            return Err(last_error("GetModuleHandleW"));
        }
        let class_name = w!("WinPortInspectorMainWindow");
        let class = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: GetSysColorBrush(COLOR_WINDOW),
            lpszMenuName: null(),
            lpszClassName: class_name,
        };
        if RegisterClassW(&class) == 0 {
            return Err(last_error("RegisterClassW"));
        }
        let mut state = State::new();
        let hwnd = CreateWindowExW(
            0,
            class_name,
            w!("Windows Port Inspector"),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1180,
            700,
            null_mut(),
            null_mut(),
            instance,
            &mut state as *mut State as _,
        );
        if hwnd.is_null() {
            let error = state
                .creation_error
                .take()
                .unwrap_or_else(|| last_error("CreateWindowExW"));
            UnregisterClassW(class_name, instance);
            return Err(error);
        }
        ShowWindow(hwnd, SW_SHOWDEFAULT);
        UpdateWindow(hwnd);
        query(&mut state);

        let mut message: MSG = zeroed();
        let result = loop {
            let next = GetMessageW(&mut message, null_mut(), 0, 0);
            if next < 0 {
                let error = last_error("GetMessageW");
                if state.alive {
                    DestroyWindow(hwnd);
                }
                break Err(error);
            }
            if next == 0 {
                break Ok(());
            }
            TranslateMessage(&message);
            DispatchMessageW(&message);
        };
        if state.alive {
            DestroyWindow(hwnd);
        }
        UnregisterClassW(class_name, instance);
        result
    }
}
