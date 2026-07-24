use eframe::egui;
use image::{DynamicImage, GenericImageView};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::sorting::natural_cmp;

/// 主纹理缓存最大数量
const MAX_TEXTURE_BYTES: usize = 512 * 1024 * 1024;
/// 预加载窗口：当前图片前后各几张
const PRELOAD_WINDOW: usize = 3;
/// 缩略图尺寸
const THUMB_SIZE: u32 = 64;
/// 缩略图缓存最大数量
const MAX_THUMBNAIL_CACHE: usize = 200;
const MAX_THUMBNAIL_BYTES: usize = 32 * 1024 * 1024;
/// 图片最大尺寸（超过此边长等比缩小，防止 OOM，也用于截图预览）
const MAX_IMAGE_DIMENSION: u32 = 4096;
const MAX_DECODE_BYTES: u64 = 768 * 1024 * 1024;
pub const MAX_SUPPORTED_IMAGE_BYTES: u64 = 200 * 1024 * 1024;
pub const MAX_SUPPORTED_IMAGE_WIDTH: u32 = 16384;
pub const MAX_SUPPORTED_IMAGE_HEIGHT: u32 = 8640;
pub const MAX_SUPPORTED_IMAGE_PIXELS: u64 = 16384 * 8640;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageLoadAssessment {
    Supported,
    RequiresConfirmation { file_bytes: u64, pixels: u64 },
    Impossible { width: u32, height: u32, pixels: u64 },
}

pub fn assess_image_load(path: &Path) -> ImageLoadAssessment {
    let Some(metadata) = std::fs::metadata(path).ok() else {
        return ImageLoadAssessment::Impossible {
            width: 0,
            height: 0,
            pixels: 0,
        };
    };
    let Some((width, height)) = image::ImageReader::open(path)
        .ok()
        .and_then(|reader| reader.into_dimensions().ok())
    else {
        return ImageLoadAssessment::Impossible {
            width: 0,
            height: 0,
            pixels: 0,
        };
    };
    let pixels = u64::from(width) * u64::from(height);
    if width > MAX_SUPPORTED_IMAGE_WIDTH
        || height > MAX_SUPPORTED_IMAGE_HEIGHT
        || pixels > MAX_SUPPORTED_IMAGE_PIXELS
        || pixels.saturating_mul(4) > MAX_DECODE_BYTES
    {
        return ImageLoadAssessment::Impossible { width, height, pixels };
    }
    if metadata.len() > MAX_SUPPORTED_IMAGE_BYTES {
        return ImageLoadAssessment::RequiresConfirmation {
            file_bytes: metadata.len(),
            pixels,
        };
    }
    ImageLoadAssessment::Supported
}

pub fn image_is_supported(path: &Path) -> bool {
    matches!(assess_image_load(path), ImageLoadAssessment::Supported)
}
pub fn resize_for_target(image: DynamicImage, target_width: u32) -> DynamicImage {
    let target_width = target_width.clamp(1, MAX_IMAGE_DIMENSION);
    let (width, height) = image.dimensions();
    if width <= target_width {
        return image;
    }
    let target_height = ((height as u64 * target_width as u64) / width as u64).max(1) as u32;
    image.resize_exact(target_width, target_height, image::imageops::FilterType::Triangle)
}

/// 图片条目结构体
#[derive(Clone)]
pub struct ImageEntry {
    pub path: PathBuf,
    pub name: String,
    pub manual_rotation: u16, // 手动旋转角度（0, 90, 180, 270）
    pub load_failed: bool,    // 解码失败（显示占位符而非移除条目）
}

/// 支持的图片扩展名
pub fn is_supported_ext(ext: &std::ffi::OsStr) -> bool {
    matches!(
        ext.to_str().unwrap_or("").to_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "tiff" | "tif"
    )
}

/// 扫描文件夹中的图片文件
pub fn scan_folder(folder: &Path) -> Vec<ImageEntry> {
    let mut entries: Vec<ImageEntry> = Vec::new();
    if let Ok(dir) = std::fs::read_dir(folder) {
        for entry in dir.flatten() {
            // 使用 file_type() 避免额外的 stat 系统调用
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if !ft.is_file() {
                continue;
            }
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if is_supported_ext(ext) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    entries.push(ImageEntry {
                        path,
                        name,
                        manual_rotation: 0,
                        load_failed: false,
                    });
                }
            }
        }
    }
    entries.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    entries
}

/// 从嵌入的PNG字节数据加载图标
pub fn load_icon_from_bytes(bytes: &[u8]) -> egui::IconData {
    let img = image::load_from_memory(bytes).expect("Failed to decode icon PNG");
    let rgba = img.to_rgba8();
    egui::IconData {
        rgba: rgba.into_raw(),
        width: img.width(),
        height: img.height(),
    }
}

// ============================================================================
// 主纹理管理器 - LRU 缓存，最多保留 MAX_TEXTURE_CACHE 张
// ============================================================================
pub struct TextureManager {
    /// LRU 队列：最近使用的在前面
    cache: VecDeque<(PathBuf, u16, u32, usize, egui::TextureHandle)>,
}

impl Default for TextureManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureManager {
    pub fn new() -> Self {
        Self {
            cache: VecDeque::with_capacity(8),
        }
    }

    /// 获取或加载纹理（LRU 策略，自动应用 EXIF 旋转 + 手动旋转）
    #[allow(dead_code)]
    pub fn get_or_load(&mut self, ctx: &egui::Context, path: &Path) -> Option<&egui::TextureHandle> {
        self.get_or_load_with_rotation(ctx, path, 0, MAX_IMAGE_DIMENSION)
    }

    /// 获取或加载纹理，带手动旋转角度
    pub fn get_or_load_with_rotation(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        manual_rotation: u16,
        target_width: u32,
    ) -> Option<&egui::TextureHandle> {
        self.get_or_load_with_rotation_policy(ctx, path, manual_rotation, target_width, false)
    }

    pub fn get_or_load_with_rotation_policy(
        &mut self,
        ctx: &egui::Context,
        path: &Path,
        manual_rotation: u16,
        target_width: u32,
        allow_large_file: bool,
    ) -> Option<&egui::TextureHandle> {
        let target_width = target_width.clamp(1, MAX_IMAGE_DIMENSION);
        // 查找并提升到前面（LRU）
        if let Some(pos) = self
            .cache
            .iter()
            .position(|(cached_path, cached_rotation, cached_target, _, _)| {
                cached_path == path && *cached_rotation == manual_rotation && *cached_target == target_width
            })
        {
            let item = self.cache.remove(pos)?;
            self.cache.push_front(item);
            return self.cache.front().map(|(_, _, _, _, texture)| texture);
        }

        // 加载新纹理（速度优化：限制解码大小）
        let assessment = assess_image_load(path);
        if !matches!(assessment, ImageLoadAssessment::Supported)
            && !(allow_large_file && matches!(assessment, ImageLoadAssessment::RequiresConfirmation { .. }))
        {
            return None;
        }
        let mut reader = image::ImageReader::open(path).ok()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_SUPPORTED_IMAGE_WIDTH);
        limits.max_image_height = Some(MAX_SUPPORTED_IMAGE_HEIGHT);
        limits.max_alloc = Some(MAX_DECODE_BYTES);
        reader.limits(limits);
        let img = reader.decode().ok()?;
        // 大图缩小到 MAX_IMAGE_DIMENSION 以内，防止 OOM
        let img = downscale_if_needed(img);
        let mut img = apply_exif_rotation(path, img);

        // 应用手动旋转
        img = match manual_rotation {
            90 => img.rotate90(),
            180 => img.rotate180(),
            270 => img.rotate270(),
            _ => img,
        };
        let img = resize_for_target(img, target_width);

