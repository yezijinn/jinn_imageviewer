use eframe::egui;

/// 快捷键动作枚举
#[derive(Clone, PartialEq)]
pub enum ShortcutAction {
    OpenFolder,
    PrevImage,
    NextImage,
    DeleteImage,
    ToggleDual,
    CopyLeft,
    CopyRight,
    ToggleFit,
    About,
    ToggleFullscreen,
    RotateCW,
    RotateCCW,
    ShowEXIF,
    ToggleSlideshow,
}

impl ShortcutAction {
    /// 获取动作的 i18n key
    pub fn i18n_key(&self) -> &'static str {
        match self {
            Self::OpenFolder => "action_open_folder",
            Self::PrevImage => "action_prev_image",
            Self::NextImage => "action_next_image",
            Self::DeleteImage => "action_delete_image",
            Self::ToggleDual => "action_toggle_dual",
            Self::CopyLeft => "action_copy_left",
            Self::CopyRight => "action_copy_right",
            Self::ToggleFit => "action_toggle_fit",
            Self::About => "action_about",
            Self::ToggleFullscreen => "action_fullscreen",
            Self::RotateCW => "action_rotate_cw",
            Self::RotateCCW => "action_rotate_ccw",
            Self::ShowEXIF => "action_show_exif",
            Self::ToggleSlideshow => "action_toggle_slideshow",
        }
    }

    pub fn from_i18n_key(key: &str) -> Option<Self> {
        match key {
            "action_open_folder" => Some(Self::OpenFolder),
            "action_prev_image" => Some(Self::PrevImage),
            "action_next_image" => Some(Self::NextImage),
            "action_delete_image" => Some(Self::DeleteImage),
            "action_toggle_dual" => Some(Self::ToggleDual),
            "action_copy_left" => Some(Self::CopyLeft),
            "action_copy_right" => Some(Self::CopyRight),
            "action_toggle_fit" => Some(Self::ToggleFit),
            "action_about" => Some(Self::About),
            "action_fullscreen" => Some(Self::ToggleFullscreen),
            "action_rotate_cw" => Some(Self::RotateCW),
            "action_rotate_ccw" => Some(Self::RotateCCW),
            "action_show_exif" => Some(Self::ShowEXIF),
            "action_toggle_slideshow" => Some(Self::ToggleSlideshow),
            _ => None,
        }
    }
}

/// 快捷键条目结构体
#[derive(Clone)]
pub struct ShortcutEntry {
    pub action: ShortcutAction,
    pub key: egui::Key,
    pub modifiers: egui::Modifiers,
}

impl ShortcutEntry {
    pub fn new(action: ShortcutAction, key: egui::Key, modifiers: egui::Modifiers) -> Self {
        Self { action, key, modifiers }
    }
}

/// 快捷键配置结构体
pub struct ShortcutConfig {
    pub entries: Vec<ShortcutEntry>,
}

impl ShortcutConfig {
    /// 获取默认快捷键配置
    pub fn defaults() -> Self {
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
                ShortcutEntry::new(ShortcutAction::About, F1, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::ToggleFullscreen, F11, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::RotateCW, R, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::RotateCCW, L, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::ShowEXIF, F3, egui::Modifiers::NONE),
                ShortcutEntry::new(ShortcutAction::ToggleSlideshow, F5, egui::Modifiers::NONE),
            ],
        }
    }
}

/// 获取按键的显示名称
pub fn key_label(key: egui::Key) -> &'static str {
    use egui::Key::*;
    match key {
        A => "A",
        B => "B",
        C => "C",
        D => "D",
        E => "E",
        F => "F",
        G => "G",
        H => "H",
        I => "I",
        J => "J",
        K => "K",
        L => "L",
        M => "M",
        N => "N",
        O => "O",
        P => "P",
        Q => "Q",
        R => "R",
        S => "S",
        T => "T",
        U => "U",
        V => "V",
        W => "W",
        X => "X",
        Y => "Y",
        Z => "Z",
        Num0 => "0",
        Num1 => "1",
        Num2 => "2",
        Num3 => "3",
        Num4 => "4",
        Num5 => "5",
        Num6 => "6",
        Num7 => "7",
        Num8 => "8",
        Num9 => "9",
        F1 => "F1",
        F2 => "F2",
        F3 => "F3",
        F4 => "F4",
        F5 => "F5",
        F6 => "F6",
        F7 => "F7",
        F8 => "F8",
        F9 => "F9",
        F10 => "F10",
        F11 => "F11",
        F12 => "F12",
        ArrowLeft => "\u{2190}",
        ArrowRight => "\u{2192}",
        ArrowUp => "\u{2191}",
        ArrowDown => "\u{2193}",
        Escape => "Esc",
        Delete => "Del",
        Space => "Space",
        Enter => "Enter",
        Tab => "Tab",
        Backspace => "Bksp",
        Home => "Home",
        End => "End",
        PageUp => "PgUp",
        PageDown => "PgDn",
        Insert => "Ins",
        Minus => "-",
        Equals => "=",
        Comma => ",",
        Period => ".",
        Semicolon => ";",
        Quote => "'",
        Backslash => "\\",
        Slash => "/",
        _ => "?",
    }
}

pub fn key_from_label(label: &str) -> Option<egui::Key> {
    all_key_options()
        .iter()
        .find(|(candidate, _)| *candidate == label)
        .map(|(_, key)| *key)
}

/// 所有可选按键列表（用于自定义快捷键界面）
pub fn all_key_options() -> &'static [(&'static str, egui::Key)] {
    use egui::Key::*;
    &[
        ("A", A),
        ("B", B),
        ("C", C),
        ("D", D),
        ("E", E),
        ("F", F),
        ("G", G),
        ("H", H),
        ("I", I),
        ("J", J),
        ("K", K),
        ("L", L),
        ("M", M),
        ("N", N),
        ("O", O),
        ("P", P),
        ("Q", Q),
        ("R", R),
        ("S", S),
        ("T", T),
        ("U", U),
        ("V", V),
        ("W", W),
        ("X", X),
        ("Y", Y),
        ("Z", Z),
        ("0", Num0),
        ("1", Num1),
        ("2", Num2),
        ("3", Num3),
        ("4", Num4),
        ("5", Num5),
        ("6", Num6),
        ("7", Num7),
        ("8", Num8),
        ("9", Num9),
        ("F1", F1),
        ("F2", F2),
        ("F3", F3),
        ("F4", F4),
        ("F5", F5),
        ("F6", F6),
        ("F7", F7),
        ("F8", F8),
        ("F9", F9),
        ("F10", F10),
        ("F11", F11),
        ("F12", F12),
        ("\u{2190}", ArrowLeft),
        ("\u{2192}", ArrowRight),
        ("\u{2191}", ArrowUp),
        ("\u{2193}", ArrowDown),
        ("Esc", Escape),
        ("Del", Delete),
        ("Space", Space),
        ("Enter", Enter),
        ("Tab", Tab),
    ]
}
