use eframe::egui;

use crate::i18n::Language;

/// 主题枚举
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Theme {
    ClassicDark,
    DeepSpace,
    CyberPurple,
    AuroraGreen,
    MinimalGray,
}

impl Theme {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ClassicDark => "Black",
            Self::DeepSpace => "Blue",
            Self::CyberPurple => "Purple",
            Self::AuroraGreen => "Green",
            Self::MinimalGray => "Gray",
        }
    }

    pub fn label_localized(&self, lang: Language) -> &'static str {
        match lang {
            Language::Chinese => match self {
                Self::ClassicDark => "黑",
                Self::DeepSpace => "蓝",
                Self::CyberPurple => "紫",
                Self::AuroraGreen => "绿",
                Self::MinimalGray => "灰",
            },
            Language::English => self.label(),
        }
    }

    pub fn all() -> &'static [Theme] {
        &[
            Theme::ClassicDark,
            Theme::DeepSpace,
            Theme::CyberPurple,
            Theme::AuroraGreen,
            Theme::MinimalGray,
        ]
    }
}

/// 主题色彩方案
pub struct ThemeColors {
    pub bg_primary: egui::Color32,
    pub bg_secondary: egui::Color32,
    pub bg_panel: egui::Color32,
    pub accent: egui::Color32,
    pub accent_dim: egui::Color32,
    pub text_primary: egui::Color32,
    pub text_secondary: egui::Color32,
    pub border: egui::Color32,
    pub shadow_color: egui::Color32,
    pub corner_radius: f32,
    pub shadow_offset: f32,
}

impl Theme {
    pub fn colors(&self) -> ThemeColors {
        match self {
            Theme::ClassicDark => ThemeColors {
                bg_primary: egui::Color32::from_rgb(0x1E, 0x1E, 0x1E),
                bg_secondary: egui::Color32::from_rgb(0x2D, 0x2D, 0x2D),
                bg_panel: egui::Color32::from_rgb(0x25, 0x25, 0x25),
                accent: egui::Color32::from_rgb(0x00, 0x7A, 0xCC),
                accent_dim: egui::Color32::from_rgb(0x00, 0x4E, 0x82),
                text_primary: egui::Color32::from_rgb(0xE0, 0xE0, 0xE0),
                text_secondary: egui::Color32::from_rgb(0xA0, 0xA0, 0xA0),
                border: egui::Color32::from_rgb(0x3A, 0x3A, 0x3A),
                shadow_color: egui::Color32::from_black_alpha(80),
                corner_radius: 6.0,
                shadow_offset: 4.0,
            },
            Theme::DeepSpace => ThemeColors {
                bg_primary: egui::Color32::from_rgb(0x0A, 0x19, 0x2F),
                bg_secondary: egui::Color32::from_rgb(0x11, 0x2A, 0x46),
                bg_panel: egui::Color32::from_rgb(0x0F, 0x20, 0x3A),
                accent: egui::Color32::from_rgb(0x64, 0xFF, 0xDA),
                accent_dim: egui::Color32::from_rgb(0x38, 0xB0, 0x98),
                text_primary: egui::Color32::from_rgb(0xCD, 0xD6, 0xF4),
                text_secondary: egui::Color32::from_rgb(0x7F, 0x9C, 0xB8),
                border: egui::Color32::from_rgb(0x1D, 0x3A, 0x5C),
                shadow_color: egui::Color32::from_black_alpha(120),
                corner_radius: 8.0,
                shadow_offset: 6.0,
            },
            Theme::CyberPurple => ThemeColors {
                bg_primary: egui::Color32::from_rgb(0x13, 0x0F, 0x25),
                bg_secondary: egui::Color32::from_rgb(0x1E, 0x17, 0x3B),
                bg_panel: egui::Color32::from_rgb(0x19, 0x13, 0x30),
                accent: egui::Color32::from_rgb(0xC7, 0x7D, 0xFF),
                accent_dim: egui::Color32::from_rgb(0x7B, 0x2C, 0xBF),
                text_primary: egui::Color32::from_rgb(0xE8, 0xDE, 0xF8),
                text_secondary: egui::Color32::from_rgb(0x9D, 0x8E, 0xB8),
                border: egui::Color32::from_rgb(0x2E, 0x22, 0x50),
                shadow_color: egui::Color32::from_rgba_premultiplied(0x7B, 0x2C, 0xBF, 40),
                corner_radius: 10.0,
                shadow_offset: 6.0,
            },
            Theme::AuroraGreen => ThemeColors {
                bg_primary: egui::Color32::from_rgb(0x0D, 0x1B, 0x1E),
                bg_secondary: egui::Color32::from_rgb(0x14, 0x2B, 0x2E),
                bg_panel: egui::Color32::from_rgb(0x10, 0x23, 0x26),
                accent: egui::Color32::from_rgb(0x00, 0xFF, 0xA3),
                accent_dim: egui::Color32::from_rgb(0x00, 0xB0, 0x72),
                text_primary: egui::Color32::from_rgb(0xDA, 0xF5, 0xEA),
                text_secondary: egui::Color32::from_rgb(0x7A, 0xB8, 0x9D),
                border: egui::Color32::from_rgb(0x1A, 0x3A, 0x3E),
                shadow_color: egui::Color32::from_rgba_premultiplied(0x00, 0xFF, 0xA3, 25),
                corner_radius: 8.0,
                shadow_offset: 5.0,
            },
            Theme::MinimalGray => ThemeColors {
                bg_primary: egui::Color32::from_rgb(0x20, 0x22, 0x24),
                bg_secondary: egui::Color32::from_rgb(0x2C, 0x2E, 0x30),
                bg_panel: egui::Color32::from_rgb(0x26, 0x28, 0x2A),
                accent: egui::Color32::from_rgb(0xE0, 0xE0, 0xE0),
                accent_dim: egui::Color32::from_rgb(0x80, 0x80, 0x80),
                text_primary: egui::Color32::from_rgb(0xF0, 0xF0, 0xF0),
                text_secondary: egui::Color32::from_rgb(0x90, 0x90, 0x90),
                border: egui::Color32::from_rgb(0x40, 0x42, 0x44),
                shadow_color: egui::Color32::from_black_alpha(60),
                corner_radius: 4.0,
                shadow_offset: 3.0,
            },
        }
    }