        let rgba = img.to_rgba8();
        let (w, h) = img.dimensions();
        let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        let name = format!("{}#{}x{}", path.to_string_lossy(), target_width, manual_rotation);
        let texture = ctx.load_texture(name, color_image, Default::default());

        let bytes = w as usize * h as usize * 4;
        self.cache
            .push_front((path.to_path_buf(), manual_rotation, target_width, bytes, texture));
        self.trim_to_budget();
        self.cache.front().map(|(_, _, _, _, texture)| texture)
    }

    pub fn get_cached(&mut self, path: &Path, rotation: u16, target_width: u32) -> Option<&egui::TextureHandle> {
        let pos = self
            .cache
            .iter()
            .position(|(cached_path, cached_rotation, cached_target, _, _)| {
                cached_path == path && *cached_rotation == rotation && *cached_target == target_width
            })?;
        let item = self.cache.remove(pos)?;
        self.cache.push_front(item);
        self.cache.front().map(|(_, _, _, _, texture)| texture)
    }

    fn trim_to_budget(&mut self) {
        let mut total = self.cache.iter().map(|(_, _, _, bytes, _)| *bytes).sum::<usize>();
        while total > MAX_TEXTURE_BYTES {
            let Some((_, _, _, bytes, _)) = self.cache.pop_back() else {
                break;
            };
            total = total.saturating_sub(bytes);
        }
    }

    /// 清理纹理缓存：保留当前位置前后 PRELOAD_WINDOW 张
    pub fn cleanup(&mut self, images: &[ImageEntry], current_index: usize) {
        if images.is_empty() {
            self.cache.clear();
            return;
        }
        let start = current_index.saturating_sub(PRELOAD_WINDOW);
        let end = std::cmp::min(current_index + PRELOAD_WINDOW + 1, images.len());

        self.cache
            .retain(|(path, _, _, _, _)| images[start..end].iter().any(|img| img.path == *path));
    }

    pub fn remove(&mut self, path: &Path) {
        self.cache.retain(|(cached_path, _, _, _, _)| cached_path != path);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// 获取已加载纹理的分辨率信息字符串
    pub fn get_size_info(&self, path: &Path) -> String {
        if let Some((_, _, _, _, tex)) = self.cache.iter().find(|(cached_path, _, _, _, _)| cached_path == path) {
            let size = tex.size_vec2();
            format!("{}x{}", size.x as u32, size.y as u32)
        } else {
            String::new()
        }
    }
}

// ============================================================================
// 缩略图管理器 - 独立的小尺寸纹理缓存
// ============================================================================
pub struct ThumbnailManager {
    /// 缩略图缓存：path -> 小纹理
    cache: Vec<(PathBuf, usize, egui::TextureHandle)>,
}

impl Default for ThumbnailManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ThumbnailManager {
    pub fn new() -> Self {
        Self { cache: Vec::new() }
    }

    /// 检查缩略图是否已缓存（不触发加载）
    pub fn is_cached(&self, path: &Path) -> bool {
        self.cache.iter().any(|(p, _, _)| p == path)
    }

    /// 获取已缓存的缩略图（不加载新的）
    pub fn get_cached(&self, path: &Path) -> Option<&egui::TextureHandle> {
        self.cache.iter().find(|(p, _, _)| p == path).map(|(_, _, t)| t)
    }

    /// 获取或生成缩略图纹理（64x64 缩小版）
    pub fn get_or_load(&mut self, ctx: &egui::Context, path: &Path) -> Option<&egui::TextureHandle> {
        // 已缓存直接返回
        if let Some(pos) = self.cache.iter().position(|(p, _, _)| p == path) {
            let item = self.cache.remove(pos);
            self.cache.push(item);
            return self.cache.last().map(|(_, _, texture)| texture);
        }

        // 加载并用快速滤波缩小（Triangle 比 Lanczos3 快 5-10x）
        if !image_is_supported(path) {
            return None;
        }
        let mut reader = image::ImageReader::open(path).ok()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(MAX_SUPPORTED_IMAGE_WIDTH);
        limits.max_image_height = Some(MAX_SUPPORTED_IMAGE_HEIGHT);
        limits.max_alloc = Some(MAX_DECODE_BYTES);
        reader.limits(limits);
        let img = reader.decode().ok()?;
        let (orig_w, orig_h) = img.dimensions();
        let scale = (THUMB_SIZE as f32 / orig_w as f32)
            .min(THUMB_SIZE as f32 / orig_h as f32)
            .min(1.0);
        let new_w = (orig_w as f32 * scale).max(1.0) as u32;
        let new_h = (orig_h as f32 * scale).max(1.0) as u32;
        let thumb = img.resize_exact(new_w, new_h, image::imageops::FilterType::Triangle);
        let rgba = thumb.to_rgba8();
        let color_image = egui::ColorImage::from_rgba_unmultiplied([new_w as usize, new_h as usize], &rgba);
        let name = format!("jinn://thumbnail/{:016x}", stable_path_hash(path));
        let texture = ctx.load_texture(name, color_image, Default::default());

        self.cache.push((path.to_path_buf(), rgba.len(), texture));

        // LRU 淘汰：超过上限时移除最早加入的
        while self.cache.len() > MAX_THUMBNAIL_CACHE || self.total_bytes() > MAX_THUMBNAIL_BYTES {
            let _ = self.cache.remove(0);
        }

        self.cache.last().map(|(_, _, texture)| texture)
    }

