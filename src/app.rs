use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::{AppConfig, ShortcutSetting};
use crate::i18n::I18n;
use crate::i18n::Language;
use crate::image::{get_image_info, scan_folder, GifAnimator, ImageEntry, TextureManager, ThumbnailManager};
use crate::shortcuts::{key_from_label, key_label, ShortcutAction, ShortcutConfig};
use crate::sorting::ImageSortMode;
use crate::theme::Theme;

/// 获取系统桌面路径
fn dirs_fallback_desktop() -> Option<PathBuf> {
    // 优先使用 USERPROFILE\Desktop
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let desktop = PathBuf::from(profile).join("Desktop");
        if desktop.is_dir() {
            return Some(desktop);
        }
    }
    // 备用：PUBLIC\Desktop
    if let Ok(public) = std::env::var("PUBLIC") {
        let desktop = PathBuf::from(public).join("Desktop");
        if desktop.is_dir() {
            return Some(desktop);
        }
    }
    None
}

/// 主应用结构体
pub struct JinnImageViewer {
    pub images: Vec<ImageEntry>,
    pub current_index: usize,
    pub scale_factor: f32,
    pub fit_to_window: bool,
    pub two_column: bool,
    pub show_shortcuts_window: bool,
    pub show_about_window: bool,
    pub shortcuts: ShortcutConfig,
    pub texture_manager: TextureManager,
    pub thumbnail_manager: ThumbnailManager,
    pub folder_path: Option<PathBuf>,
    pub initialized: bool,
    pub window_title: String,
    pub status_message: String,
    pub zoom_step_percent: f32,
    pub show_copy_error_dialog: bool,
    pub copy_error_msg: String,
    pub show_large_image_dialog: bool,
    pub large_image_path: Option<PathBuf>,
    pub approved_large_image_path: Option<PathBuf>,
    pub i18n: I18n,
    pub confirm_before_delete: bool,
    pub show_delete_confirm_dialog: bool,
    // 新增：主题、全屏、缩略图
    pub current_theme: Theme,
    pub fullscreen: bool,
    pub show_thumbnails: bool,
    pub theme_changed: bool,
    // 末尾过渡页
    pub at_end_page: bool,
    // 状态消息时间戳（用于自动清除）
    pub status_message_time: Option<std::time::Instant>,
    // 快捷键冲突缓存
    pub shortcut_conflicts: Vec<String>,
    pub shortcut_conflicts_dirty: bool,
    // 文件存在性检查节流
    pub last_file_check: std::time::Instant,
    // metadata缓存
    pub cached_file_size: String,
    pub cached_file_size_index: usize,
    pub cached_resolution: String,
    // GIF 动画
    pub gif_animator: Option<GifAnimator>,
    pub dual_left_gif_animator: Option<GifAnimator>,
    pub dual_right_gif_animator: Option<GifAnimator>,
    // 预加载标记
    pub needs_preload: bool,
    // 图片信息窗口
    pub show_image_info_window: bool,
    pub image_info_cache: Vec<(String, Vec<(String, String)>)>,
    pub image_info_loaded_index: usize,
    // 幻灯片放映
    pub slideshow_active: bool,
    pub slideshow_timer: std::time::Instant,
    pub slideshow_interval_secs: f64,
    pub image_sort_mode: ImageSortMode,
    pub image_sort_reversed: bool,
    // 单实例 IPC：接收从命令行/文件关联传入的路径
    pub ipc_rx: std::sync::mpsc::Receiver<String>,
}

impl Default for JinnImageViewer {
    fn default() -> Self {
        let (_tx, rx) = std::sync::mpsc::channel();
        Self::new(rx)
    }
}

impl JinnImageViewer {
    pub fn new(ipc_rx: std::sync::mpsc::Receiver<String>) -> Self {
        Self {
            images: Vec::new(),
            current_index: 0,
            scale_factor: 1.0,
            fit_to_window: true,
            two_column: false,
            show_shortcuts_window: false,
            show_about_window: false,
            shortcuts: ShortcutConfig::defaults(),
            texture_manager: TextureManager::new(),
            thumbnail_manager: ThumbnailManager::new(),
            folder_path: None,
            initialized: false,
            window_title: String::new(),
            status_message: String::new(),
            zoom_step_percent: 10.0,
            show_copy_error_dialog: false,
            copy_error_msg: String::new(),
            show_large_image_dialog: false,
            large_image_path: None,
            approved_large_image_path: None,
            i18n: I18n::new(),
            confirm_before_delete: false,
            show_delete_confirm_dialog: false,
            current_theme: Theme::ClassicDark,
            fullscreen: false,
            show_thumbnails: true,
            theme_changed: false,
            at_end_page: false,
            status_message_time: None,
            shortcut_conflicts: Vec::new(),
            shortcut_conflicts_dirty: true,
            last_file_check: std::time::Instant::now(),
            cached_file_size: String::new(),
            cached_file_size_index: usize::MAX,
            cached_resolution: String::new(),
            gif_animator: None,
            dual_left_gif_animator: None,
            dual_right_gif_animator: None,
            needs_preload: false,
            show_image_info_window: false,
            image_info_cache: Vec::new(),
            image_info_loaded_index: usize::MAX,
            slideshow_active: false,
            slideshow_timer: std::time::Instant::now(),
            slideshow_interval_secs: 3.0,
            image_sort_mode: ImageSortMode::Date,
            image_sort_reversed: false,
            ipc_rx,
        }
    }

