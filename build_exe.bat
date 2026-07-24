@echo off
chcp 65001 >nul 2>&1
setlocal EnableDelayedExpansion
title Jinn图片查看器 编译脚本
cd /d "%~dp0"

echo.
echo ============================================
echo Jinn图片查看器 - 一键编译脚本
echo ============================================
echo.

set "LOG_FILE=%~dp0编译日志.txt"
echo [%date% %time%] 开始编译 > "%LOG_FILE%"

:: ---- 步骤1: 检查 Rust 环境 ----
echo [1/5] 检查 Rust 环境...
where cargo >nul 2>&1
if !errorlevel! neq 0 goto :err_cargo

for /f "tokens=*" %%v in ('rustc --version 2^>nul') do set "RUSTC_VER=%%v"
for /f "tokens=*" %%v in ('cargo --version 2^>nul') do set "CARGO_VER=%%v"
echo rustc: !RUSTC_VER!
echo cargo: !CARGO_VER!
echo rustc: !RUSTC_VER! >> "%LOG_FILE%"
echo cargo: !CARGO_VER! >> "%LOG_FILE%"

:: ---- 步骤2: 检查 MSVC 工具链 ----
echo [2/5] 检查 MSVC 工具链...
where cl.exe >nul 2>&1
if !errorlevel! equ 0 goto :cl_found

echo cl.exe 不在 PATH 中，尝试查找 Visual Studio...
set "VS_BASE="
if exist "E:\DevTools\VisualStudio\VC\Tools\MSVC" set "VS_BASE=E:\DevTools\VisualStudio\VC\Tools\MSVC"
if not defined VS_BASE if exist "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC" set "VS_BASE=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC"
if not defined VS_BASE if exist "C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC" set "VS_BASE=C:\Program Files\Microsoft Visual Studio\2022\Professional\VC\Tools\MSVC"
if not defined VS_BASE if exist "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC" set "VS_BASE=C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC"
if not defined VS_BASE if exist "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Tools\MSVC" set "VS_BASE=C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Tools\MSVC"

if not defined VS_BASE goto :vs_not_found

set "MSVC_PATH="
for /f "tokens=*" %%d in ('dir /b /ad /o-n "%VS_BASE%" 2^>nul') do (
    set "MSVC_PATH=%VS_BASE%\%%d"
    goto :found_msvc_dir
)
:found_msvc_dir
if defined MSVC_PATH (
    echo 找到 MSVC: !MSVC_PATH!
    echo 找到 MSVC: !MSVC_PATH! >> "%LOG_FILE%"
)
goto :step3

:cl_found
echo cl.exe 已在 PATH 中
echo cl.exe 已在 PATH 中 >> "%LOG_FILE%"
goto :step3

:vs_not_found
echo [警告] 未找到 Visual Studio C++ 工具链。
echo [警告] cargo build 可能会失败。如果编译报错，请安装 Visual Studio 的 "C++ 桌面开发" 工作负载。
echo [警告] 未找到 Visual Studio C++ 工具链 >> "%LOG_FILE%"

:step3
:: ---- 步骤3: 检查项目文件 ----
echo [3/5] 检查项目文件...
if not exist "%~dp0Cargo.toml" goto :err_cargo_toml
if not exist "%~dp0src\main.rs" goto :err_main_rs
if not exist "%~dp0build.rs" goto :err_build_rs

echo Cargo.toml √
echo src/main.rs √
echo build.rs √

if exist "%~dp0app_icon.ico" goto :icon_ok
echo app_icon.ico ×
echo [警告] 未找到 app_icon.ico，exe 将没有自定义图标。 >> "%LOG_FILE%"
goto :step4

:icon_ok
echo app_icon.ico √

:step4
:: ---- 步骤4: 编译 ----
echo [4/5] 开始编译 (cargo build --release)...
echo 首次编译可能需要 5-10 分钟，请耐心等待...
echo 编译详细输出见: 编译日志.txt
echo.
echo [%date% %time%] cargo build --release 开始 >> "%LOG_FILE%"

cargo build --release --manifest-path "%~dp0Cargo.toml" >> "%LOG_FILE%" 2>&1
set BUILD_RESULT=!errorlevel!

echo [%date% %time%] cargo build --release 结束，退出码: !BUILD_RESULT! >> "%LOG_FILE%"

if !BUILD_RESULT! neq 0 goto :err_build

:: ---- 步骤5: 复制exe并显示结果 ----
echo [5/5] 编译成功！复制 exe 文件...
set "EXE_SRC=%~dp0target\release\jinn-imageviewer.exe"
set "EXE_DST=%~dp0Jinn图片查看器.exe"

if not exist "%EXE_SRC%" goto :no_exe

copy /y "%EXE_SRC%" "%EXE_DST%" >nul 2>&1
for %%f in ("%EXE_DST%") do set EXE_SIZE=%%~zf

echo.
echo ============================================
echo 编译成功！
echo ============================================
echo.
echo 输出文件: %EXE_DST%
echo 文件大小: !EXE_SIZE! 字节
echo.
echo 双击 Jinn图片查看器.exe 即可运行。
echo 编译成功！输出: %EXE_DST% (!EXE_SIZE! 字节) >> "%LOG_FILE%"
goto :finish

:no_exe
echo [警告] 编译成功但未找到输出 exe: %EXE_SRC%
echo [警告] 未找到输出 exe >> "%LOG_FILE%"
goto :finish

:err_cargo
echo [错误] 未找到 cargo！请确认 Rust 已安装并添加到 PATH。
echo [错误] 未找到 cargo！ >> "%LOG_FILE%"
goto :error_pause

:err_cargo_toml
echo [错误] 未找到 Cargo.toml！请确认在正确的目录运行此脚本。
echo [错误] 未找到 Cargo.toml！ >> "%LOG_FILE%"
goto :error_pause

:err_main_rs
echo [错误] 未找到 src\main.rs！
echo [错误] 未找到 src\main.rs！ >> "%LOG_FILE%"
goto :error_pause

:err_build_rs
echo [错误] 未找到 build.rs！
echo [错误] 未找到 build.rs！ >> "%LOG_FILE%"
goto :error_pause

:err_build
echo.
echo [错误] 编译失败！退出码: !BUILD_RESULT!
echo [错误] 编译失败！退出码: !BUILD_RESULT! >> "%LOG_FILE%"
echo.
echo 请查看 编译日志.txt 获取详细错误信息。
echo.
echo 常见问题：
echo 1. 未安装 Visual Studio C++ 桌面开发工作负载
echo 2. Rust 工具链不是 msvc 版本（运行 rustup default stable-x86_64-pc-windows-msvc）
echo 3. 网络问题导致依赖下载失败（重新运行此脚本即可）
goto :error_pause

:error_pause
echo [%date% %time%] 编译流程异常结束 >> "%LOG_FILE%"
echo.
echo 脚本已停止，请按任意键退出...
pause >nul
exit /b 1

:finish
echo [%date% %time%] 编译流程结束 >> "%LOG_FILE%"
echo.
echo 脚本执行完毕，请按任意键退出...
pause >nul
exit /b 0
