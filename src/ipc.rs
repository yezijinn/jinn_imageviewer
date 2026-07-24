// ============================================================================
// 命名管道 IPC — 单实例文件路径转发
//
// 当用户双击图片文件时，第二个实例通过命名管道将文件路径发送给
// 正在运行的主实例，然后立即退出。主实例的后台线程接收路径后
// 通过 mpsc 通道转发给 UI 线程处理。
// ============================================================================

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::mpsc;

// ============================================================================
// Windows FFI declarations
// ============================================================================

#[allow(non_snake_case)]
extern "system" {
    fn CreateNamedPipeW(
        lpName: *const u16,
        dwOpenMode: u32,
        dwPipeMode: u32,
        nMaxInstances: u32,
        nOutBufferSize: u32,
        nInBufferSize: u32,
        nDefaultTimeOut: u32,
        lpSecurityAttributes: *const std::ffi::c_void,
    ) -> isize;

    fn ConnectNamedPipe(hNamedPipe: isize, lpOverlapped: *mut std::ffi::c_void) -> i32;

    fn DisconnectNamedPipe(hNamedPipe: isize) -> i32;

    fn ReadFile(
        hFile: isize,
        lpBuffer: *mut std::ffi::c_void,
        nNumberOfBytesToRead: u32,
        lpNumberOfBytesRead: *mut u32,
        lpOverlapped: *mut std::ffi::c_void,
    ) -> i32;

    fn WriteFile(
        hFile: isize,
        lpBuffer: *const std::ffi::c_void,
        nNumberOfBytesToWrite: u32,
        lpNumberOfBytesWritten: *mut u32,
        lpOverlapped: *mut std::ffi::c_void,
    ) -> i32;

    fn CreateFileW(
        lpFileName: *const u16,
        dwDesiredAccess: u32,
        dwShareMode: u32,
        lpSecurityAttributes: *const std::ffi::c_void,
        dwCreationDisposition: u32,
        dwFlagsAndAttributes: u32,
        hTemplateFile: isize,
    ) -> isize;

    fn CloseHandle(hObject: isize) -> i32;

    fn GetLastError() -> u32;
}

// ============================================================================
// Windows constants
// ============================================================================

const PIPE_ACCESS_DUPLEX: u32 = 0x00000003;
const PIPE_TYPE_MESSAGE: u32 = 0x00000004;
const PIPE_READMODE_MESSAGE: u32 = 0x00000002;
const PIPE_WAIT: u32 = 0x00000000;
const PIPE_UNLIMITED_INSTANCES: u32 = 255;
const GENERIC_WRITE: u32 = 0x40000000;
const OPEN_EXISTING: u32 = 3;
const INVALID_HANDLE_VALUE: isize = -1;
const ERROR_PIPE_CONNECTED: u32 = 535;

/// 命名管道名称
const PIPE_NAME: &str = r"\\.\pipe\JinnImageViewer_IPC";

// ============================================================================
// Server — 在第一个实例的后台线程中运行
// ============================================================================

/// 启动命名管道服务器后台线程。
/// 接收到路径后通过 `tx` 发送给主线程。
pub fn start_server(tx: mpsc::Sender<String>) {
    std::thread::Builder::new()
        .name("ipc-server".into())
        .spawn(move || server_loop(tx))
        .expect("Failed to spawn IPC server thread");
}

fn server_loop(tx: mpsc::Sender<String>) {
    loop {
        let pipe_name_wide: Vec<u16> = OsStr::new(PIPE_NAME).encode_wide().chain(std::iter::once(0)).collect();

        // SAFETY: pipe_name_wide is NUL-terminated; null security descriptor uses
        // the process default security attributes.
        let h_pipe = unsafe {
            CreateNamedPipeW(
                pipe_name_wide.as_ptr(),
                PIPE_ACCESS_DUPLEX,
                PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                PIPE_UNLIMITED_INSTANCES,
                4096,             // output buffer size
                4096,             // input buffer size
                0,                // default timeout
                std::ptr::null(), // default security attributes
            )
        };

        if h_pipe == INVALID_HANDLE_VALUE {
            // Pipe creation failed; sleep and retry
            std::thread::sleep(std::time::Duration::from_millis(1000));
            continue;
        }

        // Wait for a client connection
        // SAFETY: h_pipe is a valid handle from CreateNamedPipeW
        let connected = unsafe { ConnectNamedPipe(h_pipe, std::ptr::null_mut()) } != 0;
        if !connected {
            // GetLastError may return ERROR_PIPE_CONNECTED if the client connected
            // between CreateNamedPipeW and ConnectNamedPipeW — this is a success case.
            // SAFETY: GetLastError has no preconditions
            let err = unsafe { GetLastError() };
            if err != ERROR_PIPE_CONNECTED {
                // Unexpected error; close and retry
                unsafe { CloseHandle(h_pipe) };
                continue;
            }
        }

        // Read the path string from the client
        // SAFETY: buf is a valid writable buffer; read receives actual byte count.
        let mut buf = [0u8; 4096];
        let mut read: u32 = 0;
        let ok = unsafe {
            ReadFile(
                h_pipe,
                buf.as_mut_ptr() as *mut _,
                buf.len() as u32,
                &mut read,
                std::ptr::null_mut(),
            )
        } != 0;

        if ok && read > 0 {
            let path = String::from_utf8_lossy(&buf[..read as usize]);
            let path = path.trim_end_matches('\0').trim().to_string();
            if !path.is_empty() {
                let _ = tx.send(path);
            }
        }

        // SAFETY: h_pipe is a valid handle; disconnect before close
        unsafe {
            DisconnectNamedPipe(h_pipe);
            CloseHandle(h_pipe);
        }
    }
}

// ============================================================================
// Client — 在第二个实例中调用，发送路径后退出
// ============================================================================

/// 向正在运行的主实例发送文件路径。
/// 返回 `true` 表示发送成功，`false` 表示失败（主实例未运行或管道错误）。
pub fn send_path(path: &str) -> bool {
    let pipe_name_wide: Vec<u16> = OsStr::new(PIPE_NAME).encode_wide().chain(std::iter::once(0)).collect();

    // SAFETY: pipe_name_wide is NUL-terminated; null security descriptor.
    let h_pipe = unsafe {
        CreateFileW(
            pipe_name_wide.as_ptr(),
            GENERIC_WRITE,
            0,                // dwShareMode
            std::ptr::null(), // lpSecurityAttributes
            OPEN_EXISTING,
            0, // dwFlagsAndAttributes
            0, // hTemplateFile
        )
    };

    if h_pipe == -1 {
        return false; // No server listening
    }

    let data = path.as_bytes();
    let mut written: u32 = 0;
    // SAFETY: h_pipe is a valid handle; data points to the path bytes.
    let success = unsafe {
        WriteFile(
            h_pipe,
            data.as_ptr() as *const _,
            data.len() as u32,
            &mut written,
            std::ptr::null_mut(),
        )
    } != 0;

    // SAFETY: CloseHandle on a valid handle.
    unsafe { CloseHandle(h_pipe) };

    success
}
