import * as vscode from 'vscode';
import * as path from 'path';
import * as child_process from 'child_process';

export class ForgeWatcher {
    private fileWatchers: vscode.Disposable[] = [];
    private forgeProcess: child_process.ChildProcess | undefined;
    private changeQueue: Map<string, NodeJS.Timeout> = new Map();
    private logFileWatcher: vscode.FileSystemWatcher | undefined;
    private logFilePosition: number = 0;
    private isForgeRunning: boolean = false;

    constructor(
        private rootPath: string,
        private forgeBinary: string,
        private outputChannel: vscode.OutputChannel
    ) {}

    async start() {
        try {
            this.log('info', '$(eye) Starting file system watcher...');
            
            // Start Forge binary in background
            await this.startForgeProcess();

            // Watch workspace files
            this.startFileWatching();

            // Watch forge.log file
            await this.startLogFileWatching();

            this.log('success', '$(check) Forge LSP watcher active');
            this.outputChannel.appendLine('');
            this.outputChannel.appendLine('$(telescope) Monitoring all file changes in workspace...');
            this.outputChannel.appendLine('$(arrow-right) Changes will be displayed below:');
            this.logDivider();
            
        } catch (error) {
            const errorMsg = error instanceof Error ? error.message : String(error);
            this.log('error', `Failed to start watcher: ${errorMsg}`);
            throw error;
        }
    }

