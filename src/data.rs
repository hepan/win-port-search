use std::collections::{BTreeMap, HashMap};
use std::ffi::c_void;
use std::mem::{offset_of, size_of};
use std::ptr;
use std::rc::Rc;

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_INSUFFICIENT_BUFFER, ERROR_NO_MORE_FILES, GetLastError, HANDLE,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID,
    MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, MIB_UDP6ROW_OWNER_PID, MIB_UDP6TABLE_OWNER_PID,
    MIB_UDPROW_OWNER_PID, MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    UDP_TABLE_OWNER_PID,
};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};

#[derive(Clone)]
pub struct DisplayRow {
    pub protocol: &'static str,
    pub port: u16,
    pub pid: u32,
    pub count: usize,
    pub name: Rc<str>,
    pub path: Rc<str>,
}

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

// IP Helper reports each port in the low 16 bits of a DWORD, in network byte order.
fn local_port(raw: u32) -> u16 {
    u16::from_be(raw as u16)
}

fn query_table(buffer: &mut Vec<u8>, tcp: bool, family: u32, label: &str) -> Result<usize, String> {
    if buffer.is_empty() {
        buffer.resize(16 * 1024, 0);
    }

    for _ in 0..12 {
        let mut bytes = buffer.len() as u32;
        let status = unsafe {
            if tcp {
                GetExtendedTcpTable(
                    buffer.as_mut_ptr().cast::<c_void>(),
                    &mut bytes,
                    0,
                    family,
                    TCP_TABLE_OWNER_PID_LISTENER,
                    0,
                )
            } else {
                GetExtendedUdpTable(
                    buffer.as_mut_ptr().cast::<c_void>(),
                    &mut bytes,
                    0,
                    family,
                    UDP_TABLE_OWNER_PID,
                    0,
                )
            }
        };
        if status == 0 {
            let len = bytes as usize;
            if len > buffer.len() {
                return Err(format!(
                    "{label}: API returned a size larger than the buffer"
                ));
            }
            return Ok(len);
        }
        if status != ERROR_INSUFFICIENT_BUFFER {
            return Err(format!("{label}: Windows error {status}"));
        }

        // The table can grow between the size probe and the subsequent call.
        let next = (bytes as usize).max(buffer.len().saturating_mul(2));
        if next <= buffer.len() || next > u32::MAX as usize {
            return Err(format!("{label}: endpoint table is too large"));
        }
        buffer
            .try_reserve_exact(next - buffer.len())
            .map_err(|err| format!("{label}: cannot allocate endpoint table: {err}"))?;
        buffer.resize(next, 0);
    }
    Err(format!(
        "{label}: endpoint table kept changing while reading it"
    ))
}