    /// 从配置文件加载设置
    pub fn load_config(&mut self) {
        let config = AppConfig::load();
        self.i18n.lang = match config.language.as_str() {
            "en" => Language::English,
            _ => Language::Chinese,
        };
        self.current_theme = match config.theme.as_str() {
            "DeepSpace" => Theme::DeepSpace,
            "CyberPurple" => Theme::CyberPurple,
            "AuroraGreen" => Theme::AuroraGreen,
            "MinimalGray" => Theme::MinimalGray,
            _ => Theme::ClassicDark,
        };
        self.show_thumbnails = config.show_thumbnails;
        self.confirm_before_delete = config.confirm_before_delete;
        self.zoom_step_percent = config.zoom_step_percent;
        self.fit_to_window = config.fit_to_window;
        self.two_column = config.two_column;
        self.fullscreen = config.fullscreen;
        self.slideshow_interval_secs = config.slideshow_interval_secs.clamp(1.0, 60.0);
        self.image_sort_mode = match config.image_sort_mode.as_str() {
            "size" => ImageSortMode::Size,
            "name" => ImageSortMode::Name,
            _ => ImageSortMode::Date,
        };
        self.image_sort_reversed = config.image_sort_reversed;
        let mut shortcut_occurrences = HashMap::new();
        for saved in &config.shortcuts {
            let Some(action) = ShortcutAction::from_i18n_key(&saved.action) else {
                continue;
            };
            let Some(key) = key_from_label(&saved.key) else {
                continue;
            };
            let occurrence = shortcut_occurrences.entry(saved.action.clone()).or_insert(0);
            let entry = self
                .shortcuts
                .entries
                .iter_mut()
                .filter(|entry| entry.action == action)
                .nth(*occurrence);
            *occurrence += 1;
            if let Some(entry) = entry {
                entry.key = key;
                entry.modifiers = egui::Modifiers {
                    alt: saved.alt,
                    shift: saved.shift,
                    command: saved.ctrl,
                    ..egui::Modifiers::NONE
                };
            }
        }
        if let Some(last_folder) = config.last_folder.as_deref() {
            let path = PathBuf::from(last_folder);
            if path.is_dir() {
                self.load_folder(path);
            }
        }
        self.theme_changed = true;
    }

    /// 保存当前设置到配置文件
    pub fn save_config(&mut self) -> bool {
        let config = AppConfig {
            language: match self.i18n.lang {
                Language::English => "en".to_string(),
                Language::Chinese => "zh".to_string(),
            },
            theme: match self.current_theme {
                Theme::ClassicDark => "ClassicDark".to_string(),
                Theme::DeepSpace => "DeepSpace".to_string(),
                Theme::CyberPurple => "CyberPurple".to_string(),
                Theme::AuroraGreen => "AuroraGreen".to_string(),
                Theme::MinimalGray => "MinimalGray".to_string(),
            },
            show_thumbnails: self.show_thumbnails,
            confirm_before_delete: self.confirm_before_delete,
            zoom_step_percent: self.zoom_step_percent,
            fit_to_window: self.fit_to_window,
            two_column: self.two_column,
            fullscreen: self.fullscreen,
            last_folder: self.folder_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            slideshow_interval_secs: self.slideshow_interval_secs,
            image_sort_mode: match self.image_sort_mode {
                ImageSortMode::Size => "size".to_string(),
                ImageSortMode::Date => "date".to_string(),
                ImageSortMode::Name => "name".to_string(),
            },
            image_sort_reversed: self.image_sort_reversed,
            shortcuts: self
                .shortcuts
                .entries
                .iter()
                .map(|entry| ShortcutSetting {
                    action: entry.action.i18n_key().to_string(),
                    key: key_label(entry.key).to_string(),
                    alt: entry.modifiers.alt,
                    shift: entry.modifiers.shift,
                    ctrl: entry.modifiers.command,
                })
                .collect(),
        };
        if let Err(error) = config.save() {
            self.set_status(format!("{}: {}", self.i18n.t("status_config_save_failed"), error));
            return false;
        }
        true
    }

    pub fn window_title(&self) -> String {
        if let Some(image) = self.images.get(self.current_index) {
            let name = if image.name.len() > 40 {
                format!("{}...", &image.name[..40])
            } else {
                image.name.clone()
            };
            format!("{} - Jinn Image Viewer", name)
        } else {
            "Jinn Image Viewer".to_string()
        }
    }

