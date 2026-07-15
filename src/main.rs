// ============================================================================ 
// Jinn图片查看器 - Rust + egui Image Viewer 
// ============================================================================ 
// 隐藏Windows控制台窗口
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui;
use egui::{ColorImage, TextureHandle, Vec2};
use image::GenericImageView;
use rfd::FileDialog;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use open::that; // 添加open库用于打开GitHub链接

// ============================================================================ 
// Windows dark titlebar FFI - module level extern block 
// ============================================================================ 
#[cfg(target_os = "windows")]
#[link(name = "dwmapi")]
extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: isize,
        dw_attribute: u32,
        pv_attribute: *const u32,
        cb_attribute: u32,
    ) -> i32;
}

// ============================================================================ 
// Natural sort by splitting on digit boundaries 
// ============================================================================ 
/// 将字符串分割为数字和文本片段，用于自然排序
fn natural_sort_key(s: &str) -> Vec<(Option<u64>, String)> {
    let mut fragments: Vec<(Option<u64>, String)> = Vec::new();
    let mut current_digits = String::new();
    let mut current_text = String::new();
    let mut in_digits = false;

    for c in s.chars() {
        let is_digit = c.is_ascii_digit();
        if current_digits.is_empty() && current_text.is_empty() {
            in_digits = is_digit;
            if is_digit {
                current_digits.push(c);
            } else {
                current_text.push(c);
            }
        } else if in_digits == is_digit {
            if is_digit {
                current_digits.push(c);
            } else {
                current_text.push(c);
            }
        } else {
            if in_digits {
                let n = current_digits.parse::<u64>().unwrap_or(0);
                fragments.push((Some(n), String::new()));
            } else {
                fragments.push((None, current_text.to_lowercase()));
            }
            current_digits.clear();
            current_text.clear();
            in_digits = is_digit;
            if is_digit {
                current_digits.push(c);
            } else {
                current_text.push(c);
            }
        }
    }

    if !current_digits.is_empty() {
        let n = current_digits.parse::<u64>().unwrap_or(0);
        fragments.push((Some(n), String::new()));
    } else if !current_text.is_empty() {
        fragments.push((None, current_text.to_lowercase()));
    }

    fragments
}

/// 自然排序比较函数
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let ka = natural_sort_key(a);
    let kb = natural_sort_key(b);
    let max_len = ka.len().max(kb.len());
    for i in 0..max_len {
        let ea = ka.get(i);
        let eb = kb.get(i);
        match (ea, eb) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(a), Some(b)) => {
                let cmp = match (a.0, b.0) {
                    (Some(na), Some(nb)) => na.cmp(&nb),
                    (Some(_), None) => Ordering::Greater,
                    (None, Some(_)) => Ordering::Less,
                    (None, None) => a.1.cmp(&b.1),
                };
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
        }
    }
    Ordering::Equal
}

// ============================================================================ 
// Load icon from embedded PNG bytes 
// ============================================================================ 
/// 从嵌入的PNG字节数据加载图标
fn load_icon_from_bytes(bytes: &[u8]) -> egui::IconData {
    let img = image::load_from_memory(bytes).expect("Failed to decode icon PNG");
    let rgba = img.to_rgba8();
    egui::IconData {
        rgba: rgba.into_raw(),
        width: img.width(),
        height: img.height(),
    }
}

// ============================================================================ 
// Key label helper 
// ============================================================================ 
/// 获取按键的显示名称
fn key_label(key: egui::Key) -> &'static str {
    use egui::Key::*;
    match key {
        A => "A", B => "B", C => "C", D => "D", E => "E", F => "F", G => "G", H => "H", I => "I", J => "J", K => "K", L => "L", M => "M", N => "N", O => "O", P => "P", Q => "Q", R => "R", S => "S", T => "T", U => "U", V => "V", W => "W", X => "X", Y => "Y", Z => "Z",
        Num0 => "0", Num1 => "1", Num2 => "2", Num3 => "3", Num4 => "4", Num5 => "5", Num6 => "6", Num7 => "7", Num8 => "8", Num9 => "9",
        F1 => "F1", F2 => "F2", F3 => "F3", F4 => "F4", F5 => "F5", F6 => "F6", F7 => "F7", F8 => "F8", F9 => "F9", F10 => "F10", F11 => "F11", F12 => "F12",
        ArrowLeft => "\u{2190}", ArrowRight => "\u{2192}", ArrowUp => "\u{2191}", ArrowDown => "\u{2193}",
        Escape => "Esc", Delete => "Del", Space => "Space", Enter => "Enter", Tab => "Tab", Backspace => "Bksp", Home => "Home", End => "End", PageUp => "PgUp", PageDown => "PgDn", Insert => "Ins",
        Minus => "-", Equals => "=", Comma => ",", Period => ".", Semicolon => ";", Quote => "'", Backslash => "\\", Slash => "/",
        _ => "?",
    }
}