    pub fn remove(&mut self, path: &Path) {
        self.cache.retain(|(p, _, _)| p != path);
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn cleanup(&mut self, images: &[ImageEntry]) {
        self.cache
            .retain(|(path, _, _)| images.iter().any(|image| image.path == *path));
    }

    fn total_bytes(&self) -> usize {
        self.cache.iter().map(|(_, bytes, _)| *bytes).sum()
    }
}

fn stable_path_hash(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    hasher.finish()
}

// ============================================================================
// 尺寸计算工具函数
// ============================================================================

/// 调整纹理大小以适应视口（不放大）
pub fn fit_size(tex_size: egui::Vec2, viewport: egui::Vec2) -> egui::Vec2 {
    if tex_size.x <= 0.0 || tex_size.y <= 0.0 || viewport.x <= 0.0 || viewport.y <= 0.0 {
        return tex_size;
    }
    let scale = (viewport.x / tex_size.x).min(viewport.y / tex_size.y);
    tex_size * scale.min(1.0)
}

/// 调整纹理大小以适应最大尺寸（不放大）
pub fn fit_size_to_max(tex_size: egui::Vec2, max_size: egui::Vec2) -> egui::Vec2 {
    if tex_size.x <= 0.0 || tex_size.y <= 0.0 || max_size.x <= 0.0 || max_size.y <= 0.0 {
        return tex_size;
    }
    let scale = (max_size.x / tex_size.x).min(max_size.y / tex_size.y);
    tex_size * scale.min(1.0)
}

/// 如果图片边长超过 MAX_IMAGE_DIMENSION，等比缩小
pub fn downscale_if_needed(img: DynamicImage) -> DynamicImage {
    let (w, h) = img.dimensions();
    let max_dim = w.max(h);
    if max_dim > MAX_IMAGE_DIMENSION {
        let scale = MAX_IMAGE_DIMENSION as f32 / max_dim as f32;
        let new_w = (w as f32 * scale) as u32;
        let new_h = (h as f32 * scale) as u32;
        img.resize_exact(new_w.max(1), new_h.max(1), image::imageops::FilterType::Triangle)
    } else {
        img
    }
}

// ============================================================================
// EXIF 旋转处理
// ============================================================================

/// 读取 EXIF Orientation 并应用旋转/翻转
fn apply_exif_rotation(path: &Path, img: DynamicImage) -> DynamicImage {
    let orientation = read_exif_orientation(path).unwrap_or(1);
    match orientation {
        1 => img, // 正常
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

/// 从文件读取 EXIF Orientation 值
fn read_exif_orientation(path: &Path) -> Option<u32> {
    let file = std::fs::File::open(path).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let exif = exif::Reader::new().read_from_container(&mut reader).ok()?;
    let field = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?;
    field.value.get_uint(0)
}

/// 读取 EXIF 元数据，返回 (字段名, 值) 列表
/// 使用手动字节解析，避免 kamadak-exif 的`get_field` IFD 过滤限制
pub fn read_exif_metadata(path: &Path) -> Vec<(String, String)> {
    use std::io::Read;
    let mut buf = Vec::new();
    if std::fs::File::open(path)
        .and_then(|mut f| f.read_to_end(&mut buf))
        .is_err()
    {
        return Vec::new();
    }
    if buf.is_empty() {
        return Vec::new();
    }

    // 定位到 TIFF 块起始位置（解析 JPEG APP1 / 原始 TIFF / PNG eXIf）
    let tiff_start = find_exif_tiff(&buf);
    if tiff_start + 8 > buf.len() {
        return Vec::new();
    }

    // 读取 TIFF 头：字节序 + 魔数 0x002A + IFD0 偏移
    let le = buf[tiff_start] == b'I' && buf[tiff_start + 1] == b'I';
    let be = buf[tiff_start] == b'M' && buf[tiff_start + 1] == b'M';
    if !le && !be {
        return Vec::new();
    }
    if read_u16(&buf, tiff_start + 2, le) != 0x002A {
        return Vec::new();
    }
    let ifd0_offset = read_u32(&buf, tiff_start + 4, le) as usize;
    if ifd0_offset == 0 || ifd0_offset + 2 > buf.len() {
        return Vec::new();
    }

    let mut result = Vec::new();

    // 解析 IFD0 + 递归解析 EXIF 子 IFD
    parse_ifd(&buf, tiff_start + ifd0_offset, tiff_start, le, &mut result);

    // 只保留非空值
    result.retain(|(_, v)| !v.is_empty());

    result
}

/// 文件系统信息 + 图片解码信息 + EXIF = 完整图片信息
/// 返回 [(section_title, Vec<(label, value)>)]
pub fn get_image_info(path: &Path) -> Vec<(String, Vec<(String, String)>)> {
    let mut sections: Vec<(String, Vec<(String, String)>)> = Vec::new();

    // ====================================================================
    // 1. 文件信息
    // ====================================================================
    let mut file_info = Vec::new();
    if let Ok(meta) = std::fs::metadata(path) {
        // 文件名
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        file_info.push(("name".to_string(), name));
        // 完整路径
        file_info.push(("path".to_string(), path.to_string_lossy().to_string()));
        // 文件大小
        let bytes = meta.len();
        let size_str = if bytes >= 1_073_741_824 {
            format!("{:.2} GB", bytes as f64 / 1_073_741_824.0)
        } else if bytes >= 1_048_576 {
            format!("{:.2} MB", bytes as f64 / 1_048_576.0)
        } else if bytes >= 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        };
        file_info.push(("size".to_string(), size_str));
        // 文件类型
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        file_info.push(("type".to_string(), format_imagetype(ext).to_string()));
        file_info.push(("extension".to_string(), format!(".{}", ext)));
        // 时间
        if let Ok(created) = meta.created() {
            file_info.push(("created".to_string(), format_datetime(created)));
        }
        if let Ok(modified) = meta.modified() {
            file_info.push(("modified".to_string(), format_datetime(modified)));
        }
        if let Ok(accessed) = meta.accessed() {
            file_info.push(("accessed".to_string(), format_datetime(accessed)));
        }
        // 权限
        let perm = meta.permissions();
        file_info.push((
            "permissions".to_string(),
            if perm.readonly() {
                "只读".to_string()
            } else {
                "读写".to_string()
            },
        ));
    }
    if !file_info.is_empty() {
        sections.push(("file_info".to_string(), file_info));
    }

    // ====================================================================
    // 2. 图片属性（核心）
    // ====================================================================
    let mut props = Vec::new();
    // 尝试解码获取维度 + 颜色信息
    if let Ok(reader) = image::ImageReader::open(path) {
        if let Ok(reader) = reader.with_guessed_format() {
            let fmt = reader.format();
            if let Ok(dims) = reader.into_dimensions() {
                let (w, h) = dims;
                props.push(("width".to_string(), format!("{} px", w)));
                props.push(("height".to_string(), format!("{} px", h)));
                props.push(("resolution".to_string(), format!("{} × {}", w, h)));
                // 比例
                let gcd = gcd(w, h);
                let ar_w = w / gcd;
                let ar_h = h / gcd;
                if ar_w <= 99 && ar_h <= 99 {
                    props.push(("aspect_ratio".to_string(), format!("{}:{}", ar_w, ar_h)));
                }
                // 格式
                if let Some(fmt) = fmt {
                    props.push(("format".to_string(), format_imagetype_tag(fmt).to_string()));
                    props.push(("mime".to_string(), format_mime(fmt).to_string()));
                }
            }
        }
    }
    // 颜色信息：重新解码（成本较高，但只在打开信息面板时发生一次）
    if let Ok(reader) = image::ImageReader::open(path) {
        if let Ok(reader) = reader.with_guessed_format() {
            if let Ok(img) = reader.decode() {
                let color = img.color();
                props.push(("color_mode".to_string(), format_color_mode(color).to_string()));
                props.push(("channels".to_string(), format_channels(color).to_string()));
                props.push(("bit_depth".to_string(), format_bitdepth(color).to_string()));
                props.push((
                    "bits_per_channel".to_string(),
                    format_bits_per_channel(color).to_string(),
                ));
                let has_alpha = color.has_alpha();
                props.push((
                    "alpha".to_string(),
                    if has_alpha {
                        "支持透明".to_string()
                    } else {
                        "否".to_string()
                    },
                ));
            }
        }
    }
    if !props.is_empty() {
        sections.push(("image_properties".to_string(), props));
    }

    // ====================================================================
    // 3. EXIF 信息（如有）
    // ====================================================================
    let exif = read_exif_metadata(path);
    if !exif.is_empty() {
        // 按类别分组 EXIF
        let exif_section = group_exif(exif);
        if !exif_section.is_empty() {
            sections.push(("exif".to_string(), exif_section));
        }
    }

    sections
}

/// 将原始 EXIF 按类别分组并映射为可读标签
fn group_exif(raw: Vec<(String, String)>) -> Vec<(String, String)> {
    use std::collections::HashMap;
    let mut map: HashMap<String, String> = HashMap::new();
    for (k, v) in &raw {
        map.insert(k.clone(), v.clone());
    }
    let mut result = Vec::new();

    // 相机
    if let Some(v) = map.get("Make") {
        result.push(("camera_make".to_string(), v.clone()));
    }
    if let Some(v) = map.get("Model") {
        result.push(("camera_model".to_string(), v.clone()));
    }
    // 镜头
    if let Some(v) = map.get("LensModel") {
        result.push(("lens".to_string(), v.clone()));
    }
    if let Some(v) = map.get("LensMake") {
        result.push(("lens_make".to_string(), v.clone()));
    }
    // 拍摄参数
    if let Some(v) = map.get("FocalLength") {
        result.push(("focal_length".to_string(), v.clone()));
    }
    if let Some(v) = map.get("FNumber") {
        result.push(("aperture".to_string(), v.clone()));
    }
    if let Some(v) = map.get("ExposureTime") {
        result.push(("shutter".to_string(), v.clone()));
    }
    if let Some(v) = map.get("ISOSpeed") {
        result.push(("iso".to_string(), v.clone()));
    }
    if let Some(v) = map.get("ExposureBiasValue") {
        result.push(("exposure_bias".to_string(), v.clone()));
    }
    if let Some(v) = map.get("FocalLengthIn35mmFilm") {
        result.push(("focal_35mm".to_string(), v.clone()));
    }
    if let Some(v) = map.get("ExposureProgram") {
        result.push(("exposure_program".to_string(), v.clone()));
    }
    if let Some(v) = map.get("MeteringMode") {
        result.push(("metering_mode".to_string(), v.clone()));
    }
    if let Some(v) = map.get("Flash") {
        result.push(("flash".to_string(), v.clone()));
    }
    if let Some(v) = map.get("WhiteBalance") {
        result.push(("white_balance".to_string(), v.clone()));
    }
    if let Some(v) = map.get("DigitalZoomRatio") {
        result.push(("digital_zoom".to_string(), v.clone()));
    }
    if let Some(v) = map.get("ColorSpace") {
        result.push(("color_space".to_string(), v.clone()));
    }
    // 时间
    if let Some(v) = map.get("DateTimeOriginal") {
        result.push(("datetime_original".to_string(), v.clone()));
    }
    if let Some(v) = map.get("DateTimeDigitized") {
        result.push(("datetime_digitized".to_string(), v.clone()));
    }
    // 软件
    if let Some(v) = map.get("Software") {
        result.push(("software".to_string(), v.clone()));
    }
    // 其他 EXIF
    for (k, v) in &raw {
        if !result.iter().any(|(rk, _)| rk == k) {
            // Only add non-standard tags
            if ![
                "Make",
                "Model",
                "LensModel",
                "LensMake",
                "FocalLength",
                "FNumber",
                "ExposureTime",
                "ISOSpeed",
                "ExposureBiasValue",
                "FocalLengthIn35mmFilm",
                "ExposureProgram",
                "MeteringMode",
                "Flash",
                "WhiteBalance",
                "DigitalZoomRatio",
                "ColorSpace",
                "DateTimeOriginal",
                "DateTimeDigitized",
                "Software",
                "Orientation",
                "ExifIFD",
                "GPSInfo",
                "PixelXDimension",
                "PixelYDimension",
                "XResolution",
                "YResolution",
            ]
            .contains(&k.as_str())
            {
                result.push((k.clone(), v.clone()));
            }
        }
    }

    result
}

/// GCD 用于计算宽高比
fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// 格式化文件系统时间
fn format_datetime(t: std::time::SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    if let Ok(dur) = t.duration_since(UNIX_EPOCH) {
        let secs = dur.as_secs();
        // Simple UTC datetime formatting
        let days = secs / 86400;
        let time_secs = secs % 86400;
        let h = time_secs / 3600;
        let m = (time_secs % 3600) / 60;
        let s = time_secs % 60;

        // Days since epoch to date (Gregorian)
        let mut y = 1970i64;
        let mut d = days as i64;
        loop {
            let days_in_year = if is_leap(y) { 366 } else { 365 };
            if d < days_in_year {
                break;
            }
            d -= days_in_year;
            y += 1;
        }
        let months_days = if is_leap(y) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };
        let mut mo = 0usize;
        for (mi, &md) in months_days.iter().enumerate() {
            if d < md {
                mo = mi;
                break;
            }
            d -= md;
        }
        format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, mo + 1, d as u32 + 1, h, m, s)
    } else {
        String::new()
    }
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// 文件扩展名 → 可读类型名
fn format_imagetype(ext: &str) -> &'static str {
    match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "JPEG Image",
        "png" => "PNG Image",
        "bmp" => "BMP Image",
        "gif" => "GIF Image",
        "webp" => "WebP Image",
        "tiff" | "tif" => "TIFF Image",
        "avif" => "AVIF Image",
        _ => "Unknown",
    }
}

