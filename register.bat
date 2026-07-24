@echo off
chcp 65001 >nul 2>&1
setlocal EnableDelayedExpansion
title Jinn图片查看器 - 文件关联注册

echo.
echo ============================================
echo   Jinn图片查看器 - 注册文件关联
echo ============================================
echo.
echo 此脚本将注册以下图片格式的默认打开程序：
echo   .png .jpg .jpeg .bmp .gif .webp .tiff .tif
echo.
echo 需要管理员权限运行此脚本！
echo.

:: 检查管理员权限
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 请右键此脚本，选择"以管理员身份运行"
    echo.
    pause
    exit /b 1
)

:: 获取 exe 路径
set "EXE_PATH=%~dp0target\release\jinn-imageviewer.exe"
if not exist "%EXE_PATH%" (
    set "EXE_PATH=%~dp0Jinn图片查看器.exe"
)
if not exist "%EXE_PATH%" (
    echo [错误] 未找到可执行文件，请先编译项目
    echo 尝试查找: %~dp0target\release\jinn-imageviewer.exe
    pause
    exit /b 1
)

echo 可执行文件: %EXE_PATH%
echo.

:: 注册应用程序
echo [1/3] 注册应用程序...
reg add "HKEY_CLASSES_ROOT\Applications\jinn-imageviewer.exe" /ve /d "Jinn Image Viewer" /f >nul 2>&1
reg add "HKEY_CLASSES_ROOT\Applications\jinn-imageviewer.exe\shell\open\command" /ve /d "\"%EXE_PATH%\" \"%%1\"" /f >nul 2>&1
reg add "HKEY_CLASSES_ROOT\Applications\jinn-imageviewer.exe\DefaultIcon" /ve /d "\"%EXE_PATH%\",0" /f >nul 2>&1
echo   完成

:: 注册 ProgID
echo [2/3] 注册文件类型...
reg add "HKEY_CLASSES_ROOT\JinnImageViewer.Image" /ve /d "Jinn Image Viewer" /f >nul 2>&1
reg add "HKEY_CLASSES_ROOT\JinnImageViewer.Image\shell\open\command" /ve /d "\"%EXE_PATH%\" \"%%1\"" /f >nul 2>&1
reg add "HKEY_CLASSES_ROOT\JinnImageViewer.Image\DefaultIcon" /ve /d "\"%EXE_PATH%\",0" /f >nul 2>&1
echo   完成

:: 关联文件扩展名
echo [3/3] 关联图片格式...
for %%e in (.png .jpg .jpeg .bmp .gif .webp .tiff .tif) do (
    reg add "HKEY_CLASSES_ROOT\%%e\OpenWithProgids" /v "JinnImageViewer.Image" /t REG_NONE /f >nul 2>&1
    echo   %%e 已关联
)

echo.
echo ============================================
echo   注册完成！
echo ============================================
echo.
echo 现在可以：
echo   1. 右键图片 → 打开方式 → 选择 "Jinn Image Viewer"
echo   2. 设置为默认程序后，双击图片即可直接打开
echo.
echo 如需设为默认程序：
echo   Windows 设置 → 应用 → 默认应用 → 按文件类型选择默认应用
echo.
pause
exit /b 0