// ============================================================================ 
// Shortcut model 
// ============================================================================ 
/// 快捷键动作枚举
#[derive(Clone, PartialEq)]
enum ShortcutAction {
    OpenFolder,
    PrevImage,
    NextImage,
    DeleteImage,
    ToggleDual,
    CopyLeft,
    CopyRight,
    ToggleFit,
    About, // 新增关于动作
}

impl ShortcutAction {
    /// 获取动作的显示标签
    fn label(&self) -> &'static str {
        match self {
            Self::OpenFolder => "\u{6253}\u{5F00}\u{6587}\u{4EF6}\u{5939}",
            Self::PrevImage => "\u{4E0A}\u{4E00}\u{5F20}",
            Self::NextImage => "\u{4E0B}\u{4E00}\u{5F20}",
            Self::DeleteImage => "\u{5220}\u{9664}\u{56FE}\u{7247}",
            Self::ToggleDual => "\u{53CC}\u{5217}\u{5C55}\u{793A}",
            Self::CopyLeft => "\u{590D}\u{5236}\u{56FE}\u{7247}(\u{5DE6}/\u{5F53}\u{524D})",
            Self::CopyRight => "\u{590D}\u{5236}\u{56FE}\u{7247}(\u{53F3})",
            Self::ToggleFit => "\u{9002}\u{5E94}\u{7A97}\u{53E3} (\u{9F20}\u{6807}\u{53F3}\u{952E})",
            Self::About => "\u{5173}\u{4E8E}", // 新增关于标签
        }
    }
}

/// 快捷键条目结构体
#[derive(Clone)]
struct ShortcutEntry {
    action: ShortcutAction,
    key: egui::Key,
    modifiers: egui::Modifiers,
}

impl ShortcutEntry {
    /// 创建新的快捷键条目
    fn new(action: ShortcutAction, key: egui::Key, modifiers: egui::Modifiers) -> Self {
        Self { action, key, modifiers }
    }
}

// ============================================================================ 
// Shortcut configuration 
// ============================================================================ 
/// 快捷键配置结构体
struct ShortcutConfig {
    entries: Vec<ShortcutEntry>,
}

impl ShortcutConfig {
    /// 获取默认快捷键配置
    fn defaults() -> Self {
        use egui::Key::*;
        Self {
            entries: vec![
                ShortcutEntry::new(ShortcutAction::OpenFolder, O, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::PrevImage, ArrowLeft, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::PrevImage, ArrowUp, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::NextImage, ArrowRight, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::NextImage, ArrowDown, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::DeleteImage, Delete, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::ToggleDual, F2, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::CopyLeft, Num1, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::CopyRight, Num2, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::ToggleFit, V, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::About, F1, egui::Modifiers::NONE), // 新增关于快捷键
            ],
        }
    }
}

// ============================================================================ 
// Image info 
// ============================================================================ 
/// 图片条目结构体
#[derive(Clone)]
struct ImageEntry {
    path: PathBuf,
    name: String,
}

// ============================================================================ 
// Main application 
// ============================================================================ 
/// 主应用结构体
struct JinnImageViewer {
    images: Vec<ImageEntry>,
    current_index: usize,
    scale_factor: f32,
    fit_to_window: bool,
    two_column: bool,
    show_shortcuts_window: bool,
    show_about_window: bool, // 新增关于窗口控制
    shortcuts: ShortcutConfig,
    textures: HashMap<PathBuf, TextureHandle>,
    folder_path: Option<PathBuf>,
    initialized: bool,
    status_message: String,
    zoom_step_percent: f32,
    show_copy_error_dialog: bool,
    copy_error_msg: String,
}

impl JinnImageViewer {
    /// 创建新的应用实例
    fn new() -> Self {
        Self {
            images: Vec::new(),
            current_index: 0,
            scale_factor: 1.0,
            fit_to_window: false,
            two_column: false,
            show_shortcuts_window: false,
            show_about_window: false, // 初始化为false
            shortcuts: ShortcutConfig::defaults(),
            textures: HashMap::new(),
            folder_path: None,
            initialized: false,
            status_message: String::new(),
            zoom_step_percent: 10.0,
            show_copy_error_dialog: false,
            copy_error_msg: String::new(),
        }
    }