/// image::ImageFormat → 可读类型名
fn format_imagetype_tag(fmt: image::ImageFormat) -> &'static str {
    match fmt {
        image::ImageFormat::Jpeg => "JPEG",
        image::ImageFormat::Png => "PNG",
        image::ImageFormat::Bmp => "BMP",
        image::ImageFormat::Gif => "GIF",
        image::ImageFormat::WebP => "WebP",
        image::ImageFormat::Tiff => "TIFF",
        _ => "Unknown",
    }
}

/// MIME 类型
fn format_mime(fmt: image::ImageFormat) -> &'static str {
    match fmt {
        image::ImageFormat::Jpeg => "image/jpeg",
        image::ImageFormat::Png => "image/png",
        image::ImageFormat::Bmp => "image/bmp",
        image::ImageFormat::Gif => "image/gif",
        image::ImageFormat::WebP => "image/webp",
        image::ImageFormat::Tiff => "image/tiff",
        _ => "application/octet-stream",
    }
}

/// ColorType → 颜色模式名
fn format_color_mode(c: image::ColorType) -> &'static str {
    match c {
        image::ColorType::L8 | image::ColorType::L16 => "灰度",
        image::ColorType::La8 | image::ColorType::La16 => "灰度+Alpha",
        image::ColorType::Rgb8 | image::ColorType::Rgb16 => "RGB",
        image::ColorType::Rgba8 | image::ColorType::Rgba16 => "RGBA",
        image::ColorType::Rgb32F => "RGB (Float)",
        image::ColorType::Rgba32F => "RGBA (Float)",
        _ => "Unknown",
    }
}

/// ColorType → 通道数
fn format_channels(c: image::ColorType) -> &'static str {
    match c {
        image::ColorType::L8 | image::ColorType::L16 => "1 (灰度)",
        image::ColorType::La8 | image::ColorType::La16 => "2 (灰度+Alpha)",
        image::ColorType::Rgb8 | image::ColorType::Rgb16 | image::ColorType::Rgb32F => "3 (RGB)",
        image::ColorType::Rgba8 | image::ColorType::Rgba16 | image::ColorType::Rgba32F => "4 (RGBA)",
        _ => "?",
    }
}

/// ColorType → 总位深
fn format_bitdepth(c: image::ColorType) -> &'static str {
    match c {
        image::ColorType::L8 => "8 bit",
        image::ColorType::L16 => "16 bit",
        image::ColorType::La8 => "16 bit",
        image::ColorType::La16 => "32 bit",
        image::ColorType::Rgb8 => "24 bit",
        image::ColorType::Rgb16 => "48 bit",
        image::ColorType::Rgba8 => "32 bit",
        image::ColorType::Rgba16 => "64 bit",
        image::ColorType::Rgb32F => "96 bit",
        image::ColorType::Rgba32F => "128 bit",
        _ => "?",
    }
}

/// ColorType → 每通道位数
fn format_bits_per_channel(c: image::ColorType) -> &'static str {
    match c {
        image::ColorType::L8 | image::ColorType::La8 | image::ColorType::Rgb8 | image::ColorType::Rgba8 => "8 bit",
        image::ColorType::L16 | image::ColorType::La16 | image::ColorType::Rgb16 | image::ColorType::Rgba16 => "16 bit",
        image::ColorType::Rgb32F | image::ColorType::Rgba32F => "32 bit (Float)",
        _ => "?",
    }
}

