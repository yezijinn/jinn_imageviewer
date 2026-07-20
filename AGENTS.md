# AGENTS.md — Jinn图片查看器

## 项目概览

- **类型**: 单文件 Rust 桌面 GUI 应用（Windows-only）
- **框架**: egui via `eframe` 0.31
- **源码**: 全部在 `src/main.rs`（~1000 行），无 crate 拆分
- **构建产物**: `target/release/jinn-imageviewer.exe`
- **图片格式**: 支持 PNG/JPG/JPEG/BMP/GIF/WebP/TIFF（由 `image` crate 决定）

## 构建

```bash
# 标准 release 构建（Windows + MSVC 工具链必需）
cargo build --release

# 构建后 exe 位置
target/release/jinn-imageviewer.exe
```

**平台约束**:
- Windows-only（`#![windows_subsystem = "windows"]` 隐藏控制台）
- 需要 MSVC 工具链（`stable-x86_64-pc-windows-msvc`）
- `build.rs` 在 Windows 上嵌入 `app_icon.ico` 为 exe 图标（依赖 `winres`）

## 开发注意事项

- **单文件架构**: 所有逻辑集中在 `src/main.rs`，修改时无需考虑模块边界
- **中文字体加载**: 运行时从 `C:\Windows\Fonts\` 加载（simhei / simsunb / simkai / simfang / msyh），Windows 外运行会静默跳过
- **FFI 依赖**: 使用 `dwmapi` 设置暗黑标题栏，仅 Windows 有效
- **文件操作**: Delete 键直接删除图片（无确认弹窗）；复制到 exe 同级 `copy/` 目录

## 快捷操作（运行时）

| 操作 | 按键 |
|------|------|
| 打开文件夹 | O |
| 上一张 / 下一张 | ←↑ / →↓ |
| 删除图片 | Delete |
| 双列模式 | F2 |
| 复制当前 / 右侧图片 | 1 / 2 |
| 适应窗口 | V / 右键 |
| 关于 | F1 |
| 退出 | Esc |

- 快捷键均可通过设置面板自定义（含修饰键）
- 滚轮缩放，缩放后自动退出"适应窗口"模式

## 目录结构

```
jinn_imageviewer/
├── src/main.rs        # 全部源码
├── Cargo.toml         # 依赖: eframe, image, rfd, raw-window-handle, open, winres
├── build.rs           # 嵌入 Windows 图标资源
├── app_icon.ico       # 应用图标（构建必需）
├── icons/PNG/         # 多尺寸图标源文件
├── python_version/    # Python 版本实现（遗留，不参与 Rust 构建）
└── screenshots/       # 截图素材
```

## 测试

- 无单元测试；验证方式为手动运行并测试 GUI 交互
- 构建成功即可视为可运行状态
