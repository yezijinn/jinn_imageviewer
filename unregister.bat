@echo off
chcp 65001 >nul 2>&1
title Jinn图片查看器 - 取消文件关联

echo.
echo ============================================
echo   Jinn图片查看器 - 取消文件关联
echo ============================================
echo.

:: 检查管理员权限
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo [错误] 请右键此脚本，选择"以管理员身份运行"
    pause
    exit /b 1
)

echo 正在取消文件关联...

:: 移除扩展名关联
for %%e in (.png .jpg .jpeg .bmp .gif .webp .tiff .tif) do (
    reg delete "HKEY_CLASSES_ROOT\%%e\OpenWithProgids" /v "JinnImageViewer.Image" /f >nul 2>&1
    echo   %%e 已取消
)

:: 移除 ProgID
reg delete "HKEY_CLASSES_ROOT\JinnImageViewer.Image" /f >nul 2>&1
reg delete "HKEY_CLASSES_ROOT\Applications\jinn-imageviewer.exe" /f >nul 2>&1

echo.
echo 文件关联已取消。
echo.
pause
exit /b 0