/// 在 JPEG/TIFF/PNG 文件中定位 TIFF 块的起始位置
fn find_exif_tiff(buf: &[u8]) -> usize {
    // JPEG: FF D8 ... FF E1 (APP1) + "Exif\0\0" + TIFF
    if buf.len() > 4 && buf[0] == 0xFF && buf[1] == 0xD8 {
        let mut i = 2;
        while i + 8 < buf.len() {
            if buf[i] != 0xFF {
                break;
            }
            let marker = buf[i + 1];
            let seg_len = read_u16(buf, i + 2, false) as usize;
            if seg_len < 2 {
                break;
            }
            if marker == 0xE1
                && seg_len >= 8
                && buf[i + 4] == b'E'
                && buf[i + 5] == b'x'
                && buf[i + 6] == b'i'
                && buf[i + 7] == b'f'
            {
                return i + 4 + 6; // skip "Exif\0\0" (6 bytes)
            }
            i += 2 + seg_len;
            if i >= buf.len() {
                break;
            }
        }
        return 0;
    }
    // TIFF raw: starts with II or MM
    if buf.len() > 2 && ((buf[0] == b'I' && buf[1] == b'I') || (buf[0] == b'M' && buf[1] == b'M')) {
        return 0;
    }
    0
}

/// 解析一个 IFD 的所有条目
fn parse_ifd(buf: &[u8], start: usize, tiff_base: usize, le: bool, result: &mut Vec<(String, String)>) {
    if start + 2 > buf.len() {
        return;
    }
    let count = read_u16(buf, start, le) as usize;
    if count > 200 || start + 2 + count * 12 > buf.len() {
        return;
    }
    for i in 0..count {
        let entry_off = start + 2 + i * 12;
        if entry_off + 12 > buf.len() {
            break;
        }
        let tag_id = read_u16(buf, entry_off, le);
        let typ = read_u16(buf, entry_off + 2, le);
        let count2 = read_u32(buf, entry_off + 4, le) as usize;

        // 标签名
        let tag_name = exif_tag_name(tag_id);
        if tag_name.is_empty() && tag_id != 0x8769 && tag_id != 0x8825 {
            continue;
        }

        // TIFF type sizes: 1=byte, 2=ascii, 3=short, 4=long, 5=rational, 7=undefined, 9=slong, 10=srational
        let (value_str, is_pointer) = match typ {
            2 => {
                // ASCII string: inline if <=4 bytes, else offset
                let raw = if count2 <= 4 {
                    buf[entry_off + 8..entry_off + 8 + count2.min(4)].to_vec()
                } else {
                    let data_off = read_u32(buf, entry_off + 8, le) as usize;
                    if tiff_base + data_off + count2 > buf.len() {
                        continue;
                    }
                    buf[tiff_base + data_off..tiff_base + data_off + count2].to_vec()
                };
                let s = String::from_utf8_lossy(&raw);
                let trimmed = s.trim_end_matches('\0').trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                (trimmed, false)
            }
            3 => {
                // SHORT (16-bit). Inline in first 2 bytes of value field.
                if count2 == 1 {
                    let v = read_u16(buf, entry_off + 8, le) as i64;
                    (format_value(tag_id, v), tag_id == 0x8769 || tag_id == 0x8825)
                } else {
                    let raw = if count2 * 2 <= 4 {
                        buf[entry_off + 8..entry_off + 8 + (count2 * 2).min(4)].to_vec()
                    } else {
                        let data_off = read_u32(buf, entry_off + 8, le) as usize;
                        if tiff_base + data_off + count2 * 2 > buf.len() {
                            continue;
                        }
                        buf[tiff_base + data_off..tiff_base + data_off + count2 * 2].to_vec()
                    };
                    let mut parts = Vec::new();
                    for j in 0..count2.min(10) {
                        let v = if j * 2 + 1 < raw.len() {
                            read_u16(&raw, j * 2, le) as i64
                        } else {
                            break;
                        };
                        parts.push(format_value(tag_id, v));
                    }
                    (parts.join(", "), false)
                }
            }
            4 => {
                // LONG (32-bit). Inline in 4-byte value field.
                if count2 == 1 {
                    let v = read_u32(buf, entry_off + 8, le) as i64;
                    (format_value(tag_id, v), tag_id == 0x8769 || tag_id == 0x8825)
                } else {
                    let data_off = read_u32(buf, entry_off + 8, le) as usize;
                    if tiff_base + data_off + count2 * 4 > buf.len() {
                        continue;
                    }
                    let mut parts = Vec::new();
                    for j in 0..count2.min(10) {
                        let v = read_u32(buf, tiff_base + data_off + j * 4, le) as i64;
                        parts.push(format_value(tag_id, v));
                    }
                    (parts.join(", "), false)
                }
            }
            5 | 10 => {
                // RATIONAL / SRATIONAL (8 bytes each). Always via offset.
                let data_off = read_u32(buf, entry_off + 8, le) as usize;
                if tiff_base + data_off + 8 > buf.len() {
                    continue;
                }
                let num = read_u32(buf, tiff_base + data_off, le);
                let den = read_u32(buf, tiff_base + data_off + 4, le);
                if den == 0 {
                    (format_value(tag_id, num as i64), false)
                } else {
                    (format_rational(tag_id, num, den), false)
                }
            }
            1 | 6 | 7 => {
                // BYTE / SBYTE / UNDEFINED
                let raw = if count2 <= 4 {
                    buf[entry_off + 8..entry_off + 8 + count2.min(4)].to_vec()
                } else {
                    let data_off = read_u32(buf, entry_off + 8, le) as usize;
                    if tiff_base + data_off + count2 > buf.len() {
                        continue;
                    }
                    buf[tiff_base + data_off..tiff_base + data_off + count2].to_vec()
                };
                if count2 <= 16 {
                    let s = raw.iter().map(|b| format!("{}", b)).collect::<Vec<_>>().join(" ");
                    (s, false)
                } else {
                    continue;
                }
            }
            _ => (String::new(), false),
        };

        if !value_str.is_empty() && !is_pointer {
            result.push((tag_name.to_string(), value_str));
        }

        // 如果遇到 ExifIFD 指针 (0x8769)，递归解析子 IFD
        if tag_id == 0x8769 {
            let sub_ifd_off = read_u32(buf, entry_off + 8, le) as usize;
            if tiff_base + sub_ifd_off + 2 <= buf.len() {
                parse_ifd(buf, tiff_base + sub_ifd_off, tiff_base, le, result);
            }
        }
        // 如果遇到 GPS IFD 指针 (0x8825)，递归解析
        if tag_id == 0x8825 {
            let gps_ifd_off = read_u32(buf, entry_off + 8, le) as usize;
            if tiff_base + gps_ifd_off + 2 <= buf.len() {
                parse_ifd(buf, tiff_base + gps_ifd_off, tiff_base, le, result);
            }
        }
    }
}

fn read_u16(buf: &[u8], off: usize, le: bool) -> u16 {
    if off + 1 >= buf.len() {
        return 0;
    }
    if le {
        (buf[off] as u16) | ((buf[off + 1] as u16) << 8)
    } else {
        ((buf[off] as u16) << 8) | (buf[off + 1] as u16)
    }
}

fn read_u32(buf: &[u8], off: usize, le: bool) -> u32 {
    if off + 3 >= buf.len() {
        return 0;
    }
    if le {
        (buf[off] as u32) | ((buf[off + 1] as u32) << 8) | ((buf[off + 2] as u32) << 16) | ((buf[off + 3] as u32) << 24)
    } else {
        ((buf[off] as u32) << 24) | ((buf[off + 1] as u32) << 16) | ((buf[off + 2] as u32) << 8) | (buf[off + 3] as u32)
    }
}

