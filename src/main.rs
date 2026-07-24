// ============================================================================
// Jinn图片查看器 - Rust + egui Image Viewer
// ============================================================================
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui;
use jinn_imageviewer::*;

// ============================================================================
// Entry point
// ============================================================================
fn main() {
    // ============================================================================
    // 单实例检测（Windows 命名互斥体）+ IPC 路径转发
    // ============================================================================
    #[cfg(target_os = "windows")]
    let (_mutex, ipc_rx) = {
        use std::os::windows::ffi::OsStrExt;
        let name: Vec<u16> = std::ffi::OsStr::new("Global\\JinnImageViewer_SingleInstance")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: The UTF-16 name is NUL-terminated and points to a live buffer
        // for the duration of the Windows API call. The null security
        // descriptor uses the process default security attributes.
        let handle = unsafe {
            windows_sys::Win32::System::Threading::CreateMutexW(
                std::ptr::null(),
                1, // bInitialOwner = TRUE
                name.as_ptr(),
            )
        };
        if handle.is_null() {
            return;
        }
        // SAFETY: GetLastError has no preconditions and only reads the calling
        // thread's Windows error state immediately after CreateMutexW.
        let last_error = unsafe { windows_sys::Win32::Foundation::GetLastError() };
        if last_error == 183 {
            // ERROR_ALREADY_EXISTS — 已有实例运行，通过 IPC 转发命令行参数后退出
            let args: Vec<String> = std::env::args().skip(1).collect();
            if !args.is_empty() {
                ipc::send_path(&args[0]);
            }
            return;
        }
        // 第一实例：启动命名管道服务器，接收后续实例的路径
        let (tx, rx) = std::sync::mpsc::channel();
        ipc::start_server(tx);
        (handle, rx)
    };

    #[cfg(not(target_os = "windows"))]
    let (_mutex, ipc_rx) = ((), std::sync::mpsc::channel::<String>().1);

    // ============================================================================
    // 窗口初始化
    // ============================================================================
    let icon = image::load_icon_from_bytes(include_bytes!("../icons/PNG/icon_256.png"));

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Jinn Image Viewer")
            .with_icon(icon)
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Jinn Image Viewer",
        native_options,
        Box::new(|cc| {
            setup_chinese_fonts(cc);
            let mut viewer = app::JinnImageViewer::new(ipc_rx);
            viewer.load_config();
            viewer.load_from_args();
            Ok(Box::new(viewer))
        }),
    )
    .expect("Failed to run Jinn Image Viewer");
}

// ============================================================================
// 加载中文字体（仅加载第一个可用字体，减少启动IO）
// ============================================================================
fn setup_chinese_fonts(cc: &eframe::CreationContext<'_>) {
    let mut fonts = egui::FontDefinitions::default();

    // 按优先级尝试加载，找到一个即停止
    let font_candidates = [
        ("C:\\Windows\\Fonts\\msyh.ttc", "chinese_font"), // 微软雅黑（覆盖最全）
        ("C:\\Windows\\Fonts\\simhei.ttf", "chinese_font"), // 黑体
        ("C:\\Windows\\Fonts\\simsun.ttc", "chinese_font"), // 宋体
    ];

    for (font_path, font_name) in &font_candidates {
        if let Ok(font_data) = std::fs::read(font_path) {
            fonts
                .font_data
                .insert((*font_name).into(), egui::FontData::from_owned(font_data).into());
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, (*font_name).into());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.insert(0, (*font_name).into());
            }
            break; // 找到一个即可
        }
    }

    cc.egui_ctx.set_fonts(fonts);

    // 全局字号 +5
    let mut style = (*cc.egui_ctx.style()).clone();
    for font_id in style.text_styles.values_mut() {
        font_id.size += 5.0;
    }
    cc.egui_ctx.set_style(style);
}