    // ---- Dark titlebar (Windows) -------------------------------------------
    /// 设置Windows深色标题栏
    #[cfg(target_os = "windows")]
    fn set_dark_titlebar(_ctx: &egui::Context, frame: &mut eframe::Frame) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = frame.window_handle() {
            if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                let hwnd = win32_handle.hwnd.get() as isize;
                const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
                let dark: u32 = 1;
                unsafe {
                    DwmSetWindowAttribute(
                        hwnd,
                        DWMWA_USE_IMMERSIVE_DARK_MODE,
                        &dark as *const u32,
                        std::mem::size_of::<u32>() as u32,
                    );
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn set_dark_titlebar(_ctx: &egui::Context, _frame: &mut eframe::Frame) {}

    // ---- Texture cleanup (Performance Optimization) -------------------------
    /// 清理纹理缓存，只保留当前及前后各1张图片的纹理
    fn cleanup_textures(&mut self) {
        if self.images.is_empty() {
            self.textures.clear();
            return;
        }
        let current = self.current_index;
        let start = current.saturating_sub(1);
        let end = std::cmp::min(current + 2, self.images.len());
        
        let mut to_keep = std::collections::HashSet::new();
        for i in start..end {
            to_keep.insert(self.images[i].path.clone());
        }

        self.textures.retain(|path, _| to_keep.contains(path));
    }

    // ---- Folder ops ---------------------------------------------------------
    /// 打开文件夹
    fn open_folder(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            self.load_folder(folder);
        }
    }

    /// 加载文件夹中的图片
    fn load_folder(&mut self, folder: PathBuf) {
        self.textures.clear();
        self.images.clear();
        self.current_index = 0;
        self.scale_factor = 1.0;
        self.status_message.clear();

        let supported_ext = |ext: &std::ffi::OsStr| -> bool {
            matches!(ext.to_str().unwrap_or("").to_lowercase().as_str(), "png" | "jpg" | "jpeg")
        };

        let mut entries: Vec<ImageEntry> = Vec::new();
        if let Ok(dir) = std::fs::read_dir(&folder) {
            for entry in dir.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if supported_ext(ext) {
                            let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                            entries.push(ImageEntry { path, name });
                        }
                    }
                }
            }
        }

        entries.sort_by(|a, b| natural_cmp(&a.name, &b.name));
        self.images = entries;
        self.folder_path = Some(folder);

        if self.images.is_empty() {
            self.status_message = "\u{6587}\u{4EF6}\u{5939}\u{4E2D}\u{6CA1}\u{6709}\u{627E}\u{5230}\u{56FE}\u{7247}\u{6587}\u{4EF6} (.png/.jpg/.jpeg)".to_string();
        } else {
            self.status_message.clear();
        }
    }

    // ---- Navigation ---------------------------------------------------------
    /// 导航到指定图片
    fn navigate(&mut self, step: i32) {
        if self.images.is_empty() { return; }
        let len = self.images.len() as i32;
        let mut new_idx = self.current_index as i32 + step;

        if new_idx < 0 {
            if self.two_column {
                new_idx = ((len - 1) / 2) * 2;
            } else {
                new_idx = len - 1;
            }
        } else if new_idx >= len {
            new_idx = 0;
        }

        self.current_index = new_idx as usize;
        self.scale_factor = 1.0;
        self.status_message.clear();
        self.cleanup_textures();
    }

    // ---- Delete -------------------------------------------------------------
    /// 删除当前图片
    fn delete_current(&mut self) {
        if self.images.is_empty() || self.current_index >= self.images.len() { return; }
        let path = self.images[self.current_index].path.clone();
        match std::fs::remove_file(&path) {
            Ok(_) => {
                self.textures.remove(&path);
                self.images.remove(self.current_index);
                if self.images.is_empty() {
                    self.current_index = 0;
                    self.status_message = "\u{6240}\u{6709}\u{56FE}\u{7247}\u{5DF2}\u{5220}\u{9664}".to_string();
                } else {
                    if self.current_index >= self.images.len() {
                        self.current_index = self.images.len() - 1;
                    }
                    self.status_message = format!("\u{5DF2}\u{5220}\u{9664}: {}", path.file_name().unwrap_or_default().to_string_lossy());
                }
                self.cleanup_textures();
            }
            Err(e) => {
                self.status_message = format!("\u{5220}\u{9664}\u{5931}\u{8D25}: {}", e);
            }
        }
    }

    // ---- Copy ---------------------------------------------------------------
    /// 复制图片
    fn copy_image(&mut self, offset: usize) {
        if self.images.is_empty() { return; }
        let idx = self.current_index + offset;
        if idx >= self.images.len() {
            self.status_message = "\u{5F53}\u{524D}\u{6CA1}\u{6709}\u{53F3}\u{56FE}\u{53EF}\u{590D}\u{5236}".to_string();
            return;
        }

        let src = self.images[idx].path.clone();
        let name = self.images[idx].name.clone();

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        let copy_dir = exe_dir.join("copy");
        let _ = std::fs::create_dir_all(&copy_dir);

        let dest = copy_dir.join(&name);
        
        if dest.exists() {
            self.copy_error_msg = "\u{56FE}\u{7247}\u{5DF2}\u{5B58}\u{5728}\u{FF0C}\u{5DF2}\u{7981}\u{6B62}\u{590D}\u{5236}\u{3002}".to_string();
            self.show_copy_error_dialog = true;
            return;
        }

        match std::fs::copy(&src, &dest) {
            Ok(_) => self.status_message = format!("\u{590D}\u{5236}\u{540E}\u{5B58}\u{653E}\u{8DEF}\u{5F84}: {}", dest.display()),
            Err(e) => self.status_message = format!("\u{590D}\u{5236}\u{5931}\u{8D25}: {}", e),
        }
    }

    // ---- Texture loading ----------------------------------------------------
    /// 获取或加载纹理
    fn get_or_load_texture(&mut self, ctx: &egui::Context, path: &Path) -> Option<&TextureHandle> {
        if self.textures.contains_key(path) {
            return self.textures.get(path);
        }
        let img = image::ImageReader::open(path).ok()?.decode().ok()?;
        let rgba = img.to_rgba8();
        let (w, h) = img.dimensions();
        let color_image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let name = path.to_string_lossy().to_string();
        let texture = ctx.load_texture(name, color_image, Default::default());
        self.textures.insert(path.to_path_buf(), texture);
        self.textures.get(path)
    }

    // ---- Mouse wheel zoom ---------------------------------------------------
    /// 处理鼠标滚轮缩放
    fn handle_zoom(&mut self, ctx: &egui::Context) {
        let scroll_y = ctx.input(|i| {
            let mut y = 0.0;
            for event in &i.raw.events {
                if let egui::Event::MouseWheel { unit, delta, .. } = event {
                    match unit {
                        egui::MouseWheelUnit::Line => y += delta.y,
                        egui::MouseWheelUnit::Point => y += delta.y / 100.0,
                        egui::MouseWheelUnit::Page => y += if delta.y > 0.0 { 1.0 } else { -1.0 },
                    }
                }
            }
            y
        });

        if scroll_y.abs() < 0.1 { return; }

        ctx.input_mut(|i| {
            i.smooth_scroll_delta = egui::Vec2::ZERO;
        });

        if self.fit_to_window {
            self.fit_to_window = false;
        }

        let step = (self.zoom_step_percent / 100.0).max(0.01);
        let factor = if scroll_y > 0.0 { 1.0 + step } else { 1.0 / (1.0 + step) };
        self.scale_factor = (self.scale_factor * factor).clamp(0.1, 20.0);
    }

    // ---- Input handling -----------------------------------------------------
    /// 处理输入事件
    fn handle_input(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.wants_keyboard_input() { return; }

        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        let mut triggered_action = None;
        for entry in &self.shortcuts.entries {
            if ctx.input_mut(|i| i.consume_key(entry.modifiers, entry.key)) {
                triggered_action = Some(entry.action.clone());
                break;
            }
        }

        if let Some(action) = triggered_action {
            let step = if self.two_column { 2i32 } else { 1i32 };
            match action {
                ShortcutAction::OpenFolder => self.open_folder(),
                ShortcutAction::PrevImage => self.navigate(-step),
                ShortcutAction::NextImage => self.navigate(step),
                ShortcutAction::DeleteImage => self.delete_current(),
                ShortcutAction::ToggleDual => self.two_column = !self.two_column,
                ShortcutAction::CopyLeft => self.copy_image(0),
                ShortcutAction::CopyRight => {
                    if self.two_column { self.copy_image(1); }
                }
                ShortcutAction::ToggleFit => self.fit_to_window = !self.fit_to_window,
                ShortcutAction::About => self.show_about_window = true, // 新增关于动作处理
            }
        }
    }

    // ---- Menu bar -----------------------------------------------------------
    /// 显示菜单栏
    fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("\u{6587}\u{4EF6}", |ui| {
                if ui.button("\u{6253}\u{5F00}\u{6587}\u{4EF6}\u{5939}").clicked() {
                    ui.close_menu();
                    self.open_folder();
                }
                if ui.button("\u{9000}\u{51FA}").clicked() {
                    ui.close_menu();
                    std::process::exit(0);
                }
            });

            ui.menu_button("\u{89C6}\u{56FE}", |ui| {
                if ui.button("\u{4E0A}\u{4E00}\u{5F20}").clicked() {
                    ui.close_menu();
                    let step = if self.two_column { 2 } else { 1 };
                    self.navigate(-step);
                }
                if ui.button("\u{4E0B}\u{4E00}\u{5F20}").clicked() {
                    ui.close_menu();
                    let step = if self.two_column { 2 } else { 1 };
                    self.navigate(step);
                }
                ui.separator();
                if ui.checkbox(&mut self.fit_to_window, "\u{9002}\u{5E94}\u{7A97}\u{53E3}").clicked() {
                    ui.close_menu();
                }
                if ui.checkbox(&mut self.two_column, "\u{53CC}\u{5217}\u{5C55}\u{793A}").clicked() {
                    ui.close_menu();
                }
            });

            ui.menu_button("\u{8bbe}\u{7f6e}", |ui| {
                if ui.button("\u{81ea}\u{5b9a}\u{4e49}\u{5feb}\u{6377}\u{952e}").clicked() {
                    ui.close_menu();
                    self.show_shortcuts_window = true;
                }
                ui.separator();
                if ui.button("\u{590d}\u{5236}\u{56fe}\u{7247}(\u{5de6}/\u{5f53}\u{524d})").clicked() {
                    ui.close_menu();
                    self.copy_image(0);
                }
                if ui.button("\u{590d}\u{5236}\u{56fe}\u{7247}(\u{53f3})").clicked() {
                    ui.close_menu();
                    if self.two_column { self.copy_image(1); }
                }
                ui.separator();
                // 新增关于按钮
                if ui.button("\u{5173}\u{4e8e}").clicked() {
                    ui.close_menu();
                    self.show_about_window = true;
                }
            });
        });
    }

    // ---- Status bar ---------------------------------------------------------
    /// 显示状态栏
    fn show_status(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if !self.status_message.is_empty() {
                ui.label(&self.status_message);
                return;
            }
            if self.images.is_empty() {
                ui.label("\u{672a}\u{52a0}\u{8f7d}\u{56fe}\u{7247} \u{2014} \u{53cc}\u{51fb}\u{6216}\u{6309} O \u{6253}\u{5f00}\u{6587}\u{4ef6}\u{5939}");
                return;
            }
            let idx = self.current_index;
            if idx < self.images.len() {
                ui.label(format!("{}", self.images[idx].name));
                ui.separator();
                ui.label(format!("{}/{}", idx + 1, self.images.len()));
                ui.separator();
                ui.label(format!("\u{7f29}\u{653e}: {:.0}%", self.scale_factor * 100.0));
                if self.two_column && idx + 1 < self.images.len() {
                    ui.separator();
                    ui.label(format!("\u{53f3}: {}", self.images[idx + 1].name));
                }
            }
        });
    }

    // ---- Shortcut config window content -------------------------------------
    /// 显示快捷键配置窗口内容
    fn show_shortcuts_window_content(&mut self, ui: &mut egui::Ui, request_close: &mut bool) {
        ui.heading("\u{81ea}\u{5b9a}\u{4e49}\u{5feb}\u{6377}\u{952e}");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("\u{9f20}\u{6807}\u{6eda}\u{8f6e}\u{7f29}\u{653e}\u{6b65}\u{957f} (%):");
            ui.add(egui::DragValue::new(&mut self.zoom_step_percent).range(1.0..=100.0).speed(0.1));
        });
        ui.separator();

        use egui::Key::*;
        let key_options: &[(&str, egui::Key)] = &[
            ("A", A), ("B", B), ("C", C), ("D", D), ("E", E), ("F", F), ("G", G), ("H", H), ("I", I), ("J", J), ("K", K), ("L", L), ("M", M), ("N", N), ("O", O), ("P", P), ("Q", Q), ("R", R), ("S", S), ("T", T), ("U", U), ("V", V), ("W", W), ("X", X), ("Y", Y), ("Z", Z),
            ("0", Num0), ("1", Num1), ("2", Num2), ("3", Num3), ("4", Num4), ("5", Num5), ("6", Num6), ("7", Num7), ("8", Num8), ("9", Num9),
            ("F1", F1), ("F2", F2), ("F3", F3), ("F4", F4), ("F5", F5), ("F6", F6), ("F7", F7), ("F8", F8), ("F9", F9), ("F10", F10), ("F11", F11), ("F12", F12),
            ("\u{2190}", ArrowLeft), ("\u{2192}", ArrowRight), ("\u{2191}", ArrowUp), ("\u{2193}", ArrowDown),
            ("Esc", Escape), ("Del", Delete), ("Space", Space), ("Enter", Enter), ("Tab", Tab),
        ];

        egui::ScrollArea::vertical().show(ui, |ui| {
            let total = self.shortcuts.entries.len();
            for i in 0..total {
                let action_label = self.shortcuts.entries[i].action.label();
                ui.horizontal(|ui| {
                    ui.label(action_label);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let current_key = self.shortcuts.entries[i].key;
                        let current_key_label = key_label(current_key);

                        egui::ComboBox::from_id_salt(format!("sc_key_{}", i))
                            .selected_text(current_key_label)
                            .width(80.0)
                            .show_ui(ui, |ui| {
                                for &(label, ek) in key_options {
                                    let is_selected = ek == current_key;
                                    if ui.selectable_label(is_selected, label).clicked() {
                                        self.shortcuts.entries[i].key = ek;
                                    }
                                }
                            });

                        let mut alt = self.shortcuts.entries[i].modifiers.alt;
                        let mut shift = self.shortcuts.entries[i].modifiers.shift;
                        let mut ctrl = {
                            #[cfg(target_os = "windows")] { self.shortcuts.entries[i].modifiers.command }
                            #[cfg(not(target_os = "windows"))] { self.shortcuts.entries[i].modifiers.command || self.shortcuts.entries[i].modifiers.mac_cmd }
                        };

                        ui.push_id(format!("alt_{}", i), |ui| { ui.checkbox(&mut alt, "Alt"); });
                        ui.push_id(format!("shift_{}", i), |ui| { ui.checkbox(&mut shift, "Shift"); });
                        ui.push_id(format!("ctrl_{}", i), |ui| { ui.checkbox(&mut ctrl, "Ctrl"); });

                        let mut mods = egui::Modifiers::NONE;
                        if alt { mods.alt = true; }
                        if shift { mods.shift = true; }
                        if ctrl { mods.command = true; }
                        self.shortcuts.entries[i].modifiers = mods;
                    });
                });
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("\u{4fdd}\u{5b58}\u{8bbe}\u{7f6e}").clicked() {
                    self.status_message = "\u{5feb}\u{6377}\u{952e}\u{8bbe}\u{7f6e}\u{5df2}\u{4fdd}\u{5b58}".to_string();
                }
                if ui.button("\u{6062}\u{590d}\u{9ed8}\u{8ba4}").clicked() {
                    self.shortcuts = ShortcutConfig::defaults();
                    self.zoom_step_percent = 10.0;
                    self.status_message = "\u{5df2}\u{6062}\u{590d}\u{9ed8}\u{8ba4}\u{8bbe}\u{7f6e}".to_string();
                }
            });
            ui.add_space(4.0);
            ui.label(egui::RichText::new("\u{6e29}\u{9986}\u{63d0}\u{793a}: \u{9f20}\u{6807}\u{53f3}\u{952e}\u{5728}\u{56fe}\u{7247}\u{533a}\u{57df}\u{70b9}\u{51fb}\u{53ef}\u{5feb}\u{901f}\u{9002}\u{5e94}\u{7a97}\u{53e3}").color(egui::Color32::from_gray(120)).small());
            ui.vertical_centered(|ui| {
                if ui.button("\u{5173}\u{95ed}\u{7a97}\u{53e3}").clicked() {
                    *request_close = true;
                }
            });
        });
    }

    // ---- About window content -------------------------------------
    /// 显示关于窗口
    fn show_about_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("\u{5173}\u{4e8e}")
            .open(&mut self.show_about_window)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(format!("Jinn\u{56fe}\u{7247}\u{67e5}\u{770b}\u{5668} v{}", "20260715"));
                    ui.label("\u{4f5c}\u{8005}: \u{53f6}\u{5b50}Jinn");
                    // 修复：使用format!正确拼接字符串
                    if ui.button(format!("{}GitHub{}", "\u{8bbf}\u{95ee}", "\u{9879}\u{76ee}")).clicked() {
                        if let Err(e) = that("https://github.com/yezjinn/jinn_imageviewer") {
                            // 修复：正确使用format!宏
                            self.status_message = format!("{}GitHub{}: {}", "\u{6253}\u{5f00}", "\u{5931}\u{8d25}", e);
                        }
                    }
                });
            });
    }


    // ---- Central image area -------------------------------------------------
    /// 显示中央图片区域
    fn show_image_area(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if self.images.is_empty() {
            let rect = ui.available_rect_before_wrap();
            let painter = ui.painter();
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "\u{6253}\u{5f00}\u{6587}\u{4ef6}\u{5939}\u{4ee5}\u{67e5}\u{770b}\u{56fe}\u{7247}\n\n\u{53cc}\u{51fb}\u{6b64}\u{5904} \u{6216} \u{6309} O \u{952e}",
                egui::FontId::proportional(18.0),
                egui::Color32::from_gray(160),
            );
            let response = ui.interact(rect, ui.next_auto_id(), egui::Sense::click());
            if response.double_clicked() {
                self.open_folder();
            }
            return;
        }

        self.handle_zoom(ctx);

        let idx = self.current_index;
        if idx >= self.images.len() { return; }

        if self.two_column {
            self.show_dual_images(ui, ctx);
        } else {
            self.show_single_image(ui, ctx, idx);
        }

        let response = ui.interact(ui.max_rect(), ui.next_auto_id(), egui::Sense::click());
        if response.double_clicked() {
            self.open_folder();
        }
        if response.secondary_clicked() {
            self.fit_to_window = true;
        }

        self.cleanup_textures();
    }

    /// 显示单张图片
    fn show_single_image(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, idx: usize) {
        let path = self.images[idx].path.clone();
        let fit_to_window = self.fit_to_window;
        let scale_factor = self.scale_factor;
        
        let texture = match self.get_or_load_texture(ctx, &path) {
            Some(t) => t,
            None => {
                ui.label("\u{52a0}\u{8f7d}\u{56fe}\u{7247}\u{5931}\u{8d25}");
                return;
            }
        };

        let tex_size = texture.size_vec2();
        let avail = ui.available_size();
        let draw_size = if fit_to_window {
            fit_size(tex_size, avail)
        } else {
            tex_size * scale_factor
        };

        if fit_to_window {
            ui.allocate_ui_with_layout(avail, egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                ui.image(egui::load::SizedTexture::new(texture.id(), draw_size));
            });
        } else {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.allocate_ui_with_layout(ui.available_size(), egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                        ui.image(egui::load::SizedTexture::new(texture.id(), draw_size));
                    });
                });
        }
    }

    /// 显示双张图片
    fn show_dual_images(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let left_idx = self.current_index;
        let right_idx = self.current_index + 1;
        let fit_to_window = self.fit_to_window;
        let scale_factor = self.scale_factor;

        ui.horizontal_top(|ui| {
            let total_w = ui.available_width();
            let half_w = (total_w / 2.0).max(1.0);
            let avail_h = ui.available_height();
            let half_size = Vec2::new(half_w, avail_h);

            if left_idx < self.images.len() {
                let path = self.images[left_idx].path.clone();
                if let Some(texture) = self.get_or_load_texture(ctx, &path) {
                    let tex_size = texture.size_vec2();
                    let draw_size = if fit_to_window {
                        fit_size_to_max(tex_size, half_size)
                    } else {
                        tex_size * scale_factor
                    };

                    egui::ScrollArea::both()
                        .max_width(half_w)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.allocate_ui_with_layout(ui.available_size(), egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                                ui.image(egui::load::SizedTexture::new(texture.id(), draw_size));
                            });
                        });
                } else {
                    ui.allocate_space(half_size);
                }
            } else {
                ui.allocate_space(half_size);
            }

            if right_idx < self.images.len() {
                let path = self.images[right_idx].path.clone();
                if let Some(texture) = self.get_or_load_texture(ctx, &path) {
                    let tex_size = texture.size_vec2();
                    let draw_size = if fit_to_window {
                        fit_size_to_max(tex_size, half_size)
                    } else {
                        tex_size * scale_factor
                    };

                    egui::ScrollArea::both()
                        .max_width(half_w)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.allocate_ui_with_layout(ui.available_size(), egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                                ui.image(egui::load::SizedTexture::new(texture.id(), draw_size));
                            });
                        });
                } else {
                    ui.allocate_space(half_size);
                }
            } else {
                ui.allocate_space(half_size);
            }
        });
    }
}