    /// 将主题应用到 egui 样式
    pub fn apply(&self, ctx: &egui::Context) {
        let colors = self.colors();
        let mut style = (*ctx.style()).clone();

        style.visuals.dark_mode = true;
        style.visuals.widgets.noninteractive.bg_fill = colors.bg_primary;
        style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, colors.text_primary);
        style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(colors.corner_radius as u8);

        style.visuals.widgets.inactive.bg_fill = colors.bg_secondary;
        style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, colors.text_secondary);
        style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(colors.corner_radius as u8);

        style.visuals.widgets.hovered.bg_fill = colors.accent_dim;
        style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, colors.text_primary);
        style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(colors.corner_radius as u8);

        style.visuals.widgets.active.bg_fill = colors.accent;
        style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, colors.bg_primary);
        style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(colors.corner_radius as u8);

        style.visuals.panel_fill = colors.bg_panel;
        style.visuals.window_fill = colors.bg_secondary;
        style.visuals.window_stroke = egui::Stroke::new(1.0_f32, colors.border);
        style.visuals.window_shadow = egui::Shadow {
            offset: [0, colors.shadow_offset as i8],
            blur: (colors.shadow_offset * 2.0) as u8,
            spread: 0,
            color: colors.shadow_color,
        };
        style.visuals.popup_shadow = egui::Shadow {
            offset: [0, (colors.shadow_offset * 0.5) as i8],
            blur: colors.shadow_offset as u8,
            spread: 0,
            color: colors.shadow_color,
        };

        style.visuals.selection.stroke.color = colors.accent;
        style.visuals.selection.bg_fill = colors.accent_dim;
        style.visuals.hyperlink_color = colors.accent;
        style.visuals.faint_bg_color = colors.bg_primary;
        style.visuals.extreme_bg_color = colors.bg_secondary;

        // 间距优化
        style.spacing.item_spacing = egui::Vec2::new(8.0, 5.0);
        style.spacing.window_margin = egui::Margin::same(12);
        style.spacing.button_padding = egui::Vec2::new(8.0, 4.0);

        ctx.set_style(style);
    }
}
