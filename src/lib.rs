// 公开模块供测试和外部使用
pub mod app;
pub mod config;
pub mod i18n;
pub mod image;
pub mod shortcuts;
pub mod sorting;
pub mod theme;
pub mod ui;

// 构建日期（在 build.rs 中生成，测试时提供默认值）
#[cfg(not(test))]
include!(concat!(env!("OUT_DIR"), "/build_date.rs"));

#[cfg(test)]
pub const BUILD_DATE: &str = "test-build";

// Windows dark titlebar FFI
#[cfg(target_os = "windows")]
#[link(name = "dwmapi")]
extern "system" {
    pub fn DwmSetWindowAttribute(hwnd: isize, dw_attribute: u32, pv_attribute: *const u32, cb_attribute: u32) -> i32;
}

#[cfg(not(target_os = "windows"))]
pub unsafe fn DwmSetWindowAttribute(_: isize, _: u32, _: *const u32, _: u32) -> i32 {
    0
}