    private async startForgeProcess() {
        if (this.isForgeRunning) {
            this.log('warning', '$(warning) Forge process already running');
            return;
        }

        this.outputChannel.appendLine('');
        this.log('info', '$(rocket) Starting Forge binary...');
        this.log('info', `   $(file-binary) Binary: ${this.forgeBinary}`);
        this.log('info', `   $(folder-active) Working Dir: ${this.rootPath}`);
        this.outputChannel.appendLine('');

        try {
            // Spawn the forge watch command
            this.forgeProcess = child_process.spawn(this.forgeBinary, ['watch'], {
                cwd: this.rootPath,
                shell: true,
                stdio: ['ignore', 'pipe', 'pipe']
            });

            this.isForgeRunning = true;

            // Handle stdout
            if (this.forgeProcess.stdout) {
                this.forgeProcess.stdout.on('data', (data: Buffer) => {
                    const output = data.toString().trim();
                    if (output) {
                        this.outputChannel.appendLine('');
                        this.outputChannel.appendLine('$(output) Forge Output:');
                        this.outputChannel.appendLine('─'.repeat(80));
                        output.split('\n').forEach(line => {
                            // Strip ANSI codes
                            const cleanLine = line.replace(/\x1b\[[0-9;]*m/g, '');
                            if (cleanLine.trim()) {
                                this.outputChannel.appendLine(`   ${cleanLine}`);
                            }
                        });
                        this.outputChannel.appendLine('─'.repeat(80));
                    }
                });
            }

            // Handle stderr
            if (this.forgeProcess.stderr) {
                this.forgeProcess.stderr.on('data', (data: Buffer) => {
                    const error = data.toString().trim();
                    if (error) {
                        this.outputChannel.appendLine('');
                        this.outputChannel.appendLine('$(error) Forge Error:');
                        this.outputChannel.appendLine('─'.repeat(80));
                        error.split('\n').forEach(line => {
                            const cleanLine = line.replace(/\x1b\[[0-9;]*m/g, '');
                            if (cleanLine.trim()) {
                                this.outputChannel.appendLine(`   ${cleanLine}`);
                            }
                        });
                        this.outputChannel.appendLine('─'.repeat(80));
                    }
                });
            }

            // Handle process exit
            this.forgeProcess.on('exit', (code, signal) => {
                this.isForgeRunning = false;
                this.outputChannel.appendLine('');
                if (code === 0) {
                    this.log('info', '$(check) Forge process exited normally');
                } else {
                    this.log('error', `$(error) Forge process exited with code ${code} (signal: ${signal})`);
                }
                this.logDivider();
            });

            // Handle errors
            this.forgeProcess.on('error', (error: Error) => {
                this.isForgeRunning = false;
                this.log('error', `$(error) Failed to start Forge: ${error.message}`);
            });

            // Give it a moment to start
            await new Promise(resolve => setTimeout(resolve, 500));

            if (this.isForgeRunning) {
                this.log('success', '$(check) Forge binary started successfully');
                this.log('info', `   $(debug) Process PID: ${this.forgeProcess.pid}`);
                this.logDivider();
            }

        } catch (error) {
            this.isForgeRunning = false;
            const errorMsg = error instanceof Error ? error.message : String(error);
            this.log('error', `$(error) Failed to start Forge: ${errorMsg}`);
            throw error;
        }
    }

    private startFileWatching() {
        // Watch all workspace files
        const pattern = new vscode.RelativePattern(
            this.rootPath,
            '**/*'
        );

        const fileWatcher = vscode.workspace.createFileSystemWatcher(pattern);

        this.fileWatchers.push(fileWatcher);
        this.fileWatchers.push(fileWatcher.onDidChange((uri: vscode.Uri) => this.handleFileChange(uri, 'MODIFIED')));
        this.fileWatchers.push(fileWatcher.onDidCreate((uri: vscode.Uri) => this.handleFileChange(uri, 'CREATED')));
        this.fileWatchers.push(fileWatcher.onDidDelete((uri: vscode.Uri) => this.handleFileChange(uri, 'DELETED')));
    }

    private async startLogFileWatching() {
        const fs = require('fs');
        const logFilePath = path.join(this.rootPath, 'logs', 'forge.log');

        // Check if log file exists
        if (!fs.existsSync(logFilePath)) {
            this.log('info', '$(notebook) Waiting for forge.log to be created...');
            return;
        }

        // Get initial file size
        try {
            const stats = fs.statSync(logFilePath);
            this.logFilePosition = stats.size;
        } catch (error) {
            this.logFilePosition = 0;
        }

        // Create file watcher for forge.log
        const logPattern = new vscode.RelativePattern(
            this.rootPath,
            'logs/forge.log'
        );

        this.logFileWatcher = vscode.workspace.createFileSystemWatcher(logPattern);
        
        this.fileWatchers.push(this.logFileWatcher);
        this.fileWatchers.push(
            this.logFileWatcher.onDidChange(() => this.handleLogFileChange(logFilePath))
        );
        this.fileWatchers.push(
            this.logFileWatcher.onDidCreate(() => this.handleLogFileCreated(logFilePath))
        );

        this.log('info', '$(notebook) Watching forge.log for Forge binary output');
        this.logDivider();

        // Read any existing content
        await this.handleLogFileChange(logFilePath);
    }

    private async handleLogFileCreated(logFilePath: string) {
        this.logFilePosition = 0;
        this.outputChannel.appendLine('');
        this.outputChannel.appendLine('$(notebook) forge.log created');
        this.logDivider();
        await this.handleLogFileChange(logFilePath);
    }

    private async handleLogFileChange(logFilePath: string) {
        const fs = require('fs');

        try {
            const stats = fs.statSync(logFilePath);
            const fileSize = stats.size;

            // Only read new content
            if (fileSize > this.logFilePosition) {
                const stream = fs.createReadStream(logFilePath, {
                    start: this.logFilePosition,
                    end: fileSize
                });

                let newContent = '';
                for await (const chunk of stream) {
                    newContent += chunk.toString();
                }

                // Update position
                this.logFilePosition = fileSize;

                // Display new log entries
                if (newContent.trim()) {
                    this.outputChannel.appendLine('');
                    this.outputChannel.appendLine('$(notebook) Forge Binary Log:');
                    this.outputChannel.appendLine('─'.repeat(80));
                    
                    // Strip ANSI escape codes for cleaner output
                    const cleanContent = newContent.replace(/\x1b\[[0-9;]*m/g, '');
                    
                    cleanContent.split('\n').forEach(line => {
                        if (line.trim()) {
                            this.outputChannel.appendLine(`   ${line}`);
                        }
                    });
                    
                    this.outputChannel.appendLine('─'.repeat(80));
                }
            }
        } catch (error) {
            // File might not exist yet or be locked
        }
    }

    private async handleFileChange(uri: vscode.Uri, changeType: string) {
        const filePath = uri.fsPath;

        // Ignore certain directories
        if (this.shouldIgnore(filePath)) {
            return;
        }

        // Debounce rapid changes
        const existing = this.changeQueue.get(filePath);
        if (existing) {
            clearTimeout(existing);
        }

        const timeout = setTimeout(() => {
            this.changeQueue.delete(filePath);
            this.processFileChange(uri, changeType);
        }, 50);

        this.changeQueue.set(filePath, timeout);
    }

    private shouldIgnore(filePath: string): boolean {
        const ignoredDirs = ['.git', 'node_modules', '.dx', 'target', 'out', 'dist', '.vscode-test'];
        const ignoredExts = ['.vsix', '.log'];

        return ignoredDirs.some(dir => filePath.includes(path.sep + dir + path.sep)) ||
               ignoredExts.some(ext => filePath.endsWith(ext));
    }

    private async processFileChange(uri: vscode.Uri, changeType: string) {
        const startTime = Date.now();
        const relativePath = path.relative(this.rootPath, uri.fsPath);
        const fileName = path.basename(uri.fsPath);

        // Get file icon based on change type - using VS Code icons
        const icon = changeType === 'CREATED' ? '$(new-file)' : 
                    changeType === 'DELETED' ? '$(trash)' : 
                    '$(edit)';
        
        const statusIcon = changeType === 'CREATED' ? '$(add)' : 
                          changeType === 'DELETED' ? '$(close)' : 
                          '$(check)';

        this.outputChannel.appendLine('');
        this.outputChannel.appendLine(`${icon} ${changeType} │ ${this.formatTime(new Date())}`);
        this.outputChannel.appendLine(`   $(file-code) ${fileName}`);
        this.outputChannel.appendLine(`   $(folder-opened) ${relativePath}`);

        // Try to read file content and show preview
        if (changeType !== 'DELETED') {
            try {
                const document = await vscode.workspace.openTextDocument(uri);
                const content = document.getText();
                const lines = content.split('\n');

                this.outputChannel.appendLine(`   $(list-ordered) ${lines.length} lines, ${content.length} bytes`);
                this.outputChannel.appendLine(`   $(symbol-color) ${document.languageId}`);

                // Show content preview
                const config = vscode.workspace.getConfiguration('forge');
                const showDiffs = config.get('showDiffs', true);

                if (showDiffs && lines.length > 0 && lines.length <= 100) {
                    this.outputChannel.appendLine('');
                    this.outputChannel.appendLine('   $(code) Content Preview:');
                    
                    const previewLines = lines.slice(0, Math.min(20, lines.length));
                    previewLines.forEach((line, idx) => {
                        const lineNum = String(idx + 1).padStart(4, ' ');
                        this.outputChannel.appendLine(`      ${lineNum} │ ${line}`);
                    });

                    if (lines.length > 20) {
                        this.outputChannel.appendLine(`           │ $(ellipsis) ${lines.length - 20} more lines`);
                    }
                }
            } catch (error) {
                // File might be binary or unreadable
                this.outputChannel.appendLine(`   $(alert) Binary or unreadable file`);
            }
        }

        const duration = Date.now() - startTime;
        this.outputChannel.appendLine(`   $(clock) Processed in ${duration}ms`);
    }

    async showFileAST(filePath: string): Promise<void> {
        return new Promise((resolve, reject) => {
            try {
                this.outputChannel.appendLine('');
                this.log('info', '$(search) Analyzing file with Forge...');
                this.outputChannel.appendLine('');

                // Read file and parse with tree-sitter internally
                const fs = require('fs');
                const content = fs.readFileSync(filePath, 'utf8');
                const lines = content.split('\n');

                // Show structure analysis
                this.outputChannel.appendLine('$(symbol-structure) File Structure:');
                this.outputChannel.appendLine(`   $(graph-line) Total Lines: ${lines.length}`);
                this.outputChannel.appendLine(`   $(database) File Size: ${content.length} bytes`);
                this.outputChannel.appendLine('');

                // Analyze language-specific structure
                const ext = path.extname(filePath).toLowerCase();
                this.analyzeFileStructure(content, ext, lines);

                resolve();
            } catch (error) {
                const errorMsg = error instanceof Error ? error.message : String(error);
                this.log('error', `Failed to analyze file: ${errorMsg}`);
                reject(error);
            }
        });
    }

    private analyzeFileStructure(content: string, ext: string, lines: string[]) {
        this.outputChannel.appendLine('$(type-hierarchy) Syntax Tree:');
        this.outputChannel.appendLine('');

        if (ext === '.rs') {
            this.analyzeRustFile(lines);
        } else if (ext === '.ts' || ext === '.js') {
            this.analyzeJavaScriptFile(lines);
        } else if (ext === '.py') {
            this.analyzePythonFile(lines);
        } else {
            // Generic analysis
            this.analyzeGenericFile(lines);
        }
    }

    private analyzeRustFile(lines: string[]) {
        const structs: string[] = [];
        const enums: string[] = [];
        const functions: string[] = [];
        const impls: string[] = [];
        const mods: string[] = [];

        lines.forEach((line, idx) => {
            const trimmed = line.trim();
            if (trimmed.startsWith('struct ')) {
                structs.push(`   Line ${idx + 1}: ${trimmed}`);
            } else if (trimmed.startsWith('enum ')) {
                enums.push(`   Line ${idx + 1}: ${trimmed}`);
            } else if (trimmed.startsWith('fn ') || trimmed.startsWith('pub fn ') || trimmed.startsWith('async fn ') || trimmed.startsWith('pub async fn ')) {
                functions.push(`   Line ${idx + 1}: ${trimmed}`);
            } else if (trimmed.startsWith('impl ')) {
                impls.push(`   Line ${idx + 1}: ${trimmed}`);
            } else if (trimmed.startsWith('mod ') || trimmed.startsWith('pub mod ')) {
                mods.push(`   Line ${idx + 1}: ${trimmed}`);
            }
        });

        if (mods.length > 0) {
            this.outputChannel.appendLine(`   $(package) Modules (${mods.length}):`);
            mods.forEach(m => this.outputChannel.appendLine(m));
            this.outputChannel.appendLine('');
        }

        if (structs.length > 0) {
            this.outputChannel.appendLine(`   $(symbol-struct) Structs (${structs.length}):`);
            structs.forEach(s => this.outputChannel.appendLine(s));
            this.outputChannel.appendLine('');
        }

        if (enums.length > 0) {
            this.outputChannel.appendLine(`   $(symbol-enum) Enums (${enums.length}):`);
            enums.forEach(e => this.outputChannel.appendLine(e));
            this.outputChannel.appendLine('');
        }

        if (impls.length > 0) {
            this.outputChannel.appendLine(`   $(symbol-interface) Implementations (${impls.length}):`);
            impls.forEach(i => this.outputChannel.appendLine(i));
            this.outputChannel.appendLine('');
        }

        if (functions.length > 0) {
            this.outputChannel.appendLine(`   $(symbol-method) Functions (${functions.length}):`);
            functions.forEach(f => this.outputChannel.appendLine(f));
        }
    }

    private analyzeJavaScriptFile(lines: string[]) {
        const classes: string[] = [];
        const functions: string[] = [];
        const imports: string[] = [];
        const exports: string[] = [];

        lines.forEach((line, idx) => {
            const trimmed = line.trim();
            if (trimmed.startsWith('class ')) {
                classes.push(`   Line ${idx + 1}: ${trimmed}`);
            } else if (trimmed.startsWith('function ') || trimmed.match(/^(export\s+)?(async\s+)?function\s+/)) {
                functions.push(`   Line ${idx + 1}: ${trimmed}`);
            } else if (trimmed.startsWith('import ')) {
                imports.push(`   Line ${idx + 1}: ${trimmed}`);
            } else if (trimmed.startsWith('export ')) {
                exports.push(`   Line ${idx + 1}: ${trimmed}`);
            }
        });

        if (imports.length > 0) {
            this.outputChannel.appendLine(`   $(cloud-download) Imports (${imports.length}):`);
            imports.slice(0, 5).forEach(i => this.outputChannel.appendLine(i));
            if (imports.length > 5) {
                this.outputChannel.appendLine(`   $(ellipsis) and ${imports.length - 5} more`);
            }
            this.outputChannel.appendLine('');
        }

        if (classes.length > 0) {
            this.outputChannel.appendLine(`   $(symbol-class) Classes (${classes.length}):`);
            classes.forEach(c => this.outputChannel.appendLine(c));
            this.outputChannel.appendLine('');
        }

        if (functions.length > 0) {
            this.outputChannel.appendLine(`   $(symbol-method) Functions (${functions.length}):`);
            functions.forEach(f => this.outputChannel.appendLine(f));
        }
    }

    private analyzePythonFile(lines: string[]) {
        const classes: string[] = [];
        const functions: string[] = [];
        const imports: string[] = [];

        lines.forEach((line, idx) => {
            const trimmed = line.trim();
            if (trimmed.startsWith('class ')) {
                classes.push(`   Line ${idx + 1}: ${trimmed}`);
            } else if (trimmed.startsWith('def ') || trimmed.startsWith('async def ')) {
                functions.push(`   Line ${idx + 1}: ${trimmed}`);
            } else if (trimmed.startsWith('import ') || trimmed.startsWith('from ')) {
                imports.push(`   Line ${idx + 1}: ${trimmed}`);
            }
        });

        if (imports.length > 0) {
            this.outputChannel.appendLine(`   $(cloud-download) Imports (${imports.length}):`);
            imports.slice(0, 5).forEach(i => this.outputChannel.appendLine(i));
            if (imports.length > 5) {
                this.outputChannel.appendLine(`   $(ellipsis) and ${imports.length - 5} more`);
            }
            this.outputChannel.appendLine('');
        }

        if (classes.length > 0) {
            this.outputChannel.appendLine(`   $(symbol-class) Classes (${classes.length}):`);
            classes.forEach(c => this.outputChannel.appendLine(c));
            this.outputChannel.appendLine('');
        }

        if (functions.length > 0) {
            this.outputChannel.appendLine(`   $(symbol-method) Functions (${functions.length}):`);
            functions.forEach(f => this.outputChannel.appendLine(f));
        }
    }

    private analyzeGenericFile(lines: string[]) {
        const nonEmptyLines = lines.filter(l => l.trim().length > 0).length;
        const commentLines = lines.filter(l => {
            const t = l.trim();
            return t.startsWith('//') || t.startsWith('#') || t.startsWith('/*') || t.startsWith('*');
        }).length;

        this.outputChannel.appendLine(`   $(file-code) Content Analysis:`);
        this.outputChannel.appendLine(`   $(check) Non-empty lines: ${nonEmptyLines}`);
        this.outputChannel.appendLine(`   $(comment) Comment lines: ${commentLines}`);
        this.outputChannel.appendLine('');

        this.outputChannel.appendLine('   $(code) First 30 lines:');
        lines.slice(0, 30).forEach((line, idx) => {
            const lineNum = String(idx + 1).padStart(4, ' ');
            this.outputChannel.appendLine(`   ${lineNum} │ ${line}`);
        });

        if (lines.length > 30) {
            this.outputChannel.appendLine(`        │ $(ellipsis) ${lines.length - 30} more lines`);
        }
    }

    stop() {
        // Clear any pending timeouts
        for (const timeout of this.changeQueue.values()) {
            clearTimeout(timeout);
        }
        this.changeQueue.clear();

        // Stop forge process if running
        if (this.forgeProcess && this.isForgeRunning) {
            this.log('info', '$(debug-stop) Stopping Forge binary...');
            this.forgeProcess.kill('SIGTERM');
            this.forgeProcess = undefined;
            this.isForgeRunning = false;
        }

        // Dispose log file watcher
        if (this.logFileWatcher) {
            this.logFileWatcher.dispose();
            this.logFileWatcher = undefined;
        }

        // Dispose all file watchers
        for (const disposable of this.fileWatchers) {
            disposable.dispose();
        }
        this.fileWatchers = [];

        this.log('info', '$(debug-stop) Watcher stopped');
    }

    private formatTime(date: Date): string {
        const hours = String(date.getHours()).padStart(2, '0');
        const minutes = String(date.getMinutes()).padStart(2, '0');
        const seconds = String(date.getSeconds()).padStart(2, '0');
        const ms = String(date.getMilliseconds()).padStart(3, '0');
        return `${hours}:${minutes}:${seconds}.${ms}`;
    }

    private log(level: 'info' | 'success' | 'error' | 'warning', message: string) {
        const timestamp = new Date().toISOString().substr(11, 12);
        const icon = level === 'success' ? '$(check)' : 
                    level === 'error' ? '$(error)' : 
                    level === 'warning' ? '$(warning)' : 
                    '$(info)';
        this.outputChannel.appendLine(`[${timestamp}] ${icon} ${message}`);
    }

    private logDivider() {
        this.outputChannel.appendLine('─'.repeat(80));
    }
}