/// 根据标签 ID 返回可读的字段名
fn exif_tag_name(tag: u16) -> &'static str {
    match tag {
        0x010F => "Make",
        0x0110 => "Model",
        0x010E => "ImageDescription",
        0x0131 => "Software",
        0x013B => "Artist",
        0x8298 => "Copyright",
        0x0112 => "Orientation",
        0x011A => "XResolution",
        0x011B => "YResolution",
        0x0213 => "YCbCrPositioning",
        0x8769 => "ExifIFD",
        0x8825 => "GPSInfo",
        0x9003 => "DateTimeOriginal",
        0x9004 => "DateTimeDigitized",
        0x9101 => "ComponentsConfiguration",
        0x9102 => "CompressedBitsPerPixel",
        0x9201 => "ShutterSpeedValue",
        0x9202 => "ApertureValue",
        0x9203 => "BrightnessValue",
        0x9204 => "ExposureBiasValue",
        0x9205 => "MaxApertureValue",
        0x9206 => "SubjectDistance",
        0x9207 => "MeteringMode",
        0x9208 => "LightSource",
        0x9209 => "Flash",
        0x920A => "FocalLength",
        0x927C => "MakerNote",
        0x9286 => "UserComment",
        0x9290 => "SubSecTime",
        0x9291 => "SubSecTimeOriginal",
        0x9292 => "SubSecTimeDigitized",
        0x829A => "ExposureTime",
        0x829D => "FNumber",
        0x8822 => "ExposureProgram",
        0x8824 => "SpectralSensitivity",
        0x8827 => "ISOSpeed",
        0x8828 => "OECF",
        0x8830 => "SensitivityType",
        0x8831 => "StandardOutputSensitivity",
        0x8832 => "RecommendedExposureIndex",
        0x8833 => "ISOSpeed",
        0x8834 => "ISOSpeedLatitudeyyy",
        0x8835 => "ISOSpeedLatitudezzz",
        0xA001 => "ColorSpace",
        0xA002 => "PixelXDimension",
        0xA003 => "PixelYDimension",
        0xA004 => "RelatedSoundFile",
        0xA005 => "InteroperabilityIFD",
        0xA20B => "FlashEnergy",
        0xA20C => "SpatialFrequencyResponse",
        0xA20E => "FocalPlaneXResolution",
        0xA20F => "FocalPlaneYResolution",
        0xA210 => "FocalPlaneResolutionUnit",
        0xA214 => "SubjectLocation",
        0xA215 => "ExposureIndex",
        0xA217 => "SensingMethod",
        0xA300 => "FileSource",
        0xA301 => "SceneType",
        0xA302 => "CFAPattern",
        0xA401 => "CustomRendered",
        0xA402 => "ExposureMode",
        0xA403 => "WhiteBalance",
        0xA404 => "DigitalZoomRatio",
        0xA405 => "FocalLengthIn35mmFilm",
        0xA406 => "SceneCaptureType",
        0xA407 => "GainControl",
        0xA408 => "Contrast",
        0xA409 => "Saturation",
        0xA40A => "Sharpness",
        0xA40B => "DeviceSettingDescription",
        0xA40C => "SubjectDistanceRange",
        0xA420 => "ImageUniqueID",
        0xA430 => "CameraOwnerName",
        0xA431 => "BodySerialNumber",
        0xA432 => "LensSpecification",
        0xA433 => "LensMake",
        0xA434 => "LensModel",
        0xA435 => "LensSerialNumber",
        // GPS tags
        0x0000 => "GPSVersionID",
        0x0001 => "GPSLatitudeRef",
        0x0002 => "GPSLatitude",
        0x0003 => "GPSLongitudeRef",
        0x0004 => "GPSLongitude",
        0x0005 => "GPSAltitudeRef",
        0x0006 => "GPSAltitude",
        0x0007 => "GPSTimeStamp",
        0x0008 => "GPSSatellites",
        0x0009 => "GPSStatus",
        0x000A => "GPSMeasureMode",
        0x000B => "GPSDOP",
        0x000C => "GPSSpeedRef",
        0x000D => "GPSSpeed",
        0x000E => "GPSTrackRef",
        0x000F => "GPSTrack",
        0x0010 => "GPSImgDirectionRef",
        0x0011 => "GPSImgDirection",
        0x0012 => "GPSMapDatum",
        0x0013 => "GPSDestLatitudeRef",
        0x0014 => "GPSDestLatitude",
        0x0015 => "GPSDestLongitudeRef",
        0x0016 => "GPSDestLongitude",
        0x0017 => "GPSDestBearingRef",
        0x0018 => "GPSDestBearing",
        0x0019 => "GPSDestDistanceRef",
        0x001A => "GPSDestDistance",
        0x001B => "GPSProcessingMethod",
        0x001C => "GPSAreaInformation",
        0x001D => "GPSDateStamp",
        0x001E => "GPSDifferential",
        0x001F => "GPSHPositioningError",
        _ => "",
    }
}

/// 根据标签 ID 格式化整数值（如 Orientation 1→"Horizontal"）
fn format_value(tag: u16, val: i64) -> String {
    match tag {
        0x0112 => match val {
            1 => "Horizontal".into(),
            2 => "Mirror".into(),
            3 => "Rotate 180".into(),
            4 => "Flip".into(),
            5 => "Mirror+Rotate90".into(),
            6 => "Rotate 90 CW".into(),
            7 => "Mirror+Rotate270".into(),
            8 => "Rotate 90 CCW".into(),
            _ => val.to_string(),
        },
        0x9207 => match val {
            0 => "Unknown".into(),
            1 => "Average".into(),
            2 => "CenterWeighted".into(),
            3 => "Spot".into(),
            4 => "MultiSpot".into(),
            5 => "Pattern".into(),
            6 => "Partial".into(),
            255 => "Other".into(),
            _ => val.to_string(),
        },
        0x9209 => match val {
            0x0000 => "No Flash".into(),
            0x0001 => "Flash".into(),
            0x0005 => "Flash (Return detected)".into(),
            0x0007 => "Flash (Return detected)".into(),
            0x0009 => "Flash (Compulsory)".into(),
            0x000D => "Flash (Compulsory, Return)".into(),
            0x000F => "Flash (Compulsory, Return)".into(),
            0x0010 => "No Flash".into(),
            0x0018 => "No Flash".into(),
            0x0019 => "Flash (Auto)".into(),
            0x001D => "Flash (Auto, Return)".into(),
            0x001F => "Flash (Auto, Return)".into(),
            0x0020 => "No Flash".into(),
            0x0041 => "Flash (Red-eye)".into(),
            0x0045 => "Flash (Red-eye, Return)".into(),
            0x0047 => "Flash (Red-eye, Return)".into(),
            0x0049 => "Flash (Red-eye, Compulsory)".into(),
            0x004D => "Flash (Red-eye, Compulsory, Return)".into(),
            0x004F => "Flash (Red-eye, Compulsory, Return)".into(),
            0x0059 => "Flash (Red-eye, Auto)".into(),
            0x005D => "Flash (Red-eye, Auto, Return)".into(),
            0x005F => "Flash (Red-eye, Auto, Return)".into(),
            _ => val.to_string(),
        },
        0x8822 => match val {
            0 => "Not defined".into(),
            1 => "Manual".into(),
            2 => "Program AE".into(),
            3 => "Aperture-priority AE".into(),
            4 => "Shutter speed priority AE".into(),
            5 => "Creative".into(),
            6 => "Action".into(),
            7 => "Portrait".into(),
            8 => "Landscape".into(),
            9 => "Bulb".into(),
            _ => val.to_string(),
        },
        0xA001 => match val {
            1 => "sRGB".into(),
            2 => "Adobe RGB".into(),
            65535 => "Uncalibrated".into(),
            _ => val.to_string(),
        },
        0xA403 => match val {
            0 => "Auto".into(),
            1 => "Manual".into(),
            _ => val.to_string(),
        },
        0xA402 => match val {
            0 => "Auto".into(),
            1 => "Manual".into(),
            2 => "Auto bracket".into(),
            _ => val.to_string(),
        },
        0xA406 => match val {
            0 => "Standard".into(),
            1 => "Landscape".into(),
            2 => "Portrait".into(),
            3 => "Night".into(),
            _ => val.to_string(),
        },
        0x9208 => match val {
            0 => "Unknown".into(),
            1 => "Daylight".into(),
            2 => "Fluorescent".into(),
            3 => "Tungsten".into(),
            4 => "Flash".into(),
            9 => "Fine weather".into(),
            10 => "Cloudy".into(),
            11 => "Shade".into(),
            12 => "Fluorescent D".into(),
            13 => "Fluorescent N".into(),
            14 => "Fluorescent W".into(),
            15 => "White fluorescent".into(),
            17 => "Standard A".into(),
            18 => "Standard B".into(),
            19 => "Standard C".into(),
            20 => "D55".into(),
            21 => "D65".into(),
            22 => "D75".into(),
            23 => "D50".into(),
            24 => "ISO studio".into(),
            255 => "Other".into(),
            _ => val.to_string(),
        },
        0xA217 => match val {
            1 => "Not defined".into(),
            2 => "One-chip color".into(),
            3 => "Two-chip color".into(),
            4 => "Three-chip color".into(),
            5 => "Color sequential".into(),
            7 => "Trilinear".into(),
            8 => "Color sequential linear".into(),
            _ => val.to_string(),
        },
        0xA301 => match val {
            1 => "Directly photographed".into(),
            _ => val.to_string(),
        },
        0xA300 => match val {
            1 => "DSC".into(),
            _ => val.to_string(),
        },
        _ => val.to_string(),
    }
}

