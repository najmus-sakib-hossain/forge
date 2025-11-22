# 🚀 DX Forge Watcher - Quick Start Guide

Beautiful real-time visualization of Forge VCS operations in VS Code!

## Installation & Setup

### Option 1: Quick Setup (Recommended)

**Windows:**
```bash
cd vscode-forge
setup.bat
```

**Linux/Mac:**
```bash
cd vscode-forge
chmod +x setup.sh
./setup.sh
```

### Option 2: Manual Setup

```bash
cd vscode-forge
npm install
npm run compile
```

## Running the Extension

### Development Mode (Recommended for testing)

1. Open the `vscode-forge` folder in VS Code
2. Press `F5` to launch a new VS Code window with the extension loaded
3. The extension will auto-activate if you have a Forge repository open

### Install as VSIX Package

```bash
# Install vsce globally
npm install -g @vscode/vsce

# Package the extension
vsce package

# Install the generated .vsix file
code --install-extension forge-watcher-0.0.1.vsix
```

## Usage

### 1. Open a Forge Repository

The extension automatically detects if your workspace has `.dx/forge` directory.

### 2. Start Watching

- **Automatic**: Extension starts automatically if `forge.autoStart` is enabled (default)
- **Manual**: Click the `👁️ Forge` button in the status bar, or
  - Press `Ctrl+Shift+P` (Windows/Linux) or `Cmd+Shift+P` (Mac)
  - Type "Forge: Start Watching"

### 3. View Operations

Operations appear in the **"Forge Operations"** output panel:

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
```

## Features

### ✨ Real-time Monitoring
Watch every change as it happens with ultra-fast detection

### ⏱️ Detailed Timing
See exact timestamps, durations, and relative times

### 🎨 Beautiful Output
Color-coded, emoji-enhanced logs that are easy to read

### 📊 Operation Types
- ➕ **INSERT** - New content added
- ➖ **DELETE** - Content removed
- 🔄 **REPLACE** - Content modified
- ✨ **CREATED** - File created
- 📝 **MODIFIED** - File modified
- 🗑️ **DELETED** - File deleted
- 📋 **RENAMED** - File renamed

### 🎯 Content Diffs
See exactly what changed:
```
   + console.log('New line');
   - console.log('Old line');
```

## Commands

| Command | Description | Shortcut |
|---------|-------------|----------|
| `Forge: Start Watching` | Start monitoring operations | Click status bar |
| `Forge: Stop Watching` | Stop the watcher | Click status bar |
| `Forge: Clear Output` | Clear the output panel | - |
| `Forge: Show Operation History` | Show recent operations | - |

## Configuration

Open VS Code Settings (`Ctrl+,` or `Cmd+,`) and search for "Forge":

```json
{
  "forge.autoStart": true,           // Auto-start on workspace open
  "forge.showTimestamps": true,      // Show timestamps in logs
  "forge.showDuration": true,        // Show operation duration
  "forge.showDiffs": true,           // Show content diffs
  "forge.colorizeOutput": true       // Use colors in output
}
```

## Troubleshooting

### Extension Not Activating

**Problem**: Extension doesn't start automatically

**Solution**:
1. Ensure your workspace has `.dx/forge` directory
2. Check if `forge.autoStart` is enabled in settings
3. Manually start with `Forge: Start Watching` command

### No Operations Showing

**Problem**: Output panel is empty

**Solution**:
1. Make sure Forge watcher is running (status bar shows "Active")
2. Try making a change to a file in your workspace
3. Check that the file is not in an ignored directory (`.dx`, `node_modules`, etc.)

### Permission Errors

**Problem**: Cannot watch files

**Solution**:
1. Ensure you have read permissions for the workspace
2. Check that `.dx/forge` directory exists and is writable

## Development

### Project Structure

```
vscode-forge/
├── src/
│   ├── extension.ts        # Main extension entry point
│   ├── forgeWatcher.ts     # File watcher and operation detector
│   ├── outputFormatter.ts  # Beautiful output formatting
│   └── types.ts           # TypeScript type definitions
├── package.json           # Extension manifest
├── tsconfig.json          # TypeScript configuration
└── README.md              # Documentation
```

### Building from Source

```bash
# Install dependencies
npm install

# Compile TypeScript
npm run compile

# Watch for changes (auto-compile)
npm run watch

# Run in development mode
# Press F5 in VS Code
```

### Testing

1. Open `vscode-forge` in VS Code
2. Press `F5` to launch Extension Development Host
3. Open a Forge repository in the new window
4. Make changes to files and observe the output

## Examples

### Tracking a New Feature

1. Start the watcher
2. Create a new file: `touch src/newFeature.ts`
3. See in output:
   ```
   ✨ CREATED │ 14:30:12.456 (just now)
      📄 newFeature.ts
      📂 src/newFeature.ts
   ```

4. Add code:
   ```typescript
   export function newFeature() {
     return "Hello, Forge!";
   }
   ```

5. See in output:
   ```
   ➕ INSERT │ 14:30:15.789 (3s ago)
      📄 newFeature.ts
      📂 src/newFeature.ts
      📍 Line 1, Column 1

      + export function newFeature() {
      +   return "Hello, Forge!";
      + }
   ```

### Monitoring Multiple Files

The extension tracks all changes across your workspace:

```
📝 MODIFIED │ 14:32:01.123 (5s ago)
   📄 config.json
   📂 .dx/forge/config.json

✨ CREATED │ 14:32:03.456 (3s ago)
   📄 utils.ts
   📂 src/lib/utils.ts

➕ INSERT │ 14:32:05.789 (just now)
   📄 main.ts
   📂 src/main.ts
   📍 Line 10, Column 5
```

## What Makes This Extension Special?

### 🎯 Production-Ready Logging
Inspired by enterprise monitoring tools, every operation is logged with:
- Microsecond-precision timestamps
- Relative time indicators ("just now", "3s ago")
- Duration measurements
- Actor identification
- Full file context

### 🚀 Performance
- Non-blocking file watching
- Efficient debouncing
- Minimal memory footprint
- Smart caching

### 🎨 Beautiful UX
- Emoji icons for quick scanning
- Color-coded operations
- Structured, readable format
- Clear visual hierarchy

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Test thoroughly
5. Submit a pull request

## License

MIT OR Apache-2.0

---

**Made with ❤️ for the DX Tools ecosystem**
