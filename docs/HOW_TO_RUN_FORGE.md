# How to Run Forge Binary

## 🚀 Quick Start

The **Forge LSP Extension** now automatically runs the Forge binary when it detects it!

### Prerequisites

1. **Build Forge Binary** (if not already built):
   ```bash
   cargo build --release
   ```
   This creates: `target/release/forge-cli.exe`

2. **Install VS Code Extension**:
   ```bash
   code --install-extension vscode-forge/forge-lsp-0.0.1.vsix
   ```

### Auto-Start Behavior

✅ **The extension will automatically:**
1. Detect the Forge binary at startup
2. Run `forge watch` in the background
3. Display all output in the "Forge LSP" output panel
4. Monitor file changes in your workspace
5. Stream `logs/forge.log` to the output panel

### Manual Commands

#### Run Forge Binary Directly

```bash
# Default: Start watching current directory
./target/release/forge-cli.exe

# Or explicitly use watch command
./target/release/forge-cli.exe watch

# Watch with sync enabled
./target/release/forge-cli.exe watch --sync

# Watch with peer connections
./target/release/forge-cli.exe watch --peer ws://localhost:3000/ws

# Initialize a new Forge repository
./target/release/forge-cli.exe init

# View operation log
./target/release/forge-cli.exe oplog

# Create an anchor
./target/release/forge-cli.exe anchor <file> <line> <column> -m "message"

# Show context
./target/release/forge-cli.exe context <file>
```

### VS Code Extension Usage

1. **Open Workspace**: Open the Forge repository in VS Code
2. **Extension Auto-Starts**: If configured (default: `forge.autoStart: true`)
3. **View Output**: Open "View → Output" and select "Forge LSP"

**Extension Commands:**
- `Forge: Start` - Start watching (if not auto-started)
- `Forge: Stop` - Stop the Forge binary and watcher
- `Forge: Clear Output` - Clear the output panel
- `Forge: Show AST for Current File` - Analyze current file

### What Happens When Extension Starts

```
═══════════════════════════════════════════════════════════
  🚀 DX FORGE LSP
  HH:MM:SS
═══════════════════════════════════════════════════════════

[HH:MM:SS.mmm] ℹ️  👁️  Starting file system watcher...

[HH:MM:SS.mmm] ℹ️  🚀 Starting Forge binary...
[HH:MM:SS.mmm] ℹ️     Binary: F:\Code\forge\target\release\forge-cli.exe
[HH:MM:SS.mmm] ℹ️     Working Dir: F:\Code\forge

[HH:MM:SS.mmm] ✅ Forge binary started successfully
[HH:MM:SS.mmm] ℹ️     Process PID: 12345
────────────────────────────────────────────────────────────

[HH:MM:SS.mmm] ℹ️  📋 Watching forge.log for Forge binary output
────────────────────────────────────────────────────────────

[HH:MM:SS.mmm] ✅ Forge LSP watcher active

Monitoring all file changes in workspace...
Changes will be displayed below:
────────────────────────────────────────────────────────────
```

### Output Channels

The extension displays THREE types of output:

1. **🔵 Forge Output** - stdout from `forge watch` command
2. **🔴 Forge Error** - stderr from `forge watch` command  
3. **📋 Forge Binary Log** - Content from `logs/forge.log` file

All ANSI escape codes are stripped for clean display.

### Configuration

Edit `.vscode/settings.json`:

```json
{
  "forge.autoStart": true,        // Auto-start on workspace open
  "forge.showDiffs": true         // Show file content previews
}
```

### Troubleshooting

**Extension doesn't start?**
- Check: Is `target/release/forge-cli.exe` present?
- Build: Run `cargo build --release`

**No output in panel?**
- Open: View → Output
- Select: "Forge LSP" from dropdown

**Forge binary not found?**
- Extension checks these paths (in order):
  1. `target/release/forge-cli.exe`
  2. `target/release/forge-cli`
  3. `target/debug/forge-cli.exe`
  4. `target/debug/forge-cli`

**Want to rebuild?**
```bash
# Rebuild Forge binary
cargo build --release

# Rebuild extension
cd vscode-forge
npm run compile
vsce package --allow-missing-repository --no-dependencies --skip-license

# Reinstall
code --install-extension vscode-forge/forge-lsp-0.0.1.vsix
```

### Process Management

- **Start Extension**: Forge binary runs automatically
- **Stop Extension**: `Forge: Stop` command kills the process
- **Process ID**: Displayed in output panel when started
- **Auto-Restart**: Reload window (Ctrl+R) to restart everything

### Log Files

- **forge.log**: `logs/forge.log` in workspace root
- **Format**: ISO 8601 timestamps, structured logging
- **Viewing**: Automatically tailed and displayed in VS Code output
- **Manual**: Open `logs/forge.log` in editor

### Example Session

```bash
# 1. Build Forge
cargo build --release

# 2. Install extension
code --install-extension vscode-forge/forge-lsp-0.0.1.vsix

# 3. Open workspace
code .

# 4. Extension auto-starts Forge binary!
# Check "Forge LSP" output panel to see:
#   - Forge binary starting
#   - File change detection
#   - forge.log streaming
#   - AST analysis results
```

### Advanced Usage

**Run Forge with custom arguments** (outside extension):
```bash
# Watch specific directory
./target/release/forge-cli.exe watch --path ./src

# Enable sync mode
./target/release/forge-cli.exe watch --sync --peer ws://localhost:3000/ws

# Time travel
./target/release/forge-cli.exe time-travel src/main.rs --timestamp "2025-11-15T10:00:00Z"

# Component management
./target/release/forge-cli.exe components --verbose
./target/release/forge-cli.exe register Button.tsx --source dx-ui --name Button --version 1.0.0
./target/release/forge-cli.exe update all
```

### Git Integration

Forge supports Git commands without the `git` prefix:

```bash
# These are equivalent:
git status          ←→  forge status
git commit          ←→  forge commit
git push            ←→  forge push
```

---

## 🎯 Summary

✅ **Extension auto-runs `forge watch` when it starts**  
✅ **All output appears in VS Code "Forge LSP" panel**  
✅ **Process runs in background, managed automatically**  
✅ **Logs streamed from `logs/forge.log` in real-time**  
✅ **No manual command needed - just open VS Code!**