    /// 从命令行参数加载图片或文件夹（支持多文件）
    pub fn load_from_args(&mut self) {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if args.is_empty() {
            return;
        }

        let first_path = PathBuf::from(&args[0]);

        // 如果第一个参数是文件夹，直接打开
        if first_path.is_dir() {
            self.load_folder(first_path);
            return;
        }

        // 如果是图片文件，加载其所在目录并定位到该文件
        if first_path.is_file() {
            if let Some(ext) = first_path.extension() {
                if crate::image::is_supported_ext(ext) {
                    if let Some(parent) = first_path.parent() {
                        self.load_folder(parent.to_path_buf());
                        let file_name = first_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        if let Some(pos) = self.images.iter().position(|img| img.name == file_name) {
                            self.current_index = pos;
                        }
                    }
                }
            }
        }
    }

    /// 注册文件关联（设为默认图片程序）
    pub fn register_file_association(&mut self) {
        let exe_path = std::env::current_exe().unwrap_or_default();
        let exe_str = exe_path.to_string_lossy().replace('/', "\\");

        let script = format!(
            r#"
            $ErrorActionPreference = 'Stop'
            try {{
                $exePath = '{}'
                # 注册 ProgID
                New-Item -Path 'Registry::HKEY_CURRENT_USER\Software\Classes\JinnImageViewer.Image' -Force | Out-Null
                Set-ItemProperty -Path 'Registry::HKEY_CURRENT_USER\Software\Classes\JinnImageViewer.Image' -Name '(Default)' -Value 'Jinn Image Viewer'
                New-Item -Path 'Registry::HKEY_CURRENT_USER\Software\Classes\JinnImageViewer.Image\shell\open\command' -Force | Out-Null
                Set-ItemProperty -Path 'Registry::HKEY_CURRENT_USER\Software\Classes\JinnImageViewer.Image\shell\open\command' -Name '(Default)' -Value "`"$exePath`" `"%1`""
                New-Item -Path 'Registry::HKEY_CURRENT_USER\Software\Classes\JinnImageViewer.Image\DefaultIcon' -Force | Out-Null
                Set-ItemProperty -Path 'Registry::HKEY_CURRENT_USER\Software\Classes\JinnImageViewer.Image\DefaultIcon' -Name '(Default)' -Value "`"$exePath`",0"
                # 关联扩展名
                $exts = @('.png','.jpg','.jpeg','.bmp','.gif','.webp','.tiff','.tif')
                foreach ($ext in $exts) {{
                    New-Item -Path "Registry::HKEY_CURRENT_USER\Software\Classes\$ext\OpenWithProgids" -Force | Out-Null
                    Set-ItemProperty -Path "Registry::HKEY_CURRENT_USER\Software\Classes\$ext\OpenWithProgids" -Name 'JinnImageViewer.Image' -Value '' -Type String
                }}
                # 注册 Applications
                New-Item -Path 'Registry::HKEY_CURRENT_USER\Software\Classes\Applications\jinn-imageviewer.exe\shell\open\command' -Force | Out-Null
                Set-ItemProperty -Path 'Registry::HKEY_CURRENT_USER\Software\Classes\Applications\jinn-imageviewer.exe\shell\open\command' -Name '(Default)' -Value "`"$exePath`" `"%1`""
                # 注册 Registered Applications (使程序出现在系统默认应用设置)
                New-Item -Path 'Registry::HKEY_CURRENT_USER\Software\RegisteredApplications' -Force | Out-Null
                Set-ItemProperty -Path 'Registry::HKEY_CURRENT_USER\Software\RegisteredApplications' -Name 'JinnImageViewer' -Value 'Software\Clients\Media\JinnImageViewer\Capabilities'
                # 注册 Capabilities
                New-Item -Path 'Registry::HKEY_CURRENT_USER\Software\Clients\Media\JinnImageViewer\Capabilities' -Force | Out-Null
                Set-ItemProperty -Path 'Registry::HKEY_CURRENT_USER\Software\Clients\Media\JinnImageViewer\Capabilities' -Name 'ApplicationName' -Value 'Jinn Image Viewer'
                Set-ItemProperty -Path 'Registry::HKEY_CURRENT_USER\Software\Clients\Media\JinnImageViewer\Capabilities' -Name 'ApplicationDescription' -Value 'Fast and lightweight image viewer'
                New-Item -Path 'Registry::HKEY_CURRENT_USER\Software\Clients\Media\JinnImageViewer\Capabilities\FileAssociations' -Force | Out-Null
                foreach ($ext in $exts) {{
                    Set-ItemProperty -Path 'Registry::HKEY_CURRENT_USER\Software\Clients\Media\JinnImageViewer\Capabilities\FileAssociations' -Name $ext -Value 'JinnImageViewer.Image'
                }}
                exit 0
            }} catch {{
                exit 1
            }}
            "#,
            exe_str
        );

        let result = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                self.set_status(self.i18n.t("status_register_ok").to_string());
            }
            _ => {
                self.set_status(self.i18n.t("status_register_fail").to_string());
            }
        }
    }

    /// 取消文件关联
    pub fn unregister_file_association(&mut self) {
        let script = r#"
            $ErrorActionPreference = 'Stop'
            try {
                $exts = @('.png','.jpg','.jpeg','.bmp','.gif','.webp','.tiff','.tif')
                foreach ($ext in $exts) {
                    Remove-ItemProperty -Path "Registry::HKEY_CURRENT_USER\Software\Classes\$ext\OpenWithProgids" -Name 'JinnImageViewer.Image' -ErrorAction SilentlyContinue
                }
                Remove-Item -Path 'Registry::HKEY_CURRENT_USER\Software\Classes\JinnImageViewer.Image' -Recurse -Force -ErrorAction SilentlyContinue
                Remove-Item -Path 'Registry::HKEY_CURRENT_USER\Software\Classes\Applications\jinn-imageviewer.exe' -Recurse -Force -ErrorAction SilentlyContinue
                Remove-ItemProperty -Path 'Registry::HKEY_CURRENT_USER\Software\RegisteredApplications' -Name 'JinnImageViewer' -ErrorAction SilentlyContinue
                Remove-Item -Path 'Registry::HKEY_CURRENT_USER\Software\Clients\Media\JinnImageViewer' -Recurse -Force -ErrorAction SilentlyContinue
                exit 0
            } catch {
                exit 1
            }
        "#;

        let result = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                self.set_status(self.i18n.t("status_unregister_ok").to_string());
            }
            _ => {
                self.set_status(self.i18n.t("status_unregister_fail").to_string());
            }
        }
    }
    #[cfg(target_os = "windows")]
    pub fn set_dark_titlebar(_ctx: &egui::Context, frame: &mut eframe::Frame) {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(handle) = frame.window_handle() {
            if let RawWindowHandle::Win32(win32_handle) = handle.as_raw() {
                let hwnd = win32_handle.hwnd.get();
                const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
                let dark: u32 = 1;
                unsafe {
                    crate::DwmSetWindowAttribute(
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
    pub fn set_dark_titlebar(_ctx: &egui::Context, _frame: &mut eframe::Frame) {}

    /// 设置状态消息（带自动清除计时）
    pub fn set_status(&mut self, msg: String) {
        self.status_message = msg;
        self.status_message_time = Some(std::time::Instant::now());
    }

    /// 检查并清除超时的状态消息（5秒）
    pub fn clear_expired_status(&mut self) {
        if let Some(time) = self.status_message_time {
            if time.elapsed().as_secs() >= 5 {
                self.status_message.clear();
                self.status_message_time = None;
            }
        }
    }

    /// 打开文件夹
    pub fn open_folder(&mut self) {
        let mut dialog = rfd::FileDialog::new();
        // 设置初始目录为上次打开的文件夹，减少 Shell 枚举时间
        if let Some(ref folder) = self.folder_path {
            dialog = dialog.set_directory(folder);
        }
        if let Some(folder) = dialog.pick_folder() {
            self.load_folder(folder);
        }
    }

    /// 打开最近目录（从配置文件读取，无记录时打开桌面）
    pub fn open_recent_folder(&mut self) {
        let config = crate::config::AppConfig::load();
        if let Some(last) = config.last_folder {
            let path = PathBuf::from(&last);
            if path.is_dir() {
                self.load_folder(path);
                return;
            }
        }
        // 无记录或目录不存在时，打开系统桌面
        if let Some(desktop) = dirs_fallback_desktop() {
            if desktop.is_dir() {
                self.load_folder(desktop);
                return;
            }
        }
        self.set_status(self.i18n.t("status_recent_folder_missing").to_string());
    }

    /// 加载文件夹中的图片
    pub fn load_folder(&mut self, folder: PathBuf) {
        self.texture_manager.clear();
        self.thumbnail_manager.clear();
        self.images.clear();
        self.current_index = 0;
        self.scale_factor = 1.0;
        self.image_info_cache.clear();
        self.image_info_loaded_index = usize::MAX;
        self.status_message.clear();
        self.at_end_page = false;

        self.images = scan_folder(&folder);
        self.image_sort_mode.sort(&mut self.images, self.image_sort_reversed);
        self.folder_path = Some(folder);

        if self.images.is_empty() {
            self.set_status(self.i18n.t("status_no_images_in_folder").to_string());
        }
    }

    /// 导航到指定图片
    pub fn navigate(&mut self, direction: i32) {
        if self.images.is_empty() {
            return;
        }

        // 只有1张图时禁用导航
        if self.images.len() == 1 {
            return;
        }

        // 如果当前在末尾过渡页（单列/偶数双列专用），下一张回到第一张
        if self.at_end_page {
            if direction > 0 {
                self.at_end_page = false;
                self.current_index = 0;
                self.scale_factor = 1.0;
                self.status_message.clear();
                self.last_file_check = std::time::Instant::now() - std::time::Duration::from_secs(2);
                self.texture_manager.cleanup(&self.images, self.current_index);
                return;
            } else {
                // 从过渡页按上一张，回到最后一张
                self.at_end_page = false;
                self.status_message.clear();
                return;
            }
        }

        let len = self.images.len() as i32;
        let step = if self.two_column { 2i32 } else { 1i32 };
        let new_idx = self.current_index as i32 + step * direction;

        if direction > 0 && new_idx >= len {
            if self.two_column && self.is_last_page_with_hint() {
                // 奇数双列：当前已是最后一张且右侧已显示提示，直接回到第一张
                self.current_index = 0;
                self.scale_factor = 1.0;
                self.status_message.clear();
                self.texture_manager.cleanup(&self.images, self.current_index);
            } else {
                // 单列/偶数双列：进入过渡页
                self.at_end_page = true;
                self.status_message.clear();
            }
            return;
        }

        if new_idx < 0 {
            return;
        }

        self.current_index = new_idx as usize;
        self.scale_factor = 1.0;
        self.status_message.clear();
        self.at_end_page = false;
        self.last_file_check = std::time::Instant::now() - std::time::Duration::from_secs(2);
        self.texture_manager.cleanup(&self.images, self.current_index);
        self.needs_preload = true;
    }

    /// 判断当前是否是双列模式下最后一页且右侧无图片（奇数张情况）
    fn is_last_page_with_hint(&self) -> bool {
        if !self.two_column || self.images.is_empty() {
            return false;
        }
        let right_idx = self.current_index + 1;
        // 当前是最后一张，右侧没有图片
        right_idx >= self.images.len()
    }

    /// 请求删除当前图片（考虑确认开关）
    pub fn request_delete(&mut self) {
        if self.images.is_empty() || self.current_index >= self.images.len() {
            return;
        }
        // 过渡页时不允许删除
        if self.at_end_page {
            return;
        }
        if self.confirm_before_delete {
            self.show_delete_confirm_dialog = true;
        } else {
            self.delete_current();
        }
    }

    /// 实际执行删除当前图片（移入回收站）
    pub fn delete_current(&mut self) {
        if self.images.is_empty() || self.current_index >= self.images.len() {
            return;
        }
        let path = self.images[self.current_index].path.clone();
        match trash::delete(&path) {
            Ok(_) => {
                self.remove_entry_at(self.current_index);
                let msg = format!(
                    "{}: {}",
                    self.i18n.t("status_deleted"),
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
                self.set_status(msg);
            }
            Err(e) => {
                if !path.exists() {
                    self.remove_entry_at(self.current_index);
                    self.set_status(self.i18n.t("status_file_missing").to_string());
                } else {
                    self.set_status(format!("{}: {}", self.i18n.t("status_delete_failed"), e));
                }
            }
        }
    }

    /// 从列表中移除指定索引的条目并调整 current_index
    pub fn remove_entry_at(&mut self, idx: usize) {
        if idx >= self.images.len() {
            return;
        }
        let path = self.images[idx].path.clone();
        self.texture_manager.remove(&path);
        self.thumbnail_manager.remove(&path);
        self.images.remove(idx);
        self.image_info_cache.clear();
        self.image_info_loaded_index = usize::MAX;
        self.at_end_page = false;
        if self.images.is_empty() {
            self.current_index = 0;
            self.set_status(self.i18n.t("status_all_deleted").to_string());
        } else if self.current_index >= self.images.len() {
            self.current_index = self.images.len() - 1;
        }
        self.texture_manager.cleanup(&self.images, self.current_index);
    }

    /// 检测当前图片是否存在，不存在则从列表移除
    /// 节流：每秒最多检查一次
    pub fn verify_current_file(&mut self) -> bool {
        if self.images.is_empty() || self.current_index >= self.images.len() {
            return false;
        }
        // 每秒检查一次
        if self.last_file_check.elapsed().as_millis() < 1000 {
            return true;
        }
        self.last_file_check = std::time::Instant::now();

        if !self.images[self.current_index].path.exists() {
            self.remove_entry_at(self.current_index);
            self.set_status(self.i18n.t("status_file_missing").to_string());
            return false;
        }
        true
    }

    /// 刷新当前文件夹（重新扫描）
    pub fn refresh_folder(&mut self) {
        if let Some(folder) = self.folder_path.clone() {
            let old_name = if self.current_index < self.images.len() {
                Some(self.images[self.current_index].name.clone())
            } else {
                None
            };

            self.texture_manager.clear();
            self.thumbnail_manager.clear();
            self.images = scan_folder(&folder);
            self.image_sort_mode.sort(&mut self.images, self.image_sort_reversed);
            self.image_info_cache.clear();
            self.image_info_loaded_index = usize::MAX;

            if self.images.is_empty() {
                self.current_index = 0;
                self.set_status(self.i18n.t("status_no_images_in_folder").to_string());
            } else {
                // 尝试保持在原来的图片位置
                if let Some(name) = old_name {
                    if let Some(pos) = self.images.iter().position(|img| img.name == name) {
                        self.current_index = pos;
                    } else {
                        self.current_index = self.current_index.min(self.images.len() - 1);
                    }
                }
                self.set_status(self.i18n.t("status_refreshed").to_string());
            }
        }
    }

    pub fn set_image_sort_mode(&mut self, mode: ImageSortMode) {
        if self.image_sort_mode == mode && !self.image_sort_reversed {
            return;
        }
        self.image_sort_mode = mode;
        self.image_sort_reversed = false;
        self.reorder_images();
    }

    pub fn reverse_image_sort(&mut self) {
        self.image_sort_reversed = !self.image_sort_reversed;
        self.reorder_images();
    }

    fn reorder_images(&mut self) {
        let current_path = self.images.get(self.current_index).map(|image| image.path.clone());
        self.image_sort_mode.sort(&mut self.images, self.image_sort_reversed);
        self.current_index = current_path
            .and_then(|path| self.images.iter().position(|image| image.path == path))
            .unwrap_or(0);
        self.at_end_page = false;
        self.texture_manager.cleanup(&self.images, self.current_index);
        self.thumbnail_manager.cleanup(&self.images);
    }

    /// 复制图片
    pub fn copy_image(&mut self, offset: usize) {
        if self.images.is_empty() || self.at_end_page {
            return;
        }
        let idx = self.current_index + offset;
        if idx >= self.images.len() {
            self.set_status(self.i18n.t("status_no_right_image").to_string());
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
            self.copy_error_msg = self.i18n.t("status_copy_exists").to_string();
            self.show_copy_error_dialog = true;
            return;
        }

        match std::fs::copy(&src, &dest) {
            Ok(_) => {
                self.set_status(format!("{}: {}", self.i18n.t("status_copy_path"), dest.display()));
            }
            Err(e) => {
                self.set_status(format!("{}: {}", self.i18n.t("status_copy_failed"), e));
            }
        }
    }

    /// 处理鼠标滚轮缩放
    pub fn handle_zoom(&mut self, ctx: &egui::Context) {
        let scroll_y = ctx.input(|i| {
            let mut y = 0.0f32;
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

        if scroll_y.abs() < 0.1 {
            return;
        }

        // 消费掉 smooth_scroll_delta，防止 ScrollArea::both() 再将其用于图片平移
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

    /// 处理输入事件
    pub fn handle_input(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.wants_keyboard_input() {
            return;
        }

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
            match action {
                ShortcutAction::OpenFolder => self.open_folder(),
                ShortcutAction::PrevImage => self.navigate(-1),
                ShortcutAction::NextImage => self.navigate(1),
                ShortcutAction::DeleteImage => self.request_delete(),
                ShortcutAction::ToggleDual => {
                    self.two_column = !self.two_column;
                    self.at_end_page = false;
                    let _ = self.save_config();
                }
                ShortcutAction::CopyLeft => self.copy_image(0),
                ShortcutAction::CopyRight => {
                    if self.two_column {
                        self.copy_image(1);
                    }
                }
                ShortcutAction::ToggleFit => {
                    self.fit_to_window = !self.fit_to_window;
                    let _ = self.save_config();
                }
                ShortcutAction::About => self.show_about_window = true,
                ShortcutAction::ToggleFullscreen => self.toggle_fullscreen(ctx),
                ShortcutAction::RotateCW => self.rotate_cw(),
                ShortcutAction::RotateCCW => self.rotate_ccw(),
                ShortcutAction::ShowEXIF => {
                    // 切换图片信息窗口
                    if !self.show_image_info_window {
                        // 打开时刷新缓存
                        if self.current_index < self.images.len() {
                            self.image_info_cache.clear();
                            self.image_info_loaded_index = usize::MAX;
                        }
                    }
                    self.show_image_info_window = !self.show_image_info_window;
                }
                ShortcutAction::ToggleSlideshow => {
                    self.slideshow_active = !self.slideshow_active;
                    if self.slideshow_active {
                        self.slideshow_timer = std::time::Instant::now();
                    }
                    let _ = self.save_config();
                }
            }
        }
    }

    /// 切换全屏模式
    pub fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        self.fullscreen = !self.fullscreen;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
        let _ = self.save_config();
    }

    /// 顺时针旋转90°
    pub fn rotate_cw(&mut self) {
        if self.current_index < self.images.len() {
            let entry = &mut self.images[self.current_index];
            entry.manual_rotation = (entry.manual_rotation + 90) % 360;
            let path = entry.path.clone();
            self.texture_manager.remove(&path);
        }
    }

    /// 逆时针旋转90°
    pub fn rotate_ccw(&mut self) {
        if self.current_index < self.images.len() {
            let entry = &mut self.images[self.current_index];
            entry.manual_rotation = (entry.manual_rotation + 270) % 360;
            let path = entry.path.clone();
            self.texture_manager.remove(&path);
        }
    }

    /// 保存当前旋转到文件（实际旋转像素并覆盖写入）
    pub fn save_rotation(&mut self) {
        if self.current_index >= self.images.len() {
            return;
        }
        let entry = &self.images[self.current_index];
        let rotation = entry.manual_rotation;
        if rotation == 0 {
            return;
        }
        let path = entry.path.clone();

        let result = (|| -> Result<(), String> {
            let img = image::ImageReader::open(&path)
                .map_err(|e| e.to_string())?
                .decode()
                .map_err(|e| e.to_string())?;

            let rotated = match rotation {
                90 => img.rotate90(),
                180 => img.rotate180(),
                270 => img.rotate270(),
                _ => img,
            };

            rotated.save(&path).map_err(|e| e.to_string())?;
            Ok(())
        })();

        match result {
            Ok(()) => {
                // 重置旋转角度并刷新纹理
                self.images[self.current_index].manual_rotation = 0;
                self.texture_manager.remove(&path);
                self.thumbnail_manager.remove(&path);
                self.set_status(self.i18n.t("status_rotation_saved").to_string());
            }
            Err(e) => {
                self.set_status(format!("{}: {}", self.i18n.t("status_rotation_save_fail"), e));
            }
        }
    }

    /// 打印当前图片（调用 Windows 系统打印）
    pub fn print_current(&mut self) {
        if self.current_index >= self.images.len() {
            return;
        }
        let path = self.images[self.current_index].path.clone();
        let path_str = path.to_string_lossy().to_string();

        // 使用 Windows 的 ShellExecute "print" verb
        let result = std::process::Command::new("rundll32")
            .args(["shimgvw.dll,ImageView_PrintTo", &path_str])
            .spawn();

        if result.is_err() {
            // 备用方案：直接用 print verb
            let result2 = std::process::Command::new("cmd")
                .args(["/c", "start", "/min", "mspaint", "/p", &path_str])
                .spawn();
            if result2.is_err() {
                self.set_status(self.i18n.t("status_print_fail").to_string());
            }
        }
    }

    /// 处理拖放文件事件（提前短路，无事件时零分配）
    pub fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        // 先检查是否有拖放事件，避免无用的 Vec 分配
        let has_dropped = ctx.input(|i| !i.raw.dropped_files.is_empty());
        if !has_dropped {
            return;
        }

        let first_path: Option<PathBuf> = ctx.input(|i| i.raw.dropped_files.first().and_then(|f| f.path.clone()));

        let first_file = match first_path {
            Some(p) => p,
            None => return,
        };

        // 如果拖入的是文件夹，直接打开该文件夹
        if first_file.is_dir() {
            self.load_folder(first_file);
            return;
        }

        // 判断是否是支持的图片文件
        if let Some(ext) = first_file.extension() {
            if !crate::image::is_supported_ext(ext) {
                return;
            }
        } else {
            return;
        }

        // 获取所在文件夹
        if let Some(parent) = first_file.parent() {
            let folder = parent.to_path_buf();
            self.load_folder(folder);

            // 定位到拖入的那张图片
            let file_name = first_file.file_name().unwrap_or_default().to_string_lossy().to_string();
            if let Some(pos) = self.images.iter().position(|img| img.name == file_name) {
                self.current_index = pos;
            }
        }
    }

    /// 处理通过 IPC 传入的路径（文件关联双击或第二个实例的命令行参数）。
    /// 该路径将被当作文件打开，自动加载其所在目录并定位到该文件。
    fn handle_ipc_paths(&mut self, ctx: &egui::Context) {
        while let Ok(path_str) = self.ipc_rx.try_recv() {
            let path = std::path::PathBuf::from(&path_str);
            if path.is_dir() {
                self.load_folder(path);
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    if !crate::image::is_supported_ext(ext) {
                        continue;
                    }
                } else {
                    continue;
                }
                if let Some(parent) = path.parent() {
                    self.load_folder(parent.to_path_buf());
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    if let Some(pos) = self.images.iter().position(|img| img.name == file_name) {
                        self.current_index = pos;
                    }
                }
            }
            ctx.request_repaint();
        }
    }
}

// ============================================================================
// eframe::App trait 实现
// ============================================================================
impl eframe::App for JinnImageViewer {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.initialized {
            self.initialized = true;
            Self::set_dark_titlebar(ctx, frame);
            self.current_theme.apply(ctx);
            if self.fullscreen {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
        }

        // 主题变更时重新应用
        if self.theme_changed {
            self.theme_changed = false;
            self.current_theme.apply(ctx);
        }

        // 处理拖放文件
        self.handle_dropped_files(ctx);

        // 处理通过 IPC 传入的路径（来自文件关联双击或第二个实例的命令行参数）
        self.handle_ipc_paths(ctx);

        // 自动清除超时状态消息
        self.clear_expired_status();

        // 处理快捷键输入
        self.handle_input(ctx, frame);

        // 仅在标题变化时更新窗口标题，避免空闲状态持续触发重绘
        let title = self.window_title();
        if self.window_title != title {
            self.window_title = title.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }

        // 全屏模式下隐藏菜单和状态栏
        if !self.fullscreen {
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
        }

        // 缩略图轨道（非全屏且开启时显示）
        if self.show_thumbnails && !self.fullscreen && !self.images.is_empty() {
            egui::TopBottomPanel::bottom("thumbnail_panel")
                .min_height(80.0)
                .max_height(100.0)
                .show(ctx, |ui| {
                    self.show_thumbnail_track(ui, ctx);
                });
        }

        // 中央图片区域
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_image_area(ui, ctx);
        });

        // 普通图片走同步缓存路径，不使用后台队列。
        if self.needs_preload && !self.images.is_empty() {
            let current = self.current_index;
            let len = self.images.len();
            let step = if self.two_column { 2 } else { 1 };

            // 预加载顺序：+1, +step, -1, +step*2（覆盖前后和双列下一对）
            let preload_offsets: &[i32] = &[1, step, -1, step * 2];
            let mut loaded_one = false;

            for &offset in preload_offsets {
                let target = current as i32 + offset;
                if target >= 0 && (target as usize) < len && target as usize != current {
                    let idx = target as usize;
                    let path = self.images[idx].path.clone();
                    let rot = self.images[idx].manual_rotation;
                    let _ = self.texture_manager.get_or_load_with_rotation(ctx, &path, rot, 1200);
                    loaded_one = true;
                    break;
                }
            }

            if !loaded_one {
                self.needs_preload = false; // 全部已预加载完成
            }
        }

        // 对话框
        if self.show_copy_error_dialog {
            self.show_copy_error_dialog_ui(ctx);
        }

        if self.show_large_image_dialog {
            self.show_large_image_dialog_ui(ctx);
        }

        if self.show_shortcuts_window {
            let mut open = self.show_shortcuts_window;
            let mut request_close = false;
            egui::Window::new(self.i18n.t("settings_title"))
                .open(&mut open)
                .resizable(true)
                .default_width(420.0)
                .default_height(560.0)
                .show(ctx, |ui| {
                    self.show_shortcuts_window_content(ui, &mut request_close);
                });
            if request_close {
                open = false;
            }
            self.show_shortcuts_window = open;
        }

        if self.show_about_window {
            self.show_about_window_content(ctx);
        }

        if self.show_delete_confirm_dialog {
            self.show_delete_confirm_dialog(ctx);
        }

        // 全屏模式下显示退出提示
        if self.fullscreen {
            let hint_text = self.i18n.t("fullscreen_exit_hint");
            let rect = ctx.screen_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("fullscreen_hint"),
            ));
            let text_pos = egui::Pos2::new(rect.center().x, rect.top() + 30.0);
            painter.text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                hint_text,
                egui::FontId::proportional(14.0),
                egui::Color32::from_rgba_premultiplied(200, 200, 200, 180),
            );
        }

        // 非全屏模式下幻灯片提示
        if self.slideshow_active {
            // Item 12: 幻灯片放映自动翻页（每 3 秒）
            if self.slideshow_timer.elapsed() >= std::time::Duration::from_secs_f64(self.slideshow_interval_secs) {
                if !self.images.is_empty() {
                    self.navigate(1);
                }
                self.slideshow_timer = std::time::Instant::now();
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(500));

            // 右上角幻灯片提示
            let hint_text = self.i18n.t("slideshow_exit_hint");
            let rect = ctx.screen_rect();
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("slideshow_hint"),
            ));
            let text_pos = egui::Pos2::new(rect.right() - 10.0, rect.top() + 10.0);
            painter.text(
                text_pos,
                egui::Align2::RIGHT_TOP,
                hint_text,
                egui::FontId::proportional(14.0),
                egui::Color32::from_rgba_premultiplied(255, 200, 50, 200),
            );
        }

        // 图片信息窗口
        if self.show_image_info_window {
            if self.current_index < self.images.len() {
                // 切换图片时刷新缓存
                if self.image_info_loaded_index != self.current_index {
                    let path = self.images[self.current_index].path.clone();
                    self.image_info_cache = get_image_info(&path);
                    self.image_info_loaded_index = self.current_index;
                }
            }
            self.show_image_info_window_content(ctx);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.save_config();
    }
}