/// 格式化有理数
fn format_rational(tag: u16, num: u32, den: u32) -> String {
    match tag {
        0x829A => {
            // ExposureTime
            if den > 0 && num < den {
                format!("1/{}", den / num.max(1))
            } else if den > 0 {
                format!("{:.2}s", num as f64 / den as f64)
            } else {
                format!("{}/{}", num, den)
            }
        }
        0x829D => {
            // FNumber
            if den > 0 {
                format!("F/{:.1}", num as f64 / den as f64)
            } else {
                format!("{}/{}", num, den)
            }
        }
        0x920A => {
            // FocalLength
            if den > 0 {
                format!("{:.1} mm", num as f64 / den as f64)
            } else {
                format!("{}/{}", num, den)
            }
        }
        0xA432 => {
            // LensSpecification
            if den > 0 {
                format!("{:.0}-{:.0}mm", num as f64 / den as f64, 0.0)
            } else {
                format!("{}/{}", num, den)
            }
        }
        0x9205 => {
            // MaxApertureValue
            if den > 0 && num > 0 {
                let f_stop = (num as f64 / den as f64) / 2.0_f64.ln();
                format!("F/{:.1}", f_stop.exp())
            } else {
                format!("{}/{}", num, den)
            }
        }
        0x9204 => {
            // ExposureBiasValue
            if den > 0 {
                format!("{:.2} EV", num as f64 / den as f64)
            } else {
                format!("{}/{}", num, den)
            }
        }
        _ => {
            if den == 1 || den == 0 {
                format!("{}", num)
            } else if num < den && den < 10000 {
                format!("{}/{}", num, den)
            } else {
                format!("{:.2}", num as f64 / den as f64)
            }
        }
    }
}

// ============================================================================
// GIF 动画播放管理
// ============================================================================

/// GIF 最大帧数限制（超过此帧数后均匀采样，防止内存爆炸）
const MAX_GIF_FRAMES: usize = 256;

/// GIF 帧数据（仅存 CPU 端 ColorImage，GPU 纹理按需上传）
struct GifFrame {
    color_image: egui::ColorImage,
    delay: Duration,
}

/// 待解码的 GIF 帧原始数据
struct GifRawFrame {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    delay: Duration,
}

/// 初始加载帧数上限
const GIF_INITIAL_FRAMES: usize = 10;

/// GIF 动画管理器（渐进式加载，GPU 仅缓存当前帧）
pub struct GifAnimator {
    frames: Vec<GifFrame>,
    pending_frames: Vec<GifRawFrame>,
    current_frame: usize,
    last_frame_time: Instant,
    path: PathBuf,
    pub paused: bool,
    total_frames: usize, // 总帧数（已解码 + 待解码）
    /// GPU 纹理缓存：仅保存当前正在显示的帧，切换时丢弃旧纹理
    cached_tex: Option<(usize, egui::TextureHandle)>,
}