fn add_rows<Row>(
    bytes: &[u8],
    first_row: usize,
    port_field: usize,
    pid_field: usize,
    protocol: &'static str,
    label: &str,
    groups: &mut BTreeMap<(&'static str, u16, u32), usize>,
) -> Result<(), String> {
    let row_size = size_of::<Row>();
    if bytes.len() < first_row
        || first_row < size_of::<u32>()
        || row_size < size_of::<u32>()
        || port_field > row_size - size_of::<u32>()
        || pid_field > row_size - size_of::<u32>()
    {
        return Err(format!("{label}: malformed endpoint table header"));
    }
    // Both the DWORD header and row fields may be unaligned in a byte buffer.
    let count = unsafe { ptr::read_unaligned(bytes.as_ptr().cast::<u32>()) } as usize;
    if count > (bytes.len() - first_row) / row_size {
        return Err(format!("{label}: endpoint table row count exceeds buffer"));
    }
    for index in 0..count {
        let row = unsafe { bytes.as_ptr().add(first_row + index * row_size) };
        let port = unsafe { ptr::read_unaligned(row.add(port_field).cast::<u32>()) };
        let pid = unsafe { ptr::read_unaligned(row.add(pid_field).cast::<u32>()) };
        *groups.entry((protocol, local_port(port), pid)).or_default() += 1;
    }
    Ok(())
}

fn process_image_path(pid: u32) -> Result<String, u32> {
    let raw = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if raw.is_null() {
        return Err(unsafe { GetLastError() });
    }
    let handle = OwnedHandle(raw);
    let mut buffer = vec![0u16; 260];
    loop {
        let mut size = buffer.len() as u32;
        if unsafe {
            QueryFullProcessImageNameW(handle.0, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut size)
        } != 0
        {
            return Ok(String::from_utf16_lossy(&buffer[..size as usize]));
        }
        let error = unsafe { GetLastError() };
        if error != ERROR_INSUFFICIENT_BUFFER || buffer.len() >= 32_768 {
            return Err(error);
        }
        buffer.resize((buffer.len() * 2).min(32_768), 0);
    }
}

fn snapshot_process_names() -> Result<HashMap<u32, String>, u32> {
    let raw = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if raw == INVALID_HANDLE_VALUE {
        return Err(unsafe { GetLastError() });
    }
    let _snapshot = OwnedHandle(raw);
    let mut entry = PROCESSENTRY32W::default();
    entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut names = HashMap::new();
    if unsafe { Process32FirstW(raw, &mut entry) } == 0 {
        return Err(unsafe { GetLastError() });
    }
    loop {
        let length = entry
            .szExeFile
            .iter()
            .position(|&character| character == 0)
            .unwrap_or(entry.szExeFile.len());
        names.insert(
            entry.th32ProcessID,
            String::from_utf16_lossy(&entry.szExeFile[..length]),
        );
        if unsafe { Process32NextW(raw, &mut entry) } == 0 {
            let error = unsafe { GetLastError() };
            return if error == ERROR_NO_MORE_FILES {
                Ok(names)
            } else {
                Err(error)
            };
        }
    }
}

pub fn collect() -> Result<Vec<DisplayRow>, String> {
    let mut groups = BTreeMap::new();
    let mut buffer = Vec::new();

    let len = query_table(&mut buffer, true, AF_INET as u32, "TCP IPv4")?;
    add_rows::<MIB_TCPROW_OWNER_PID>(
        &buffer[..len],
        offset_of!(MIB_TCPTABLE_OWNER_PID, table),
        offset_of!(MIB_TCPROW_OWNER_PID, dwLocalPort),
        offset_of!(MIB_TCPROW_OWNER_PID, dwOwningPid),
        "TCP",
        "TCP IPv4",
        &mut groups,
    )?;
    let len = query_table(&mut buffer, true, AF_INET6 as u32, "TCP IPv6")?;
    add_rows::<MIB_TCP6ROW_OWNER_PID>(
        &buffer[..len],
        offset_of!(MIB_TCP6TABLE_OWNER_PID, table),
        offset_of!(MIB_TCP6ROW_OWNER_PID, dwLocalPort),
        offset_of!(MIB_TCP6ROW_OWNER_PID, dwOwningPid),
        "TCP",
        "TCP IPv6",
        &mut groups,
    )?;
    let len = query_table(&mut buffer, false, AF_INET as u32, "UDP IPv4")?;
    add_rows::<MIB_UDPROW_OWNER_PID>(
        &buffer[..len],
        offset_of!(MIB_UDPTABLE_OWNER_PID, table),
        offset_of!(MIB_UDPROW_OWNER_PID, dwLocalPort),
        offset_of!(MIB_UDPROW_OWNER_PID, dwOwningPid),
        "UDP",
        "UDP IPv4",
        &mut groups,
    )?;
    let len = query_table(&mut buffer, false, AF_INET6 as u32, "UDP IPv6")?;
    add_rows::<MIB_UDP6ROW_OWNER_PID>(
        &buffer[..len],
        offset_of!(MIB_UDP6TABLE_OWNER_PID, table),
        offset_of!(MIB_UDP6ROW_OWNER_PID, dwLocalPort),
        offset_of!(MIB_UDP6ROW_OWNER_PID, dwOwningPid),
        "UDP",
        "UDP IPv6",
        &mut groups,
    )?;

    // Ask for the full path once per distinct PID. A single process snapshot provides
    // filenames for processes whose image path cannot be queried (including protected ones).
    let mut images = HashMap::<u32, Result<String, u32>>::new();
    for &(_, _, pid) in groups.keys() {
        if pid != 0 {
            images.entry(pid).or_insert_with(|| process_image_path(pid));
        }
    }
    let names = if images.values().any(Result::is_err) {
        Some(snapshot_process_names())
    } else {
        None
    };

    // Multiple ports may belong to one process; share its metadata rather than
    // allocating another copy of the image path for each displayed row.
    let details: HashMap<u32, (Rc<str>, Rc<str>)> = images
        .into_iter()
        .map(|(pid, image)| {
            let pair = match image {
                Ok(image) => (
                    Rc::from(image.rsplit(['\\', '/']).next().unwrap_or(&image)),
                    Rc::from(image),
                ),
                Err(error) => {
                    let name = match &names {
                        Some(Ok(names)) => names
                            .get(&pid)
                            .map(|name| Rc::from(name.as_str()))
                            .unwrap_or_else(|| Rc::from("Unknown (process not in snapshot)")),
                        Some(Err(snapshot_error)) => {
                            Rc::from(format!("Unknown (snapshot error {snapshot_error})"))
                        }
                        None => Rc::from("Unknown (process name unavailable)"),
                    };
                    (
                        name,
                        Rc::from(format!("Unavailable: Windows error {error}")),
                    )
                }
            };
            (pid, pair)
        })
        .collect();
    let unknown = (
        Rc::from("Unknown owner (PID 0)"),
        Rc::from("Unavailable: no owning process"),
    );
    Ok(groups
        .into_iter()
        .map(|((protocol, port, pid), count)| {
            let (name, path) = if pid == 0 {
                unknown.clone()
            } else {
                details[&pid].clone()
            };
            DisplayRow {
                protocol,
                port,
                pid,
                count,
                name,
                path,
            }
        })
        .collect())
}