// ============================================================================ 
// Helper: scale texture to fit within viewport 
// ============================================================================ 
/// 调整纹理大小以适应视口
fn fit_size(tex_size: Vec2, viewport: Vec2) -> Vec2 {
    if tex_size.x <= 0.0 || tex_size.y <= 0.0 || viewport.x <= 0.0 || viewport.y <= 0.0 {
        return tex_size;
    }
    let scale = (viewport.x / tex_size.x).min(viewport.y / tex_size.y);
    tex_size * scale.min(1.0)
}

/// 调整纹理大小以适应最大尺寸
fn fit_size_to_max(tex_size: Vec2, max_size: Vec2) -> Vec2 {
    if tex_size.x <= 0.0 || tex_size.y <= 0.0 || max_size.x <= 0.0 || max_size.y <= 0.0 {
        return tex_size;
    }
    let scale = (max_size.x / tex_size.x).min(max_size.y / tex_size.y);
    tex_size * scale.min(1.0)
}

// ============================================================================ 
// eframe::App implementation 
// ============================================================================ 
impl eframe::App for JinnImageViewer {
    /// 更新应用状态
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.initialized {
            self.initialized = true;
            Self::set_dark_titlebar(ctx, frame);

            let mut style = (*ctx.style()).clone();
            style.visuals.dark_mode = true;
            style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(0x1E, 0x1E, 0x1E);
            style.visuals.panel_fill = egui::Color32::from_rgb(0x2D, 0x2D, 0x2D);
            style.visuals.window_fill = egui::Color32::from_rgb(0x2D, 0x2D, 0x2D);
            style.visuals.selection.stroke.color = egui::Color32::from_rgb(0x00, 0x7A, 0xCC);
            style.visuals.selection.bg_fill = egui::Color32::from_rgb(0x00, 0x7A, 0xCC);
            style.visuals.hyperlink_color = egui::Color32::from_rgb(0x00, 0x7A, 0xCC);
            style.visuals.faint_bg_color = egui::Color32::from_rgb(0x1E, 0x1E, 0x1E);
            style.visuals.extreme_bg_color = egui::Color32::from_rgb(0x2D, 0x2D, 0x2D);
            style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4);
            ctx.set_style(style);
        }

        self.handle_input(ctx, frame);

        egui::TopBottomPanel::top("menu_panel")
            .min_height(24.0)
            .show(ctx, |ui| {
                self.show_menu(ui);
            });

        egui::TopBottomPanel::bottom("status_panel")
            .min_height(22.0)
            .show(ctx, |ui| {
                self.show_status(ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_image_area(ui, ctx);
        });

        if self.show_copy_error_dialog {
            egui::Window::new("\u{63d0}\u{793a}")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(10.0);
                        ui.label(&self.copy_error_msg);
                        ui.add_space(10.0);
                        if ui.button("\u{786e}\u{5b9a}").clicked() {
                            self.show_copy_error_dialog = false;
                        }
                    });
                });
        }

        if self.show_shortcuts_window {
            let mut open = self.show_shortcuts_window;
            let mut request_close = false;
            egui::Window::new("\u{81ea}\u{5b9a}\u{4e49}\u{5feb}\u{6377}\u{952e}")
                .open(&mut open)
                .resizable(true)
                .default_width(400.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    self.show_shortcuts_window_content(ui, &mut request_close);
                });
            if request_close {
                open = false;
            }
            self.show_shortcuts_window = open;
        }

        // 处理关于窗口
        if self.show_about_window {
            self.show_about_window(ctx);
        }
    }
}

