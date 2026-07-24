#![allow(clippy::field_reassign_with_default)]

use eframe::egui;
use jinn_imageviewer::app::JinnImageViewer;
use jinn_imageviewer::image::scan_folder;
use jinn_imageviewer::shortcuts::{key_from_label, key_label};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

/// 创建唯一的临时测试目录
fn setup_test_dir() -> std::io::Result<PathBuf> {
    let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp = std::env::temp_dir().join(format!("jinn_integ_test_{}", id));
    if temp.exists() {
        fs::remove_dir_all(&temp)?;
    }
    fs::create_dir_all(&temp)?;
    Ok(temp)
}

/// 创建1x1像素测试图片
fn create_test_image(path: &Path, format: image::ImageFormat) -> std::io::Result<()> {
    let img = image::DynamicImage::new_rgb8(1, 1);
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), format).unwrap();
    let mut file = fs::File::create(path)?;
    file.write_all(&buf)?;
    Ok(())
}

#[test]
fn test_app_initialization() {
    let app = JinnImageViewer::default();
    assert_eq!(app.images.len(), 0, "新应用应无图片");
    assert_eq!(app.current_index, 0);
    assert_eq!(app.scale_factor, 1.0);
    assert!(!app.two_column, "默认应为单列模式");
    assert_eq!(app.window_title(), "Jinn Image Viewer");
}

#[test]
fn test_window_title_uses_current_image() {
    let temp = setup_test_dir().unwrap();
    create_test_image(&temp.join("current.png"), image::ImageFormat::Png).unwrap();

    let mut app = JinnImageViewer::default();
    app.images = scan_folder(&temp);

    assert_eq!(app.window_title(), "current.png - Jinn Image Viewer");
}

#[test]
fn test_shortcut_key_labels_round_trip() {
    let key = egui::Key::F5;
    assert_eq!(key_from_label(key_label(key)), Some(key));
}

#[test]
fn test_navigate_empty_list() {
    let mut app = JinnImageViewer::default();
    app.navigate(1);
    assert_eq!(app.current_index, 0, "空列表导航应无变化");
    app.navigate(-1);
    assert_eq!(app.current_index, 0);
}

#[test]
fn test_navigate_forward() {
    let temp = setup_test_dir().unwrap();
    create_test_image(&temp.join("a.png"), image::ImageFormat::Png).unwrap();
    create_test_image(&temp.join("b.png"), image::ImageFormat::Png).unwrap();
    create_test_image(&temp.join("c.png"), image::ImageFormat::Png).unwrap();

    let mut app = JinnImageViewer::default();
    app.images = scan_folder(&temp);
    assert_eq!(app.images.len(), 3);

    app.navigate(1);
    assert_eq!(app.current_index, 1, "应前进到索引1");
    app.navigate(1);
    assert_eq!(app.current_index, 2, "应前进到索引2");
}

#[test]
fn test_navigate_backward() {
    let temp = setup_test_dir().unwrap();
    create_test_image(&temp.join("a.png"), image::ImageFormat::Png).unwrap();
    create_test_image(&temp.join("b.png"), image::ImageFormat::Png).unwrap();

    let mut app = JinnImageViewer::default();
    app.images = scan_folder(&temp);
    app.current_index = 1;

    app.navigate(-1);
    assert_eq!(app.current_index, 0, "应后退到索引0");
}

#[test]
fn test_navigate_wrap_around() {
    let temp = setup_test_dir().unwrap();
    create_test_image(&temp.join("a.png"), image::ImageFormat::Png).unwrap();
    create_test_image(&temp.join("b.png"), image::ImageFormat::Png).unwrap();

    let mut app = JinnImageViewer::default();
    app.images = scan_folder(&temp);
    app.current_index = 1;

    // 超出末尾应进入过渡页
    app.navigate(1);
    assert!(app.at_end_page, "应进入末尾过渡页");
}

#[test]
fn test_remove_entry_at() {
    let temp = setup_test_dir().unwrap();
    create_test_image(&temp.join("a.png"), image::ImageFormat::Png).unwrap();
    create_test_image(&temp.join("b.png"), image::ImageFormat::Png).unwrap();
    create_test_image(&temp.join("c.png"), image::ImageFormat::Png).unwrap();

    let mut app = JinnImageViewer::default();
    app.images = scan_folder(&temp);
    assert_eq!(app.images.len(), 3);

    app.remove_entry_at(1);
    assert_eq!(app.images.len(), 2, "应剩余2张图片");
    assert_eq!(app.images[1].name, "c.png", "索引1应为c.png");
}

