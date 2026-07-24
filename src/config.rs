use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 配置文件名
const CONFIG_FILE_NAME: &str = "jinn_imageviewer_config.json";

/// 持久化配置结构
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppConfig {
    pub language: String, // "zh" 或 "en"
    pub theme: String,    // 主题名
    pub show_thumbnails: bool,
    pub confirm_before_delete: bool,
    pub zoom_step_percent: f32,
    pub fit_to_window: bool,
    pub two_column: bool,
    #[serde(default)]
    pub fullscreen: bool,
    pub last_folder: Option<String>,
    #[serde(default = "default_slideshow_interval_secs")]
    pub slideshow_interval_secs: f64,
    #[serde(default = "default_image_sort_mode")]
    pub image_sort_mode: String,
    #[serde(default)]
    pub image_sort_reversed: bool,
    #[serde(default)]
    pub shortcuts: Vec<ShortcutSetting>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ShortcutSetting {
    pub action: String,
    pub key: String,
    pub alt: bool,
    pub shift: bool,
    pub ctrl: bool,
}

fn default_slideshow_interval_secs() -> f64 {
    3.0
}

fn default_image_sort_mode() -> String {
    "date".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: "zh".to_string(),
            theme: "ClassicDark".to_string(),
            show_thumbnails: true,
            confirm_before_delete: false,
            zoom_step_percent: 10.0,
            fit_to_window: true,
            two_column: false,
            fullscreen: false,
            last_folder: None,
            slideshow_interval_secs: default_slideshow_interval_secs(),
            image_sort_mode: default_image_sort_mode(),
            image_sort_reversed: false,
            shortcuts: Vec::new(),
        }
    }
}

impl AppConfig {
    /// 获取配置文件路径：优先使用用户本地应用数据目录。
    pub fn config_path() -> PathBuf {
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            let path = PathBuf::from(local_app_data).join("JinnImageViewer");
            if path.exists() || std::fs::create_dir_all(&path).is_ok() {
                return path.join(CONFIG_FILE_NAME);
            }
        }

        // 兜底：exe 同级目录
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        exe_dir.join(CONFIG_FILE_NAME)
    }

    /// 从文件加载配置
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// 保存配置到文件
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_use_newest_first_date_sort() {
        let config = AppConfig::default();
        assert_eq!(config.image_sort_mode, "date");
        assert!(!config.image_sort_reversed);
        assert_eq!(config.slideshow_interval_secs, 3.0);
        assert!(!config.fullscreen);
    }

    #[test]
    fn legacy_config_gets_defaults_for_new_fields() {
        let config: AppConfig = serde_json::from_str(
            r#"{
                "language":"zh",
                "theme":"ClassicDark",
                "show_thumbnails":true,
                "confirm_before_delete":false,
                "zoom_step_percent":10.0,
                "fit_to_window":true,
                "two_column":false,
                "last_folder":null
            }"#,
        )
        .unwrap();
        assert_eq!(config.image_sort_mode, "date");
        assert!(!config.image_sort_reversed);
        assert_eq!(config.slideshow_interval_secs, 3.0);
        assert!(!config.fullscreen);
    }
}
