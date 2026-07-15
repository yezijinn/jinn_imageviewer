# Jinn图片查看器 Visual Studio 编译指南
## 环境准备
### 1. 安装 Visual Studio
确保已安装 Visual Studio 2019 或更高版本，并包含以下工作负载：
- 使用 C++ 的桌面开发
- .NET 桌面开发（可选，用于某些依赖）
### 2. 安装 Rust 工具链
1. 下载并安装 Rust：[https://rustup.rs/](https://rustup.rs/)
2. 打开命令提示符并运行：
```bash
rustup update
rustup default stable
```
### 3. 安装 C++ 构建工具
```bash
rustup component add rust-src
```
## 项目设置
### 1. 克隆项目
```bash
git clone https://github.com/yezjinn/jinn_imageviewer.git
cd jinn_imageviewer
```
### 2. 创建 Visual Studio 项目文件
在项目根目录运行：
```bash
cargo install cargo-msvc
cargo msvc --target x86_64-pc-windows-msvc
```
## 依赖管理
### 1. 编辑 Cargo.toml
确保 `Cargo.toml` 包含以下依赖：
```toml
[package]
name = "jinn-imageviewer"
version = "0.1.0"
edition = "2021"
[dependencies]
eframe = "0.27.2"
egui = "0.27.2"
image = "0.24.7"
rfd = "0.12.1"
open = "5.0"
[build-dependencies]
embed-resource = "2.4.2"
```
### 2. 安装依赖
```bash
cargo build --release
```
## 编译步骤
### 1. 打开解决方案
1. 在 Visual Studio 中打开项目文件夹
2. 选择 `jinn-imageviewer.sln` 文件
### 2. 配置项目属性
1. 右键点击项目 → 属性
2. 配置以下属性：
#### C/C++ → 常规
- **SDL 检查**：是（/sdl）
#### 链接器 → 输入
- **附加依赖项**：
  ```
  ole32.lib
  user32.lib
  gdi32.lib
  opengl32.lib
  ```
#### 链接器 → 系统
- **子系统**：Windows (/SUBSYSTEM:WINDOWS)
#### 链接器 → 优化
- **引用优化**：是 (/OPT:REF)
### 3. 构建项目
1. 选择 "Release" 配置
2. 选择 "x64" 平台
3. 点击 "生成" → "生成解决方案"
## 调试和运行
### 1. 运行项目
1. 设置启动项目为 `jinn-imageviewer`
2. 按 F5 或点击 "本地 Windows 调试器"
### 2. 调试配置
1. 在项目属性中配置调试信息
2. 使用 Visual Studio 调试器进行断点调试
## 常见问题解决
### 1. 编译错误：缺少依赖
```bash
cargo update
cargo build --release
```
### 2. 链接错误：找不到库
确保已安装 Windows SDK，并在 Visual Studio 安装程序中勾选 "使用 C++ 的桌面开发" 工作负载。
### 3. Unicode 编码问题
确保源文件使用 UTF-8 编码，可以在 Visual Studio 中通过 "文件" → "高级保存选项" 检查编码。
### 4. 性能优化
在 Release 配置下编译以获得最佳性能：
```bash
cargo build --release
```
## 高级配置
### 1. 自定义图标
1. 准备 256x256 PNG 图标
2. 在 `build.rs` 中添加图标嵌入代码：
```rust
fn main() {
    embed_resource::compile("icons/PNG/icon_256.png", embed_resource::Type::Icon, "icon_256");
}
```
### 2. 多语言支持
在 `setup_chinese_fonts` 函数中添加其他字体支持。
## 发布准备
### 1. 创建发布版本
```bash
cargo build --release
```
### 2. 查找可执行文件
发布版本的可执行文件位于：
```
target\release\jinn-imageviewer.exe
```
### 3. 打包应用
可以使用 Inno Setup 或 NSIS 创建安装程序。
## 贡献指南
1. Fork 项目
2. 创建功能分支
3. 提交更改
4. 创建 Pull Request
## 联系方式
- 项目主页：https://github.com/yezjinn/jinn_imageviewer
- 作者：叶子Jinn
---
**注意**：本指南适用于 Windows 平台，如需在其他平台编译，请参考相应的平台特定指南。
