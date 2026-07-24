use eframe::egui;
use std::path::PathBuf;

use crate::app::JinnImageViewer;
use crate::i18n::Language;
use crate::image::{assess_image_load, fit_size, fit_size_to_max, GifAnimator, ImageLoadAssessment};
use crate::shortcuts::{all_key_options, key_label, ShortcutConfig};
use crate::sorting::ImageSortMode;
use crate::theme::Theme;

/// 格式化文件大小
fn format_file_size(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn get_dual_gif_frame(
    animator: &mut Option<GifAnimator>,
    path: &std::path::Path,
    ctx: &egui::Context,
) -> Option<(egui::TextureId, egui::Vec2, std::time::Duration)> {
    if !GifAnimator::is_gif(path) {
        *animator = None;
        return None;
    }

    let needs_load = animator.as_ref().is_none_or(|current| !current.matches(path));
    if needs_load {
        *animator = GifAnimator::load(path);
    }

    let animator = animator.as_mut()?;
    let texture = animator.current_texture(ctx);
    Some((texture.id(), texture.size_vec2(), animator.current_frame_delay()))
}

impl JinnImageViewer {
    /// 显示菜单栏
    pub fn show_menu(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            // 文件菜单
            ui.menu_button(self.i18n.t("menu_file"), |ui| {
                if ui.button(format!("{}    O", self.i18n.t("menu_open_folder"))).clicked() {
                    ui.close_menu();
                    self.open_folder();
                }
                if ui.button(self.i18n.t("menu_recent_folder")).clicked() {
                    ui.close_menu();
                    self.open_recent_folder();
                }
                if ui.button(self.i18n.t("menu_refresh")).clicked() {
                    ui.close_menu();
                    self.refresh_folder();
                }
                ui.separator();
                if ui.button(format!("{}    Esc", self.i18n.t("menu_exit"))).clicked() {
                    ui.close_menu();
                    let _ = self.save_config();
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            // 视图菜单
            ui.menu_button(self.i18n.t("menu_view"), |ui| {
                if ui
                    .checkbox(
                        &mut self.fit_to_window,
                        format!("{}    V", self.i18n.t("menu_fit_window")),
                    )
                    .clicked()
                {
                    ui.close_menu();
                    let _ = self.save_config();
                }
                if ui
                    .checkbox(
                        &mut self.two_column,
                        format!("{}    F2", self.i18n.t("menu_dual_column")),
                    )
                    .clicked()
                {
                    ui.close_menu();
                    let _ = self.save_config();
                }
                if ui
                    .checkbox(&mut self.show_thumbnails, self.i18n.t("settings_thumbnails"))
                    .clicked()
                {
                    ui.close_menu();
                    let _ = self.save_config();
                }
                if ui
                    .checkbox(&mut self.confirm_before_delete, self.i18n.t("settings_confirm_delete"))
                    .clicked()
                {
                    ui.close_menu();
                    let _ = self.save_config();
                }
                ui.separator();
                ui.label(self.i18n.t("menu_image_sort"));
                let mut sort_by_size = self.image_sort_mode.is_size();
                if ui.checkbox(&mut sort_by_size, self.i18n.t("sort_by_size")).clicked() {
                    ui.close_menu();
                    self.set_image_sort_mode(ImageSortMode::Size);
                    let _ = self.save_config();
                }
                let mut sort_by_date = matches!(self.image_sort_mode, ImageSortMode::Date);
                if ui.checkbox(&mut sort_by_date, self.i18n.t("sort_by_date")).clicked() {
                    ui.close_menu();
                    self.set_image_sort_mode(ImageSortMode::Date);
                    let _ = self.save_config();
                }
                let mut sort_by_name = self.image_sort_mode.is_name();
                if ui.checkbox(&mut sort_by_name, self.i18n.t("sort_by_name")).clicked() {
                    ui.close_menu();
                    self.set_image_sort_mode(ImageSortMode::Name);
                    let _ = self.save_config();
                }
                let mut reverse = self.image_sort_reversed;
                if ui.checkbox(&mut reverse, self.i18n.t("sort_reverse")).clicked() {
                    ui.close_menu();
                    self.reverse_image_sort();
                    let _ = self.save_config();
                }
                ui.separator();
                // 图片信息
                if ui
                    .button(format!("{}    F3", self.i18n.t("action_show_exif")))
                    .clicked()
                {
                    ui.close_menu();
                    self.show_image_info_window = !self.show_image_info_window;
                }
                // 幻灯片放映
                if ui
                    .button(format!("{}    F5", self.i18n.t("action_toggle_slideshow")))
                    .clicked()
                {
                    ui.close_menu();
                    self.slideshow_active = !self.slideshow_active;
                    if self.slideshow_active {
                        self.slideshow_timer = std::time::Instant::now();
                    }
                    let _ = self.save_config();
                }
                // 幻灯片间隔（始终可见可调）
                ui.horizontal(|ui| {
                    ui.label(match self.i18n.lang {
                        Language::Chinese => "幻灯片间隔(秒):",
                        _ => "Slideshow Interval(s):",
                    });
                    let response = ui.add(
                        egui::DragValue::new(&mut self.slideshow_interval_secs)
                            .range(1.0..=60.0)
                            .speed(0.5),
                    );
                    if response.changed() {
                        let _ = self.save_config();
                    }
                    if self.slideshow_active {
                        ui.label(
                            egui::RichText::new(match self.i18n.lang {
                                Language::Chinese => "▶ 放映中",
                                _ => "▶ Running",
                            })
                            .color(egui::Color32::YELLOW)
                            .size(12.0),
                        );
                    }
                });
                ui.separator();
                if ui
                    .button(format!("{}    F11", self.i18n.t("action_fullscreen")))
                    .clicked()
                {
                    ui.close_menu();
                    self.toggle_fullscreen(ui.ctx());
                }
                ui.separator();
                // 语言切换
                ui.horizontal(|ui| {
                    ui.label(self.i18n.t("settings_language"));
                    if ui
                        .selectable_label(self.i18n.lang == Language::Chinese, Language::Chinese.label())
                        .clicked()
                    {
                        self.i18n.lang = Language::Chinese;
                        let _ = self.save_config();
                        ui.ctx().request_repaint();
                    }
                    if ui
                        .selectable_label(self.i18n.lang == Language::English, Language::English.label())
                        .clicked()
                    {
                        self.i18n.lang = Language::English;
                        let _ = self.save_config();
                        ui.ctx().request_repaint();
                    }
                });
                // 主题切换
                ui.horizontal(|ui| {
                    ui.label(self.i18n.t("settings_theme"));
                    for &t in Theme::all() {
                        if ui
                            .selectable_label(self.current_theme == t, t.label_localized(self.i18n.lang))
                            .clicked()
                        {
                            self.current_theme = t;
                            self.theme_changed = true;
                            self.current_theme.apply(ui.ctx());
                            let _ = self.save_config();
                            ui.ctx().request_repaint();
                        }
                    }
                });
            });

            // 操作菜单
            ui.menu_button(self.i18n.t("menu_actions"), |ui| {
                if ui
                    .button(format!("{}    \u{2190}/\u{2191}", self.i18n.t("menu_prev")))
                    .clicked()
                {
                    ui.close_menu();
                    self.navigate(-1);
                }
                if ui
                    .button(format!("{}    \u{2192}/\u{2193}", self.i18n.t("menu_next")))
                    .clicked()
                {
                    ui.close_menu();
                    self.navigate(1);
                }
                ui.separator();
                if ui.button(format!("{}    1", self.i18n.t("menu_copy_left"))).clicked() {
                    ui.close_menu();
                    self.copy_image(0);
                }
                if ui.button(format!("{}    2", self.i18n.t("menu_copy_right"))).clicked() {
                    ui.close_menu();
                    if self.two_column {
                        self.copy_image(1);
                    }
                }
                ui.separator();
                if ui
                    .button(format!("{}    Del", self.i18n.t("action_delete_image")))
                    .clicked()
                {
                    ui.close_menu();
                    self.request_delete();
                }
                if ui.button(self.i18n.t("ctx_rotate_cw")).clicked() {
                    ui.close_menu();
                    self.rotate_cw();
                }
                if ui.button(self.i18n.t("ctx_rotate_ccw")).clicked() {
                    ui.close_menu();
                    self.rotate_ccw();
                }
                ui.separator();
                if ui.button(self.i18n.t("menu_register_assoc")).clicked() {
                    ui.close_menu();
                    self.register_file_association();
                }
                if ui.button(self.i18n.t("menu_unregister_assoc")).clicked() {
                    ui.close_menu();
                    self.unregister_file_association();
                }
                ui.separator();
                // 缩放步长
                ui.horizontal(|ui| {
                    ui.label(self.i18n.t("settings_zoom_step"));
                    let response = ui.add(
                        egui::DragValue::new(&mut self.zoom_step_percent)
                            .range(1.0..=100.0)
                            .speed(0.1),
                    );
                    if response.changed() {
                        let _ = self.save_config();
                    }
                });
            });

            // 快捷键菜单
            ui.menu_button(self.i18n.t("menu_hotkeys"), |ui| {
                if ui.button(self.i18n.t("menu_shortcuts")).clicked() {
                    ui.close_menu();
                    self.show_shortcuts_window = true;
                }
            });

            // 帮助菜单
            ui.menu_button(self.i18n.t("menu_help"), |ui| {
                if ui.button(self.i18n.t("menu_about")).clicked() {
                    ui.close_menu();
                    self.show_about_window = true;
                }
            });
        });
    }

    /// 显示状态栏
    pub fn show_status(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if !self.status_message.is_empty() {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Jinn Image Viewer");
                });
                return;
            }
            if self.images.is_empty() {
                ui.label(self.i18n.t("status_no_images"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Jinn Image Viewer");
                });
                return;
            }
            let idx = self.current_index;
            if idx < self.images.len() {
                // 左侧：文件名（前30字符）+ 序号 + 缩放 + 分辨率 + 大小
                let name = &self.images[idx].name;
                let short_name = if name.len() > 30 {
                    format!("{}...", &name[..30])
                } else {
                    name.clone()
                };

                let scale_pct = (self.scale_factor * 100.0) as u32;

                // 分辨率和文件大小缓存：仅切换图片时读取一次
                if idx != self.cached_file_size_index {
                    self.cached_file_size_index = idx;
                    let path = &self.images[idx].path;
                    self.cached_resolution = self.texture_manager.get_size_info(path);
                    self.cached_file_size = std::fs::metadata(path)
                        .map(|m| format_file_size(m.len()))
                        .unwrap_or_default();
                }
                let res_info = &self.cached_resolution;
                let file_size = &self.cached_file_size;

                let mut left_text = format!(
                    "{}  |  {}/{}  |  {}%",
                    short_name,
                    idx + 1,
                    self.images.len(),
                    scale_pct,
                );

                if !res_info.is_empty() {
                    left_text.push_str(&format!("  |  {}", res_info));
                }
                if !file_size.is_empty() {
                    left_text.push_str(&format!("  |  {}", file_size));
                }

                if self.two_column && idx + 1 < self.images.len() {
                    let right_name = &self.images[idx + 1].name;
                    let short_right = if right_name.len() > 30 {
                        format!("{}...", &right_name[..30])
                    } else {
                        right_name.clone()
                    };
                    left_text.push_str(&format!("  |  {}: {}", self.i18n.t("status_right"), short_right));
                }

                ui.label(&left_text);

                // 右侧：程序名称
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("Jinn Image Viewer");
                });
            }
        });
    }

    /// 显示快捷键配置窗口内容
    pub fn show_shortcuts_window_content(&mut self, ui: &mut egui::Ui, request_close: &mut bool) {
        ui.heading(self.i18n.t("menu_hotkeys"));
        ui.separator();

        let key_options = all_key_options();

        egui::ScrollArea::vertical().show(ui, |ui| {
            let total = self.shortcuts.entries.len();
            for i in 0..total {
                let action_label = self.i18n.t(self.shortcuts.entries[i].action.i18n_key()).to_string();
                ui.horizontal(|ui| {
                    ui.label(&action_label);
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
                                        self.shortcut_conflicts_dirty = true;
                                    }
                                }
                            });

                        let mut alt = self.shortcuts.entries[i].modifiers.alt;
                        let mut shift = self.shortcuts.entries[i].modifiers.shift;
                        let mut ctrl = self.shortcuts.entries[i].modifiers.command;

                        let old_alt = alt;
                        let old_shift = shift;
                        let old_ctrl = ctrl;

                        ui.push_id(format!("alt_{}", i), |ui| {
                            ui.checkbox(&mut alt, "Alt");
                        });
                        ui.push_id(format!("shift_{}", i), |ui| {
                            ui.checkbox(&mut shift, "Shift");
                        });
                        ui.push_id(format!("ctrl_{}", i), |ui| {
                            ui.checkbox(&mut ctrl, "Ctrl");
                        });

                        if alt != old_alt || shift != old_shift || ctrl != old_ctrl {
                            self.shortcut_conflicts_dirty = true;
                        }

                        let mut mods = egui::Modifiers::NONE;
                        if alt {
                            mods.alt = true;
                        }
                        if shift {
                            mods.shift = true;
                        }
                        if ctrl {
                            mods.command = true;
                        }
                        self.shortcuts.entries[i].modifiers = mods;
                    });
                });
            }
            ui.add_space(8.0);

            // 快捷键冲突检测（仅修改时重新计算）
            if self.shortcut_conflicts_dirty {
                self.shortcut_conflicts_dirty = false;
                self.shortcut_conflicts.clear();
                for i in 0..self.shortcuts.entries.len() {
                    for j in (i + 1)..self.shortcuts.entries.len() {
                        let a = &self.shortcuts.entries[i];
                        let b = &self.shortcuts.entries[j];
                        if a.key == b.key && a.modifiers == b.modifiers {
                            let name_a = self.i18n.t(a.action.i18n_key());
                            let name_b = self.i18n.t(b.action.i18n_key());
                            self.shortcut_conflicts.push(format!("{} / {}", name_a, name_b));
                        }
                    }
                }
            }
            if !self.shortcut_conflicts.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(0xFF, 0x66, 0x66),
                    format!(
                        "{}: {}",
                        self.i18n.t("settings_conflict"),
                        self.shortcut_conflicts.join(", ")
                    ),
                );
            }

            ui.horizontal(|ui| {
                if ui.button(self.i18n.t("settings_save")).clicked() {
                    if self.shortcut_conflicts.is_empty() {
                        if self.save_config() {
                            self.set_status(self.i18n.t("settings_saved_msg").to_string());
                        }
                    } else {
                        self.set_status(self.i18n.t("settings_conflict_warn").to_string());
                    }
                }
                if ui.button(self.i18n.t("settings_reset")).clicked() {
                    self.shortcuts = ShortcutConfig::defaults();
                    let _ = self.save_config();
                    self.set_status(self.i18n.t("settings_reset_msg").to_string());
                }
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(self.i18n.t("settings_tip"))
                    .color(egui::Color32::from_gray(120))
                    .small(),
            );
            ui.vertical_centered(|ui| {
                if ui.button(self.i18n.t("settings_close")).clicked() {
                    *request_close = true;
                }
            });
        });
    }

    /// 显示关于窗口
    pub fn show_about_window_content(&mut self, ctx: &egui::Context) {
        let title = self.i18n.t("about_title").to_string();
        let author = self.i18n.t("about_author");
        let github_label = self.i18n.t("about_github");
        let error_prefix = self.i18n.t("about_github_error").to_string();
        let mut error_msg: Option<String> = None;

        egui::Window::new(&title)
            .open(&mut self.show_about_window)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(format!("Jinn Image Viewer v{}", crate::BUILD_DATE));
                    ui.label(author);
                    if ui.button(github_label).clicked() {
                        if let Err(e) = open::that("https://github.com/yezijinn/jinn_imageviewer") {
                            error_msg = Some(format!("{}: {}", error_prefix, e));
                        }
                    }
                });
            });

        if let Some(msg) = error_msg {
            self.set_status(msg);
        }
    }

    /// 显示删除确认对话框
    pub fn show_delete_confirm_dialog(&mut self, ctx: &egui::Context) {
        let title = self.i18n.t("delete_confirm_title").to_string();
        let msg = self.i18n.t("delete_confirm_msg").to_string();
        let yes = self.i18n.t("delete_confirm_yes").to_string();
        let no = self.i18n.t("delete_confirm_no").to_string();

        let mut should_delete = false;
        let mut should_close = false;

        egui::Window::new(&title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    if self.current_index < self.images.len() {
                        ui.label(format!("{}\n\n{}", msg, self.images[self.current_index].name));
                    } else {
                        ui.label(&msg);
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button(&yes).clicked() {
                            should_delete = true;
                            should_close = true;
                        }
                        if ui.button(&no).clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_delete {
            self.delete_current();
        }
        if should_close {
            self.show_delete_confirm_dialog = false;
        }
    }

    /// 显示复制错误对话框
    pub fn show_copy_error_dialog_ui(&mut self, ctx: &egui::Context) {
        let title = self.i18n.t("dialog_notice").to_string();
        let ok_text = self.i18n.t("dialog_ok").to_string();

        egui::Window::new(&title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(&self.copy_error_msg);
                    ui.add_space(10.0);
                    if ui.button(&ok_text).clicked() {
                        self.show_copy_error_dialog = false;
                    }
                });
            });
    }

    pub fn show_large_image_dialog_ui(&mut self, ctx: &egui::Context) {
        let Some(path) = self.large_image_path.clone() else {
            self.show_large_image_dialog = false;
            return;
        };
        let title = self.i18n.t("large_image_title").to_string();
        let yes = self.i18n.t("large_image_load").to_string();
        let no = self.i18n.t("large_image_cancel").to_string();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let warning = match assess_image_load(&path) {
            ImageLoadAssessment::RequiresConfirmation { file_bytes, pixels } => format!(
                "{}\n{}: {}\n{}: {}",
                self.i18n.t("large_image_warning"),
                self.i18n.t("large_image_file_size"),
                format_file_size(file_bytes),
                self.i18n.t("large_image_pixels"),
                pixels,
            ),
            _ => self.i18n.t("large_image_unavailable").to_string(),
        };
        let mut load = false;
        let mut close = false;
        egui::Window::new(&title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(name.as_ref());
                    ui.add_space(8.0);
                    ui.label(&warning);
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button(&yes).clicked() {
                            load = true;
                            close = true;
                        }
                        if ui.button(&no).clicked() {
                            close = true;
                        }
                    });
                });
            });
        if load {
            self.approved_large_image_path = Some(path);
        }
        if close {
            self.show_large_image_dialog = false;
            self.large_image_path = None;
        }
    }

    /// 显示中央图片区域
    pub fn show_image_area(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if self.images.is_empty() {
            let rect = ui.available_rect_before_wrap();
            let painter = ui.painter();
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                self.i18n.t("image_open_hint"),
                egui::FontId::proportional(18.0),
                egui::Color32::from_gray(160),
            );
            let response = ui.interact(rect, ui.next_auto_id(), egui::Sense::click());
            if response.double_clicked() {
                self.open_folder();
            }
            return;
        }

        // 末尾过渡页
        if self.at_end_page {
            let rect = ui.available_rect_before_wrap();
            let painter = ui.painter();
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                self.i18n.t("end_page_hint"),
                egui::FontId::proportional(20.0),
                egui::Color32::from_gray(160),
            );
            // 过渡页也支持双击打开文件夹和右键适应窗口
            let response = ui.interact(rect, ui.next_auto_id(), egui::Sense::click());
            if response.double_clicked() {
                self.open_folder();
            }
            return;
        }

        let idx = self.current_index;
        if idx >= self.images.len() {
            return;
        }

        // 有浮动窗口打开时不处理滚轮缩放（让窗口的 vscroll 独占滚轮事件）
        let no_window_open = !self.show_image_info_window
            && !self.show_shortcuts_window
            && !self.show_about_window
            && !self.show_delete_confirm_dialog
            && !self.show_copy_error_dialog;
        if no_window_open {
            self.handle_zoom(ctx);
        }

        // 文件存在性检测：外部删除时自动移除
        if !self.verify_current_file() {
            return; // 文件已移除，下一帧重新渲染
        }

        if self.two_column {
            self.show_dual_images(ui);
        } else {
            let is_gif = idx < self.images.len() && crate::image::GifAnimator::is_gif(&self.images[idx].path);
            if is_gif {
                // GIF: show_single_image 已处理全部控件和右键菜单
                // 跳过下面的 interact 区域，避免覆盖按钮点击
                self.show_single_image(ui, ctx, idx);
                self.texture_manager.cleanup(&self.images, self.current_index);
                return;
            }
            self.show_single_image(ui, ctx, idx);
        }

        let response = ui.interact(ui.max_rect(), ui.id().with("image_area_interact"), egui::Sense::click());
        if response.double_clicked() {
            self.open_folder();
        }

        // 右键上下文菜单
        response.context_menu(|ui| {
            if ui
                .button(format!("{}    \u{2190}/\u{2191}", self.i18n.t("menu_prev")))
                .clicked()
            {
                self.navigate(-1);
                ui.close_menu();
            }
            if ui
                .button(format!("{}    \u{2192}/\u{2193}", self.i18n.t("menu_next")))
                .clicked()
            {
                self.navigate(1);
                ui.close_menu();
            }
            ui.separator();
            if ui.button(self.i18n.t("ctx_fit_window")).clicked() {
                self.fit_to_window = !self.fit_to_window;
                let _ = self.save_config();
                ui.close_menu();
            }
            ui.separator();
            if ui.button(self.i18n.t("ctx_rotate_cw")).clicked() {
                self.rotate_cw();
                ui.close_menu();
            }
            if ui.button(self.i18n.t("ctx_rotate_ccw")).clicked() {
                self.rotate_ccw();
                ui.close_menu();
            }
            ui.separator();
            if ui.button(self.i18n.t("ctx_copy_image")).clicked() {
                self.copy_image(0);
                ui.close_menu();
            }
            if ui.button(self.i18n.t("ctx_delete")).clicked() {
                self.request_delete();
                ui.close_menu();
            }
            if self.two_column && ui.button(format!("{}    2", self.i18n.t("menu_copy_right"))).clicked() {
                self.copy_image(1);
                ui.close_menu();
            }
            ui.separator();
            if ui.button(self.i18n.t("action_show_exif")).clicked() {
                self.show_image_info_window = true;
                ui.close_menu();
            }
            if ui.button(self.i18n.t("action_toggle_slideshow")).clicked() {
                self.slideshow_active = !self.slideshow_active;
                if self.slideshow_active {
                    self.slideshow_timer = std::time::Instant::now();
                }
                let _ = self.save_config();
                ui.close_menu();
            }
            if ui.button(self.i18n.t("action_fullscreen")).clicked() {
                self.toggle_fullscreen(ui.ctx());
                ui.close_menu();
            }
            ui.separator();
            if ui.button(self.i18n.t("ctx_open_folder")).clicked() {
                self.open_folder();
                ui.close_menu();
            }
            if ui.button(self.i18n.t("ctx_refresh")).clicked() {
                self.refresh_folder();
                ui.close_menu();
            }
            ui.separator();
            if ui.button(self.i18n.t("ctx_save_rotation")).clicked() {
                self.save_rotation();
                ui.close_menu();
            }
            if ui.button(self.i18n.t("menu_print")).clicked() {
                self.print_current();
                ui.close_menu();
            }
        });

        self.texture_manager.cleanup(&self.images, self.current_index);
    }

    /// 显示单张图片
    fn show_single_image(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, idx: usize) {
        let path = self.images[idx].path.clone();
        let fit_to_window = self.fit_to_window;
        let scale_factor = self.scale_factor;

        // GIF 动画处理
        if crate::image::GifAnimator::is_gif(&path) {
            // 加载或复用 GIF 动画器
            let need_load = match &self.gif_animator {
                Some(animator) => !animator.matches(&path),
                None => true,
            };
            if need_load {
                self.gif_animator = crate::image::GifAnimator::load(&path);
            }

            if self.gif_animator.is_some() {
                // 渲染 GIF 帧
                let animator = self.gif_animator.as_mut().unwrap();
                let tex = animator.current_texture(ctx);
                let tex_size = tex.size_vec2();
                let tex_id = tex.id();

                // 预留底部控制栏空间
                let control_height = 30.0;
                let avail = ui.available_size() - egui::Vec2::new(0.0, control_height);
                let draw_size = if fit_to_window {
                    fit_size(tex_size, avail)
                } else {
                    tex_size * scale_factor
                };

                // 图片区域
                let image_response = ui
                    .allocate_ui_with_layout(
                        avail,
                        egui::Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| {
                            ui.image(egui::load::SizedTexture::new(tex_id, draw_size));
                        },
                    )
                    .response;

                // 右键上下文菜单（在图片区域上）
                image_response.context_menu(|ui| {
                    if ui
                        .button(format!("{}    \u{2190}/\u{2191}", self.i18n.t("menu_prev")))
                        .clicked()
                    {
                        self.navigate(-1);
                        ui.close_menu();
                    }
                    if ui
                        .button(format!("{}    \u{2192}/\u{2193}", self.i18n.t("menu_next")))
                        .clicked()
                    {
                        self.navigate(1);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(self.i18n.t("ctx_fit_window")).clicked() {
                        self.fit_to_window = !self.fit_to_window;
                        let _ = self.save_config();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(self.i18n.t("ctx_rotate_cw")).clicked() {
                        self.rotate_cw();
                        ui.close_menu();
                    }
                    if ui.button(self.i18n.t("ctx_rotate_ccw")).clicked() {
                        self.rotate_ccw();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(self.i18n.t("ctx_copy_image")).clicked() {
                        self.copy_image(0);
                        ui.close_menu();
                    }
                    if ui.button(self.i18n.t("ctx_delete")).clicked() {
                        self.request_delete();
                        ui.close_menu();
                    }
                    if ui.button(self.i18n.t("action_show_exif")).clicked() {
                        self.show_image_info_window = true;
                        ui.close_menu();
                    }
                    if ui.button(self.i18n.t("action_toggle_slideshow")).clicked() {
                        self.slideshow_active = !self.slideshow_active;
                        if self.slideshow_active {
                            self.slideshow_timer = std::time::Instant::now();
                        }
                        let _ = self.save_config();
                        ui.close_menu();
                    }
                    if ui.button(self.i18n.t("action_fullscreen")).clicked() {
                        self.toggle_fullscreen(ui.ctx());
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(self.i18n.t("ctx_open_folder")).clicked() {
                        self.open_folder();
                        ui.close_menu();
                    }
                    if ui.button(self.i18n.t("ctx_refresh")).clicked() {
                        self.refresh_folder();
                        ui.close_menu();
                    }
                });

                // 底部 GIF 控制栏
                let animator = self.gif_animator.as_ref().unwrap();
                let frame_num = animator.current_frame_number();
                let total = animator.total_frames();

                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    // ← 上一帧（同时触发暂停）
                    if ui.button("  ←  ").clicked() {
                        if let Some(a) = &mut self.gif_animator {
                            a.prev_frame();
                        }
                    }
                    // ⏸ 暂停
                    if ui.button("  ⏸  ").clicked() {
                        if let Some(a) = &mut self.gif_animator {
                            a.pause();
                        }
                    }
                    // ▶ 播放（恢复动画）
                    if ui.button("  ▶  ").clicked() {
                        if let Some(a) = &mut self.gif_animator {
                            a.play();
                        }
                    }
                    // → 下一帧（同时触发暂停）
                    if ui.button("  →  ").clicked() {
                        if let Some(a) = &mut self.gif_animator {
                            a.next_frame();
                        }
                    }
                    ui.add_space(12.0);
                    // 帧计数
                    ui.label(format!("{}/{}", frame_num, total));
                });

                // 非暂停时按帧延时请求重绘
                if let Some(animator) = &self.gif_animator {
                    if !animator.paused {
                        let delay = animator.current_frame_delay();
                        ctx.request_repaint_after(delay);
                    }
                }
                return;
            }
        } else {
            // 非 GIF 时清除动画器
            self.gif_animator = None;
        }

        // 已标记损坏的图片，直接显示占位符，避免重复解码
        if self.images[idx].load_failed {
            let rect = ui.available_rect_before_wrap();
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("⚠ {}", self.i18n.t("status_image_corrupted")),
                egui::FontId::proportional(16.0),
                egui::Color32::RED,
            );
            return;
        }

        let rotation = self.images[idx].manual_rotation;
        let target_width = if fit_to_window {
            ui.available_width().ceil().max(1.0) as u32
        } else {
            4096
        };

        let assessment = assess_image_load(&path);
        let is_approved = self.approved_large_image_path.as_ref() == Some(&path);
        if matches!(assessment, ImageLoadAssessment::Impossible { .. }) {
            let rect = ui.available_rect_before_wrap();
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                self.i18n.t("large_image_unavailable"),
                egui::FontId::proportional(16.0),
                egui::Color32::YELLOW,
            );
            return;
        }
        if matches!(assessment, ImageLoadAssessment::RequiresConfirmation { .. }) && !is_approved {
            if !self.show_large_image_dialog {
                self.large_image_path = Some(path.clone());
                self.show_large_image_dialog = true;
            }
            let rect = ui.available_rect_before_wrap();
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                self.i18n.t("large_image_waiting_confirmation"),
                egui::FontId::proportional(16.0),
                egui::Color32::YELLOW,
            );
            return;
        }
        let texture =
            match self
                .texture_manager
                .get_or_load_with_rotation_policy(ctx, &path, rotation, target_width, is_approved)
            {
                Some(t) => t,
                None => {
                    // 普通图片同步解码失败时明确显示错误
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        self.i18n.t("status_loading"),
                        egui::FontId::proportional(16.0),
                        egui::Color32::from_gray(140),
                    );
                    return;
                }
            };
        if is_approved {
            self.approved_large_image_path = None;
        }

        let tex_size = texture.size_vec2();
        let avail = ui.available_size();
        let draw_size = if fit_to_window {
            fit_size(tex_size, avail)
        } else {
            tex_size * scale_factor
        };

        if fit_to_window {
            ui.allocate_ui_with_layout(
                avail,
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.image(egui::load::SizedTexture::new(texture.id(), draw_size));
                },
            );
        } else {
            egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                ui.allocate_ui_with_layout(
                    ui.available_size(),
                    egui::Layout::centered_and_justified(egui::Direction::TopDown),
                    |ui| {
                        ui.image(egui::load::SizedTexture::new(texture.id(), draw_size));
                    },
                );
            });
        }
    }

    /// 显示双张图片
    fn show_dual_images(&mut self, ui: &mut egui::Ui) {
        let left_idx = self.current_index;
        let right_idx = self.current_index + 1;
        let fit_to_window = self.fit_to_window;
        let scale_factor = self.scale_factor;

        // 双列模式同样只读取已上传纹理，解码请求交给后台线程。
        let (left_path, left_rotation) = if left_idx < self.images.len() {
            (
                self.images[left_idx].path.clone(),
                self.images[left_idx].manual_rotation,
            )
        } else {
            (PathBuf::new(), 0)
        };

        ui.horizontal_top(|ui| {
            let total_w = ui.available_width();
            let half_w = (total_w / 2.0).max(1.0);
            let avail_h = ui.available_height();
            let half_size = egui::Vec2::new(half_w, avail_h);

            // 左图（已预检通过，直接用缓存的 path）
            if left_idx < self.images.len() {
                let left_gif = get_dual_gif_frame(&mut self.dual_left_gif_animator, &left_path, ui.ctx());
                if let Some((texture_id, tex_size, delay)) = left_gif {
                    let draw_size = if fit_to_window {
                        fit_size_to_max(tex_size, half_size)
                    } else {
                        tex_size * scale_factor
                    };
                    egui::ScrollArea::both()
                        .max_width(half_w)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.allocate_ui_with_layout(
                                ui.available_size(),
                                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                |ui| {
                                    ui.image(egui::load::SizedTexture::new(texture_id, draw_size));
                                },
                            );
                        });
                    ui.ctx().request_repaint_after(delay);
                } else if let Some(texture) = self.texture_manager.get_or_load_with_rotation(
                    ui.ctx(),
                    &left_path,
                    left_rotation,
                    if fit_to_window { half_w.ceil() as u32 } else { 4096 },
                ) {
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
                            ui.allocate_ui_with_layout(
                                ui.available_size(),
                                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                |ui| {
                                    ui.image(egui::load::SizedTexture::new(texture.id(), draw_size));
                                },
                            );
                        });
                }
            } else {
                ui.allocate_space(half_size);
            }

            // 右图
            if right_idx < self.images.len() {
                let right_path = self.images[right_idx].path.clone();
                let right_rotation = self.images[right_idx].manual_rotation;
                let right_gif = get_dual_gif_frame(&mut self.dual_right_gif_animator, &right_path, ui.ctx());
                if let Some((texture_id, tex_size, delay)) = right_gif {
                    let draw_size = if fit_to_window {
                        fit_size_to_max(tex_size, half_size)
                    } else {
                        tex_size * scale_factor
                    };
                    egui::ScrollArea::both()
                        .max_width(half_w)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.allocate_ui_with_layout(
                                ui.available_size(),
                                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                |ui| {
                                    ui.image(egui::load::SizedTexture::new(texture_id, draw_size));
                                },
                            );
                        });
                    ui.ctx().request_repaint_after(delay);
                } else if let Some(texture) = self.texture_manager.get_or_load_with_rotation(
                    ui.ctx(),
                    &right_path,
                    right_rotation,
                    if fit_to_window { half_w.ceil() as u32 } else { 4096 },
                ) {
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
                            ui.allocate_ui_with_layout(
                                ui.available_size(),
                                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                                |ui| {
                                    ui.image(egui::load::SizedTexture::new(texture.id(), draw_size));
                                },
                            );
                        });
                }
            } else {
                // 奇数张图时右侧显示末尾提示（只有1张图时不显示）
                let (rect, _) = ui.allocate_exact_size(half_size, egui::Sense::hover());
                if self.images.len() > 1 {
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        self.i18n.t("end_page_hint"),
                        egui::FontId::proportional(16.0),
                        egui::Color32::from_gray(140),
                    );
                }
            }
        });
    }

    /// 显示缩略图轨道（分页模式：根据窗口宽度确定每页缩略图数量，翻页切换）
    pub fn show_thumbnail_track(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let thumb_size = 64.0_f32;
        let spacing = 4.0_f32;
        let item_width = thumb_size + spacing;
        let total_count = self.images.len();
        if total_count == 0 {
            return;
        }
        let current = self.current_index;

        // 根据当前可用宽度确定每页缩略图数量
        let avail_width = ui.available_width();
        let page_size = (avail_width / item_width).floor() as usize;
        let page_size = page_size.max(1);

        // 计算当前页的起止索引
        let current_page = current / page_size;
        let page_start = current_page * page_size;
        let page_end = (page_start + page_size).min(total_count);

        // 分配高度空间（宽度无需固定，底层 panel 会裁剪）
        let page_width = (page_end - page_start) as f32 * item_width;
        let (base_rect, _) =
            ui.allocate_exact_size(egui::Vec2::new(page_width, thumb_size + 4.0), egui::Sense::hover());

        let mut loads_this_frame = 0;
        const MAX_THUMB_LOADS_PER_FRAME: usize = 2;

        for i in page_start..page_end {
            let x = base_rect.left() + (i - page_start) as f32 * item_width;
            let rect =
                egui::Rect::from_min_size(egui::Pos2::new(x, base_rect.top() + 2.0), egui::Vec2::splat(thumb_size));

            let response = ui.interact(rect, ui.id().with(("thumb", i)), egui::Sense::click());
            let is_current = i == current;

            // 高亮当前选中的缩略图
            if is_current {
                let colors = self.current_theme.colors();
                ui.painter()
                    .rect_filled(rect.expand(2.0), egui::CornerRadius::same(4), colors.accent);
            }

            // 加载并渲染缩略图（每帧最多加载2张新的）
            let path = &self.images[i].path;
            let tex = if self.thumbnail_manager.is_cached(path) {
                self.thumbnail_manager.get_cached(path)
            } else if loads_this_frame < MAX_THUMB_LOADS_PER_FRAME {
                loads_this_frame += 1;
                self.thumbnail_manager.get_or_load(ui.ctx(), path)
            } else {
                None
            };

            if let Some(tex) = tex {
                let tex_size = tex.size_vec2();
                let scale = (thumb_size / tex_size.x).min(thumb_size / tex_size.y);
                let draw_size = tex_size * scale;
                let offset = (egui::Vec2::splat(thumb_size) - draw_size) * 0.5;
                let img_rect = egui::Rect::from_min_size(rect.min + offset, draw_size);
                ui.painter().image(
                    tex.id(),
                    img_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            } else {
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(2), egui::Color32::from_gray(40));
            }

            if response.clicked() {
                self.current_index = i;
                self.scale_factor = 1.0;
                self.status_message.clear();
                self.at_end_page = false;
            }
        }

        if loads_this_frame >= MAX_THUMB_LOADS_PER_FRAME {
            ctx.request_repaint();
        }
    }

    /// 图片信息窗口（文件信息 + 图像属性 + EXIF 元数据）
    pub fn show_image_info_window_content(&mut self, ctx: &egui::Context) {
        use egui::*;
        let mut open = self.show_image_info_window;
        Window::new(self.i18n.t("action_show_exif"))
            .open(&mut open)
            .resizable(true)
            .default_width(400.0)
            .default_height(480.0)
            .vscroll(true)
            .show(ctx, |ui| {
                if self.image_info_cache.is_empty() {
                    ui.label(
                        RichText::new(match self.i18n.lang {
                            Language::Chinese => "无法读取图片信息",
                            _ => "Cannot read image info",
                        })
                        .color(Color32::from_gray(140)),
                    );
                } else {
                    for (section_title, fields) in &self.image_info_cache {
                        ui.label(
                            RichText::new(section_title.clone())
                                .size(13.0)
                                .color(Color32::from_rgb(220, 190, 120))
                                .strong(),
                        );
                        ui.separator();
                        for (key, value) in fields {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("  {}: ", key))
                                        .color(Color32::from_rgb(180, 200, 255))
                                        .strong(),
                                );
                                // 让值可以选中复制
                                ui.add(
                                    Label::new(RichText::new(value).color(Color32::from_gray(210)))
                                        .sense(Sense::click()),
                                );
                            });
                        }
                        ui.add_space(8.0);
                    }
                }
            });
        if !open {
            self.show_image_info_window = false;
        }
    }
}
