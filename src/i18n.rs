use std::collections::HashMap;

/// 语言枚举
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Language {
    Chinese,
    English,
}

impl Language {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Chinese => "简体中文",
            Self::English => "English",
        }
    }
}

/// 国际化管理器
pub struct I18n {
    pub lang: Language,
    zh: HashMap<&'static str, &'static str>,
    en: HashMap<&'static str, &'static str>,
}

impl Default for I18n {
    fn default() -> Self {
        Self::new()
    }
}

impl I18n {
    pub fn new() -> Self {
        let mut zh = HashMap::new();
        let mut en = HashMap::new();

        // 菜单
        zh.insert("menu_file", "文件");
        en.insert("menu_file", "File");
        zh.insert("menu_open_folder", "打开文件夹");
        en.insert("menu_open_folder", "Open Folder");
        zh.insert("menu_exit", "退出");
        en.insert("menu_exit", "Exit");
        zh.insert("menu_refresh", "刷新目录");
        en.insert("menu_refresh", "Refresh Folder");
        zh.insert("menu_recent_folder", "打开最近目录");
        en.insert("menu_recent_folder", "Open Recent Folder");
        zh.insert("status_recent_folder_missing", "最近目录不存在");
        en.insert("status_recent_folder_missing", "Recent folder not found");
        zh.insert("menu_view", "视图");
        en.insert("menu_view", "View");
        zh.insert("menu_prev", "上一张");
        en.insert("menu_prev", "Previous");
        zh.insert("menu_next", "下一张");
        en.insert("menu_next", "Next");
        zh.insert("menu_fit_window", "适应窗口");
        en.insert("menu_fit_window", "Fit Window");
        zh.insert("menu_dual_column", "双列展示");
        en.insert("menu_dual_column", "Dual Column");
        zh.insert("menu_image_sort", "图片排列");
        en.insert("menu_image_sort", "Image Order");
        zh.insert("sort_by_size", "按大小排列");
        en.insert("sort_by_size", "Sort by Size");
        zh.insert("sort_by_date", "按日期排序");
        en.insert("sort_by_date", "Sort by Date");
        zh.insert("sort_by_name", "按文件名排列");
        en.insert("sort_by_name", "Sort by File Name");
        zh.insert("sort_reverse", "倒序");
        en.insert("sort_reverse", "Reverse Order");
        zh.insert("menu_settings", "设置");
        en.insert("menu_settings", "Settings");
        zh.insert("menu_actions", "操作");
        en.insert("menu_actions", "Actions");
        zh.insert("menu_hotkeys", "快捷键");
        en.insert("menu_hotkeys", "Hotkeys");
        zh.insert("menu_help", "帮助");
        en.insert("menu_help", "Help");
        zh.insert("menu_shortcuts", "自定义快捷键");
        en.insert("menu_shortcuts", "Customize Shortcuts");
        zh.insert("menu_copy_left", "复制图片(左/当前)");
        en.insert("menu_copy_left", "Copy Image (Left/Current)");
        zh.insert("menu_copy_right", "复制图片(右)");
        en.insert("menu_copy_right", "Copy Image (Right)");
        zh.insert("menu_about", "关于");
        en.insert("menu_about", "About");

        // 快捷键动作
        zh.insert("action_open_folder", "打开文件夹");
        en.insert("action_open_folder", "Open Folder");
        zh.insert("action_prev_image", "上一张");
        en.insert("action_prev_image", "Previous");
        zh.insert("action_next_image", "下一张");
        en.insert("action_next_image", "Next");
        zh.insert("action_delete_image", "删除图片");
        en.insert("action_delete_image", "Delete Image");
        zh.insert("action_toggle_dual", "双列展示");
        en.insert("action_toggle_dual", "Dual Column");
        zh.insert("action_copy_left", "复制图片(左/当前)");
        en.insert("action_copy_left", "Copy Image (Left/Current)");
        zh.insert("action_copy_right", "复制图片(右)");
        en.insert("action_copy_right", "Copy Image (Right)");
        zh.insert("action_toggle_fit", "适应窗口 (鼠标右键)");
        en.insert("action_toggle_fit", "Fit Window (Right Click)");
        zh.insert("action_about", "关于");
        en.insert("action_about", "About");
        zh.insert("action_fullscreen", "全屏模式");
        en.insert("action_fullscreen", "Fullscreen");
        zh.insert("action_show_exif", "图片信息");
        en.insert("action_show_exif", "Image Info");
        zh.insert("image_info_file", "文件信息");
        en.insert("image_info_file", "File Info");
        zh.insert("image_info_image", "图像属性");
        en.insert("image_info_image", "Image Properties");
        zh.insert("image_info_exif", "EXIF 元数据");
        en.insert("image_info_exif", "EXIF Metadata");
        zh.insert("image_info_gps", "GPS 信息");
        en.insert("image_info_gps", "GPS Info");
        zh.insert("action_toggle_slideshow", "幻灯片放映");
        en.insert("action_toggle_slideshow", "Slideshow");
        zh.insert("fullscreen_exit_hint", "按 F11 退出全屏");
        en.insert("fullscreen_exit_hint", "Press F11 to exit fullscreen");
        zh.insert("slideshow_exit_hint", "幻灯片放映中（按 F5 退出）");
        en.insert("slideshow_exit_hint", "Slideshow mode (press F5 to exit)");
        zh.insert("end_page_hint", "已浏览到最后一张图片\n下一页自动回到第一张图片");
        en.insert(
            "end_page_hint",
            "You have reached the last image\nNext page will return to the first image",
        );

        // 设置 - 主题与缩略图
        zh.insert("settings_theme", "主题");
        en.insert("settings_theme", "Theme");
        zh.insert("settings_thumbnails", "显示缩略图轨道");
        en.insert("settings_thumbnails", "Show Thumbnail Track");

        // 状态栏
        zh.insert("status_no_images", "未加载图片 — 双击或按 O 打开文件夹");
        en.insert("status_no_images", "No images — double-click or press O to open folder");
        zh.insert("status_zoom", "缩放");
        en.insert("status_zoom", "Zoom");
        zh.insert("status_right", "右");
        en.insert("status_right", "Right");
        zh.insert("status_all_deleted", "所有图片已删除");
        en.insert("status_all_deleted", "All images deleted");
        zh.insert("status_deleted", "已删除");
        en.insert("status_deleted", "Deleted");
        zh.insert("status_delete_failed", "删除失败");
        en.insert("status_delete_failed", "Delete failed");
        zh.insert("status_file_missing", "文件已被外部删除，已从列表移除");
        en.insert("status_file_missing", "File was deleted externally, removed from list");
        zh.insert("status_image_corrupted", "图片损坏或无法解码，已跳过");
        en.insert("status_image_corrupted", "Image corrupted or unreadable, skipped");
        zh.insert("status_loading", "加载中...");
        en.insert("status_loading", "Loading...");
        zh.insert("large_image_title", "图片加载确认");
        en.insert("large_image_title", "Confirm Image Loading");
        zh.insert(
            "large_image_warning",
            "图片超过推荐限制，加载可能占用大量内存并导致明显卡顿。是否继续？",
        );
        en.insert(
            "large_image_warning",
            "This image exceeds the recommended limit and may use substantial memory or cause visible lag. Continue?",
        );
        zh.insert("large_image_file_size", "文件大小");
        en.insert("large_image_file_size", "File size");
        zh.insert("large_image_pixels", "像素数");
        en.insert("large_image_pixels", "Pixels");
        zh.insert("large_image_load", "继续加载");
        en.insert("large_image_load", "Load Anyway");
        zh.insert("large_image_cancel", "放弃加载");
        en.insert("large_image_cancel", "Cancel");
        zh.insert("large_image_waiting_confirmation", "请确认是否加载此图片");
        en.insert(
            "large_image_waiting_confirmation",
            "Please confirm whether to load this image",
        );
        zh.insert(
            "large_image_unavailable",
            "图片尺寸或解码内存需求超过当前系统可安全处理的范围，无法加载",
        );
        en.insert(
            "large_image_unavailable",
            "This image exceeds the safe dimensions or decode memory available on this system",
        );
        zh.insert("action_rotate", "旋转90°");
        en.insert("action_rotate", "Rotate 90°");
        zh.insert("action_rotate_cw", "顺时针旋转");
        en.insert("action_rotate_cw", "Rotate CW");
        zh.insert("action_rotate_ccw", "逆时针旋转");
        en.insert("action_rotate_ccw", "Rotate CCW");
        zh.insert("ctx_fit_window", "适应窗口");
        en.insert("ctx_fit_window", "Fit Window");
        zh.insert("ctx_rotate_cw", "顺时针旋转 90°");
        en.insert("ctx_rotate_cw", "Rotate 90° CW");
        zh.insert("ctx_rotate_ccw", "逆时针旋转 90°");
        en.insert("ctx_rotate_ccw", "Rotate 90° CCW");
        zh.insert("ctx_copy_image", "复制图片");
        en.insert("ctx_copy_image", "Copy Image");
        zh.insert("ctx_delete", "删除");
        en.insert("ctx_delete", "Delete");
        zh.insert("ctx_open_folder", "打开文件夹");
        en.insert("ctx_open_folder", "Open Folder");
        zh.insert("ctx_refresh", "刷新目录");
        en.insert("ctx_refresh", "Refresh");
        zh.insert("menu_register_assoc", "设为默认图片程序");
        en.insert("menu_register_assoc", "Set as Default Viewer");
        zh.insert("menu_unregister_assoc", "取消文件关联");
        en.insert("menu_unregister_assoc", "Remove File Association");
        zh.insert("status_register_ok", "文件关联注册成功（需要管理员权限）");
        en.insert("status_register_ok", "File association registered (admin required)");
        zh.insert("status_register_fail", "注册失败，请尝试以管理员身份运行程序");
        en.insert(
            "status_register_fail",
            "Registration failed, try running as administrator",
        );
        zh.insert("status_unregister_ok", "文件关联已取消");
        en.insert("status_unregister_ok", "File association removed");
        zh.insert("status_unregister_fail", "取消关联失败");
        en.insert("status_unregister_fail", "Failed to remove association");
        zh.insert("ctx_save_rotation", "保存旋转到文件");
        en.insert("ctx_save_rotation", "Save Rotation to File");
        zh.insert("status_rotation_saved", "旋转已保存到文件");
        en.insert("status_rotation_saved", "Rotation saved to file");
        zh.insert("status_rotation_save_fail", "保存旋转失败");
        en.insert("status_rotation_save_fail", "Failed to save rotation");
        zh.insert("menu_print", "打印");
        en.insert("menu_print", "Print");
        zh.insert("status_print_fail", "打印失败");
        en.insert("status_print_fail", "Print failed");
        zh.insert("status_refreshed", "目录已刷新");
        en.insert("status_refreshed", "Folder refreshed");
        zh.insert("status_no_right_image", "当前没有右图可复制");
        en.insert("status_no_right_image", "No right image to copy");
        zh.insert("status_copy_exists", "图片已存在，已禁止复制。");
        en.insert("status_copy_exists", "Image already exists, copy denied.");
        zh.insert("status_copy_path", "复制后存放路径");
        en.insert("status_copy_path", "Copied to");
        zh.insert("status_copy_failed", "复制失败");
        en.insert("status_copy_failed", "Copy failed");
        zh.insert(
            "status_no_images_in_folder",
            "文件夹中没有找到图片文件 (.png/.jpg/.jpeg/.bmp/.gif/.webp/.tiff)",
        );
        en.insert(
            "status_no_images_in_folder",
            "No image files found in folder (.png/.jpg/.jpeg/.bmp/.gif/.webp/.tiff)",
        );
        zh.insert("status_config_save_failed", "配置保存失败");
        en.insert("status_config_save_failed", "Failed to save configuration");

        // 设置窗口
        zh.insert("settings_title", "自定义快捷键");
        en.insert("settings_title", "Customize Shortcuts");
        zh.insert("settings_zoom_step", "鼠标滚轮缩放步长 (%):");
        en.insert("settings_zoom_step", "Mouse wheel zoom step (%):");
        zh.insert("settings_save", "保存设置");
        en.insert("settings_save", "Save");
        zh.insert("settings_reset", "恢复默认");
        en.insert("settings_reset", "Reset Defaults");
        zh.insert("settings_saved_msg", "快捷键设置已保存");
        en.insert("settings_saved_msg", "Shortcut settings saved");
        zh.insert("settings_reset_msg", "已恢复默认设置");
        en.insert("settings_reset_msg", "Defaults restored");
        zh.insert("settings_conflict", "快捷键冲突");
        en.insert("settings_conflict", "Hotkey conflict");
        zh.insert("settings_conflict_warn", "存在快捷键冲突，请修改后再保存");
        en.insert(
            "settings_conflict_warn",
            "Hotkey conflicts detected, please fix before saving",
        );
        zh.insert("settings_tip", "温馨提示: 鼠标右键在图片区域点击可快速适应窗口");
        en.insert("settings_tip", "Tip: Right-click on image area to fit window");
        zh.insert("settings_close", "关闭窗口");
        en.insert("settings_close", "Close");
        zh.insert("settings_language", "语言");
        en.insert("settings_language", "Language");
        zh.insert("settings_confirm_delete", "删除前确认");
        en.insert("settings_confirm_delete", "Confirm before delete");

        // 删除确认对话框
        zh.insert("delete_confirm_title", "确认删除");
        en.insert("delete_confirm_title", "Confirm Delete");
        zh.insert("delete_confirm_msg", "确定要删除这张图片吗？");
        en.insert("delete_confirm_msg", "Are you sure you want to delete this image?");
        zh.insert("delete_confirm_yes", "确定");
        en.insert("delete_confirm_yes", "Yes");
        zh.insert("delete_confirm_no", "取消");
        en.insert("delete_confirm_no", "Cancel");

        // 关于窗口
        zh.insert("about_title", "关于");
        en.insert("about_title", "About");
        zh.insert("about_author", "作者: 叶子Jinn");
        en.insert("about_author", "Author: YeziJinn");
        zh.insert("about_github", "访问GitHub项目");
        en.insert("about_github", "Visit GitHub");
        zh.insert("about_github_error", "打开GitHub失败");
        en.insert("about_github_error", "Failed to open GitHub");

        // 提示对话框
        zh.insert("dialog_notice", "提示");
        en.insert("dialog_notice", "Notice");
        zh.insert("dialog_ok", "确定");
        en.insert("dialog_ok", "OK");

        // 图片区域
        zh.insert("image_open_hint", "打开文件夹以查看图片\n\n双击此处 或 按 O 键");
        en.insert(
            "image_open_hint",
            "Open a folder to view images\n\nDouble-click here or press O",
        );
        zh.insert("image_load_failed", "加载图片失败");
        en.insert("image_load_failed", "Failed to load image");

        Self {
            lang: Language::Chinese,
            zh,
            en,
        }
    }

    /// 翻译
    pub fn t(&self, key: &'static str) -> &'static str {
        let map = match self.lang {
            Language::Chinese => &self.zh,
            Language::English => &self.en,
        };
        map.get(key).copied().unwrap_or(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chinese_translation() {
        let i18n = I18n::new();
        assert_eq!(i18n.t("menu_file"), "文件");
    }

    #[test]
    fn test_english_translation() {
        let mut i18n = I18n::new();
        i18n.lang = Language::English;
        assert_eq!(i18n.t("menu_file"), "File");
    }

    #[test]
    fn test_missing_key_returns_key() {
        let i18n = I18n::new();
        assert_eq!(i18n.t("nonexistent_key"), "nonexistent_key");
    }
}