#[test]
fn test_remove_entry_adjust_index() {
    let temp = setup_test_dir().unwrap();
    create_test_image(&temp.join("a.png"), image::ImageFormat::Png).unwrap();
    create_test_image(&temp.join("b.png"), image::ImageFormat::Png).unwrap();

    let mut app = JinnImageViewer::default();
    app.images = scan_folder(&temp);
    app.current_index = 1;

    app.remove_entry_at(1);
    assert_eq!(app.current_index, 0, "删除当前图片后索引应调整到0");
}

#[test]
fn test_remove_all_entries() {
    let temp = setup_test_dir().unwrap();
    create_test_image(&temp.join("a.png"), image::ImageFormat::Png).unwrap();

    let mut app = JinnImageViewer::default();
    app.images = scan_folder(&temp);
    app.remove_entry_at(0);

    assert_eq!(app.images.len(), 0, "删除所有图片后列表应为空");
    assert_eq!(app.current_index, 0);
}

#[test]
fn test_rotate_cw() {
    let temp = setup_test_dir().unwrap();
    create_test_image(&temp.join("a.png"), image::ImageFormat::Png).unwrap();

    let mut app = JinnImageViewer::default();
    app.images = scan_folder(&temp);

    app.rotate_cw();
    assert_eq!(app.images[0].manual_rotation, 90, "应旋转90度");
    app.rotate_cw();
    assert_eq!(app.images[0].manual_rotation, 180, "应旋转180度");
    app.rotate_cw();
    assert_eq!(app.images[0].manual_rotation, 270, "应旋转270度");
    app.rotate_cw();
    assert_eq!(app.images[0].manual_rotation, 0, "应回到0度");
}

#[test]
fn test_rotate_ccw() {
    let temp = setup_test_dir().unwrap();
    create_test_image(&temp.join("a.png"), image::ImageFormat::Png).unwrap();

    let mut app = JinnImageViewer::default();
    app.images = scan_folder(&temp);

    app.rotate_ccw();
    assert_eq!(app.images[0].manual_rotation, 270, "应旋转270度");
    app.rotate_ccw();
    assert_eq!(app.images[0].manual_rotation, 180, "应旋转180度");
}

#[test]
fn test_rotate_independent_per_image() {
    let temp = setup_test_dir().unwrap();
    create_test_image(&temp.join("a.png"), image::ImageFormat::Png).unwrap();
    create_test_image(&temp.join("b.png"), image::ImageFormat::Png).unwrap();

    let mut app = JinnImageViewer::default();
    app.images = scan_folder(&temp);

    // 旋转第一张
    app.current_index = 0;
    app.rotate_cw();
    assert_eq!(app.images[0].manual_rotation, 90);

    // 切换到第二张
    app.navigate(1);
    assert_eq!(app.images[1].manual_rotation, 0, "第二张应保持0度");

    // 旋转第二张
    app.rotate_cw();
    app.rotate_cw();
    assert_eq!(app.images[1].manual_rotation, 180);

    // 切回第一张，应保持90度
    app.navigate(-1);
    assert_eq!(app.images[0].manual_rotation, 90, "第一张应保持90度");
}

#[test]
fn test_toggle_dual_column() {
    let mut app = JinnImageViewer::default();
    assert!(!app.two_column);

    app.two_column = !app.two_column;
    assert!(app.two_column, "应切换到双列模式");

    app.two_column = !app.two_column;
    assert!(!app.two_column, "应切换回单列模式");
}

#[test]
fn test_scale_factor_modification() {
    let mut app = JinnImageViewer::default();
    let original = app.scale_factor;

    app.scale_factor = 2.0;
    assert_eq!(app.scale_factor, 2.0, "应能修改缩放比例");

    app.scale_factor = original;
    assert_eq!(app.scale_factor, original);
}

#[test]
fn test_set_status_message() {
    let mut app = JinnImageViewer::default();
    app.set_status("测试消息".to_string());
    assert_eq!(app.status_message, "测试消息");
    assert!(app.status_message_time.is_some(), "应设置时间戳");
}

#[test]
fn test_clear_expired_status() {
    let mut app = JinnImageViewer::default();
    app.set_status("临时消息".to_string());

    // 模拟6秒后（超时阈值为5秒）
    app.status_message_time = Some(std::time::Instant::now() - std::time::Duration::from_secs(6));
    app.clear_expired_status();

    assert_eq!(app.status_message, "", "过期消息应被清除");
}
