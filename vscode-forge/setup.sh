#!/bin/bash

# DX Forge VS Code Extension - Setup Script

echo "🚀 Setting up DX Forge Watcher Extension..."
echo ""

cd "$(dirname "$0")"

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo "❌ npm is not installed. Please install Node.js first."
    exit 1
fi

echo "📦 Installing dependencies..."
npm install

echo "🔨 Compiling TypeScript..."
npm run compile

echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "  1. Press F5 in VS Code to run the extension in debug mode"
echo "  2. Or package it: npm install -g @vscode/vsce && vsce package"
echo "  3. Install the .vsix file in VS Code"
echo ""
echo "The extension will:"
echo "  ✨ Monitor file changes in real-time"
echo "  ⏱️  Show detailed timing information"
echo "  🎨 Display beautiful formatted output"
echo "  📊 Track operation history"
echo ""
