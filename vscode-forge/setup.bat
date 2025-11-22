@echo off
REM DX Forge VS Code Extension - Setup Script for Windows

echo 🚀 Setting up DX Forge Watcher Extension...
echo.

cd /d "%~dp0"

REM Check if npm is installed
where npm >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ npm is not installed. Please install Node.js first.
    exit /b 1
)

echo 📦 Installing dependencies...
call npm install

echo 🔨 Compiling TypeScript...
call npm run compile

echo.
echo ✅ Setup complete!
echo.
echo Next steps:
echo   1. Press F5 in VS Code to run the extension in debug mode
echo   2. Or package it: npm install -g @vscode/vsce ^&^& vsce package
echo   3. Install the .vsix file in VS Code
echo.
echo The extension will:
echo   ✨ Monitor file changes in real-time
echo   ⏱️  Show detailed timing information
echo   🎨 Display beautiful formatted output
echo   📊 Track operation history
echo.
pause