impl GifAnimator {
    /// 判断文件是否为 GIF
    pub fn is_gif(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase() == "gif")
            .unwrap_or(false)
    }

    /// 从文件加载 GIF（解码为 CPU 端 ColorImage，不上传 GPU）
    /// GPU 纹理在首次渲染时按需上传
    pub fn load(path: &Path) -> Option<Self> {
        use image::codecs::gif::GifDecoder;
        use image::AnimationDecoder;

        let file = std::fs::File::open(path).ok()?;
        let decoder = GifDecoder::new(std::io::BufReader::new(file)).ok()?;
        let frames_data: Vec<_> = decoder.into_frames().collect();
        let total_raw = frames_data.len();

        let mut frames = Vec::new();
        let mut pending_frames = Vec::new();

        // 如果帧数超过限制，均匀采样
        let sampling = if total_raw > MAX_GIF_FRAMES {
            total_raw / MAX_GIF_FRAMES
        } else {
            1
        };

        for (count, frame_result) in frames_data.into_iter().enumerate() {
            // 采样：只保留每 sampling 帧中的一帧
            if count % sampling != 0 {
                continue;
            }

            let frame = match frame_result.ok() {
                Some(f) => f,
                None => break,
            };
            let (numerator, denominator) = frame.delay().numer_denom_ms();
            let delay_ms = if denominator > 0 {
                (numerator as u64) / (denominator as u64)
            } else {
                100
            };
            let delay = Duration::from_millis(delay_ms.max(20) * sampling as u64);
            let rgba = frame.into_buffer();
            let (w, h) = (rgba.width(), rgba.height());

            if count < GIF_INITIAL_FRAMES * sampling {
                // 立即解码为 ColorImage（不上传 GPU）
                frames.push(GifFrame {
                    color_image: egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw()),
                    delay,
                });
            } else {
                // 存储原始数据，延迟解码
                pending_frames.push(GifRawFrame {
                    rgba: rgba.into_raw(),
                    width: w,
                    height: h,
                    delay,
                });
            }

            if frames.len() + pending_frames.len() >= MAX_GIF_FRAMES {
                break;
            }
        }

        if frames.is_empty() {
            return None;
        }

        let total_frames = frames.len() + pending_frames.len();

        Some(Self {
            frames,
            pending_frames,
            current_frame: 0,
            last_frame_time: Instant::now(),
            path: path.to_path_buf(),
            paused: false,
            total_frames,
            cached_tex: None,
        })
    }

    /// 确保当前帧的 GPU 纹理已缓存
    fn ensure_tex_cached(&mut self, ctx: &egui::Context) {
        match &self.cached_tex {
            Some((idx, _)) if *idx == self.current_frame => return,
            _ => {}
        }
        let frame = &self.frames[self.current_frame];
        let name = format!("gif_{}_{}", self.path.to_string_lossy(), self.current_frame);
        let tex = ctx.load_texture(name, frame.color_image.clone(), Default::default());
        self.cached_tex = Some((self.current_frame, tex));
    }

    /// 获取当前帧纹理（自动推进动画，按需解码待处理帧）
    /// 仅缓存当前帧的 GPU 纹理，切换帧时释放旧纹理
    pub fn current_texture(&mut self, ctx: &egui::Context) -> &egui::TextureHandle {
        if !self.paused {
            let elapsed = self.last_frame_time.elapsed();
            let delay = self.frames[self.current_frame].delay;

            if elapsed >= delay {
                let total = self.frames.len() + self.pending_frames.len();
                let next_frame = (self.current_frame + 1) % total;

                // 如果下一帧还是待解码的，立即解码
                self.decode_frame_if_needed(next_frame);

                self.current_frame = next_frame.min(self.frames.len() - 1);
                self.last_frame_time = Instant::now();
            }
        }

        self.ensure_tex_cached(ctx);
        &self.cached_tex.as_ref().unwrap().1
    }

    /// 按需解码指定帧（仅解码为 ColorImage，不上传 GPU）
    fn decode_frame_if_needed(&mut self, frame_idx: usize) {
        while frame_idx >= self.frames.len() && !self.pending_frames.is_empty() {
            let raw = self.pending_frames.remove(0);
            self.frames.push(GifFrame {
                color_image: egui::ColorImage::from_rgba_unmultiplied(
                    [raw.width as usize, raw.height as usize],
                    &raw.rgba,
                ),
                delay: raw.delay,
            });
        }
    }

    /// 暂停/恢复
    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
        if !self.paused {
            self.last_frame_time = Instant::now();
        }
    }

    /// 暂停（停留在当前帧）
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// 恢复播放（从当前帧继续）
    pub fn play(&mut self) {
        self.paused = false;
        self.last_frame_time = Instant::now();
    }

    /// 上一帧（暂停状态下）
    pub fn prev_frame(&mut self) {
        self.paused = true;
        if self.current_frame > 0 {
            self.current_frame -= 1;
        } else {
            // 回到最后一帧（已解码的）
            self.current_frame = self.frames.len().saturating_sub(1);
        }
    }

    /// 下一帧（暂停状态下，按需解码）
    pub fn next_frame(&mut self) {
        self.paused = true;
        let total = self.frames.len() + self.pending_frames.len();
        let next = (self.current_frame + 1) % total;
        self.decode_frame_if_needed(next);
        self.current_frame = next.min(self.frames.len() - 1);
    }

    /// 当前帧编号（1-based）
    pub fn current_frame_number(&self) -> usize {
        self.current_frame + 1
    }

    /// 总帧数
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }

    /// 获取当前帧尺寸
    pub fn size_vec2(&self) -> egui::Vec2 {
        let img = &self.frames[self.current_frame].color_image;
        egui::Vec2::new(img.width() as f32, img.height() as f32)
    }

    /// 是否匹配指定路径
    pub fn matches(&self, path: &Path) -> bool {
        self.path == path
    }

    /// 当前帧剩余延时
    pub fn current_frame_delay(&self) -> Duration {
        let elapsed = self.last_frame_time.elapsed();
        let delay = self.frames[self.current_frame].delay;
        if elapsed >= delay {
            Duration::from_millis(1)
        } else {
            delay - elapsed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// 创建唯一的临时测试目录
    fn setup_test_dir() -> std::io::Result<PathBuf> {
        let id = TEST_COUNTER.fetch_add(1, AtomicOrdering::SeqCst);
        let temp = std::env::temp_dir().join(format!("jinn_test_{}", id));
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
    fn test_scan_folder_empty() {
        let temp = setup_test_dir().unwrap();
        let entries = scan_folder(&temp);
        assert_eq!(entries.len(), 0, "空目录应返回0个条目");
    }

    #[test]
    fn test_scan_folder_with_images() {
        let temp = setup_test_dir().unwrap();
        create_test_image(&temp.join("a.png"), image::ImageFormat::Png).unwrap();
        create_test_image(&temp.join("b.jpg"), image::ImageFormat::Jpeg).unwrap();
        create_test_image(&temp.join("c.webp"), image::ImageFormat::WebP).unwrap();

        let entries = scan_folder(&temp);
        assert_eq!(entries.len(), 3, "应识别3个图片文件");
        assert!(entries.iter().any(|e| e.name == "a.png"));
        assert!(entries.iter().any(|e| e.name == "b.jpg"));
        assert!(entries.iter().any(|e| e.name == "c.webp"));
    }

    #[test]
    fn test_scan_folder_ignores_non_images() {
        let temp = setup_test_dir().unwrap();
        create_test_image(&temp.join("valid.png"), image::ImageFormat::Png).unwrap();
        fs::write(temp.join("invalid.txt"), b"not an image").unwrap();
        fs::write(temp.join("invalid.exe"), b"not an image").unwrap();

        let entries = scan_folder(&temp);
        assert_eq!(entries.len(), 1, "应只识别图片文件");
        assert_eq!(entries[0].name, "valid.png");
    }

    #[test]
    fn test_scan_folder_large_directory() {
        let temp = setup_test_dir().unwrap();
        // 创建100个测试图片
        for i in 0..100 {
            let name = format!("img_{:03}.png", i);
            create_test_image(&temp.join(&name), image::ImageFormat::Png).unwrap();
        }

        let entries = scan_folder(&temp);
        assert_eq!(entries.len(), 100, "应识别100个图片");
    }

    #[test]
    fn test_scan_folder_nonexistent() {
        let entries = scan_folder(Path::new("/nonexistent_path_12345"));
        assert_eq!(entries.len(), 0, "不存在的路径应返回空列表");
    }

    #[test]
    fn test_fit_size_no_upscale() {
        let tex = egui::Vec2::new(100.0, 100.0);
        let viewport = egui::Vec2::new(200.0, 200.0);
        let result = fit_size(tex, viewport);
        assert_eq!(result, tex, "小图不应放大");
    }

    #[test]
    fn test_fit_size_downscale() {
        let tex = egui::Vec2::new(200.0, 100.0);
        let viewport = egui::Vec2::new(100.0, 100.0);
        let result = fit_size(tex, viewport);
        assert_eq!(result, egui::Vec2::new(100.0, 50.0), "应等比缩小");
    }

    #[test]
    fn test_fit_size_zero_input() {
        let tex = egui::Vec2::new(0.0, 0.0);
        let viewport = egui::Vec2::new(100.0, 100.0);
        let result = fit_size(tex, viewport);
        assert_eq!(result, tex, "零尺寸应原样返回");
    }

    #[test]
    fn test_resize_for_target_preserves_aspect_ratio() {
        let image = DynamicImage::new_rgb8(400, 200);
        let resized = resize_for_target(image, 100);
        assert_eq!(resized.dimensions(), (100, 50));
    }

    #[test]
    fn test_resize_for_target_does_not_upscale() {
        let image = DynamicImage::new_rgb8(40, 20);
        let resized = resize_for_target(image, 100);
        assert_eq!(resized.dimensions(), (40, 20));
    }

    #[test]
    fn test_fit_size_to_max_aspect_ratio() {
        let tex = egui::Vec2::new(400.0, 200.0); // 2:1
        let max_size = egui::Vec2::new(100.0, 100.0);
        let result = fit_size_to_max(tex, max_size);
        assert_eq!(result, egui::Vec2::new(100.0, 50.0), "应保持宽高比");
    }

    #[test]
    fn test_image_entry_default_rotation() {
        let temp = setup_test_dir().unwrap();
        create_test_image(&temp.join("test.png"), image::ImageFormat::Png).unwrap();
        let entries = scan_folder(&temp);
        assert_eq!(entries[0].manual_rotation, 0, "新图片旋转角度应为0");
    }
}
