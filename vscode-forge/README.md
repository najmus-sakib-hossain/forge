# DX Forge LSP - Real-Time File Change Monitor

A **simple and powerful** VS Code extension that detects ALL file changes in your workspace and displays them beautifully with detailed timing information and content previews.

## ✨ What It Does

- 👁️ **Watches ALL files** in your workspace automatically
- 📊 **Shows file content** and AST-like structure  
- ⏱️ **Displays precise timing** (microsecond timestamps)
- 🎨 **Beautiful output** with emojis and formatting
- 🚀 **Zero configuration** - just install and use!

## Usage

### Commands

- **Forge: Start Watching** - Begin monitoring Forge operations
- **Forge: Stop Watching** - Stop the watcher
- **Forge: Clear Output** - Clear the output panel
- **Forge: Show Operation History** - Display recent operations

### Status Bar

The status bar shows the current Forge watcher status:
- 👁️ **Forge** - Inactive (click to start)
- 👁️ **Forge: Active** - Watching for changes (click to stop)

## Configuration

Access via File → Preferences → Settings → Extensions → DX Forge Watcher

- `forge.autoStart` - Automatically start watching when opening a Forge repository (default: true)
- `forge.showTimestamps` - Show timestamps in operation logs (default: true)
- `forge.showDuration` - Show operation duration (default: true)
- `forge.showDiffs` - Show content diffs in output (default: true)
- `forge.colorizeOutput` - Use colors in output panel (default: true)

## Output Format

```
═══════════════════════════════════════════════════════════════════════════════
  DX FORGE WATCHER
  14:23:45.123
═══════════════════════════════════════════════════════════════════════════════

➕ INSERT │ 14:23:47.456 (2s ago)
   📄 main.ts
   📂 src/main.ts
   ⏱️  35.24µs
   👤 vscode-user
   📍 Line 42, Column 10

   + console.log('Hello, Forge!');

────────────────────────────────────────────────────────────────────────────────

📝 MODIFIED │ 14:23:48.789 (1s ago)
   📄 config.json
   📂 .dx/forge/config.json
   ⏱️  1.23ms
```

## Requirements

- VS Code 1.85.0 or higher
- DX Forge initialized in your workspace (`.dx/forge` directory)

## Installation

### From Source

1. Clone the repository
2. `cd vscode-forge`
3. `npm install`
4. `npm run compile`
5. Press F5 to open a new VS Code window with the extension loaded

### From VSIX

1. Package: `vsce package`
2. Install: `code --install-extension forge-watcher-0.0.1.vsix`

## How It Works

The extension monitors:
1. **File System Changes** - Watches workspace files for modifications
2. **Forge Database** - Monitors `.dx/forge/forge.db` for operation logs
3. **VS Code Events** - Integrates with VS Code's file system watcher

All changes are beautifully formatted and displayed in the "Forge Operations" output channel.

## Development

```bash
# Install dependencies
npm install

# Compile TypeScript
npm run compile

# Watch for changes
npm run watch

# Run extension
Press F5 in VS Code
```

## License

MIT OR Apache-2.0