// ============================================================================ 
// Entry point 
// ============================================================================ 
fn main() {
    let icon = load_icon_from_bytes(include_bytes!("../icons/PNG/icon_256.png"));

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Jinn\u{56FE}\u{7247}\u{67E5}\u{770B}\u{5668}")
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "Jinn\u{56FE}\u{7247}\u{67E5}\u{770B}\u{5668}",
        native_options,
        Box::new(|cc| {
            setup_chinese_fonts(&cc);
            Ok(Box::new(JinnImageViewer::new()))
        }),
    )
    .expect("Failed to run Jinn\u{56FE}\u{7247}\u{67E5}\u{770B}\u{5668}");
}

// ============================================================================ 
// 加载中文字体 
// ============================================================================ 
/// 设置中文字体
fn setup_chinese_fonts(cc: &eframe::CreationContext<'_>) {
    let mut fonts = egui::FontDefinitions::default();

    let font_paths = [
        ("C:\\Windows\\Fonts\\simhei.ttf", "chinese_font_heiti"),
        ("C:\\Windows\\Fonts\\simsunb.ttf", "chinese_font_songti"),
        ("C:\\Windows\\Fonts\\simkai.ttf", "chinese_font_kaiti"),
        ("C:\\Windows\\Fonts\\simfang.ttf", "chinese_font_fangti"),
        ("C:\\Windows\\Fonts\\msyh.ttc", "chinese_font_yahei"),
    ];

    for (font_path, font_name) in &font_paths {
        if let Ok(font_data) = std::fs::read(font_path) {
            fonts.font_data.insert(
                (*font_name).into(),
                egui::FontData::from_owned(font_data).into(),
            );

            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, (*font_name).into());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.insert(0, (*font_name).into());
            }
        }
    }

    cc.egui_ctx.set_fonts(fonts);
}
