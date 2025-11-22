import * as vscode from 'vscode';
import * as path from 'path';
import * as child_process from 'child_process';
import { ForgeWatcher } from './forgeWatcher';
import { DxCompletionProvider, DxMetric } from './dxCompletionProvider';
import { ForgeDatabase } from './database';
import { WebSocketClient } from './websocketClient';
import { TrafficBranchPanel } from './trafficBranchPanel';

let forgeWatcher: ForgeWatcher | undefined;
let outputChannel: vscode.OutputChannel;
let statusBarItem: vscode.StatusBarItem;
let dxProvider: DxCompletionProvider;
let forgeDatabase: ForgeDatabase | null = null;
let webSocketClient: WebSocketClient | null = null;

export function activate(context: vscode.ExtensionContext) {
    console.log('$(rocket) DX Forge LSP Extension activated!');

    // Create output channel
    outputChannel = vscode.window.createOutputChannel('Forge LSP');
    context.subscriptions.push(outputChannel);

    // Create status bar item
    statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
    statusBarItem.text = '$(eye) Forge LSP';
    statusBarItem.tooltip = 'Forge LSP: Inactive';
    statusBarItem.command = 'forge.start';
    context.subscriptions.push(statusBarItem);
    statusBarItem.show();

    // Initialize DX IntelliSense Provider
    dxProvider = new DxCompletionProvider();

    // Register DX completion provider for ALL file types (file, untitled, etc.)
    // This works on any file: .md, .js, .ts, .py, .txt, untitled files, etc.
    const allDocumentSelector = [
        { scheme: 'file' },      // Regular files
        { scheme: 'untitled' }   // Unsaved/new files
    ];

    const dxCompletionDisposable = vscode.languages.registerCompletionItemProvider(
        allDocumentSelector,
        dxProvider,
        'd', 'x', '-' // Trigger on typing 'd', 'x', or '-'
    );
    context.subscriptions.push(dxCompletionDisposable);

    // Register DX hover provider for all file types
    const dxHoverDisposable = vscode.languages.registerHoverProvider(
        allDocumentSelector,
        dxProvider
    );
    context.subscriptions.push(dxHoverDisposable);

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('forge.start', () => startWatching())
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('forge.stop', () => stopWatching())
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('forge.clearOutput', () => {
            outputChannel.clear();
            logInfo('$(sparkle) Output cleared');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('forge.showAST', () => showCurrentFileAST())
    );

    // Command to show DX metric details
    context.subscriptions.push(
        vscode.commands.registerCommand('forge.showDxMetricDetail', (metric: DxMetric) => {
            showDxMetricDetail(metric);
        })
    );

    // Command to show all DX metrics
    context.subscriptions.push(
        vscode.commands.registerCommand('forge.showAllDxMetrics', () => {
            showAllDxMetrics();
        })
    );

    // Command to show Traffic Branch Status
    context.subscriptions.push(
        vscode.commands.registerCommand('forge.showTraffic', () => {
            TrafficBranchPanel.createOrShow(context.extensionUri, forgeDatabase);
        })
    );

    // Auto-start if configured
    const config = vscode.workspace.getConfiguration('forge');
    if (config.get('autoStart', true)) {
        startWatching();
    }
}

async function startWatching() {
    if (forgeWatcher) {
        vscode.window.showInformationMessage('Forge LSP is already running');
        return;
    }

    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders || workspaceFolders.length === 0) {
        vscode.window.showErrorMessage('No workspace folder open');
        return;
    }

    const rootPath = workspaceFolders[0].uri.fsPath;

    outputChannel.clear();
    logHeader('$(rocket) DX FORGE LSP');
    logInfo(`$(folder-active) Monitoring: ${rootPath}`);
    logDivider();

    // Check if forge binary exists
    const forgeBinary = await findForgeBinary(rootPath);
    if (!forgeBinary) {
        vscode.window.showErrorMessage('Forge binary not found. Please build the forge project first.');
        logError('$(error) Forge binary not found');
        logInfo('$(lightbulb) Run: cargo build --release');
        return;
    }

    logSuccess(`$(check) Found Forge binary: ${forgeBinary}`);
    logDivider();

    forgeWatcher = new ForgeWatcher(rootPath, forgeBinary, outputChannel);
    await forgeWatcher.start();

    // Initialize Database
    try {
        forgeDatabase = new ForgeDatabase(rootPath);
        if (forgeDatabase.open()) {
            logSuccess('$(database) Forge Database connected');
        } else {
            logError('$(error) Failed to open Forge Database');
        }
    } catch (error) {
        logError(`$(error) Database error: ${error}`);
    }

    // Initialize WebSocket
    try {
        // Default port 3456, in production this should be configurable
        webSocketClient = new WebSocketClient('ws://localhost:3456');
        webSocketClient.connect().then(() => {
            logSuccess('$(plug) WebSocket connected');
        }).catch(err => {
            logError(`$(alert) WebSocket connection failed: ${err}`);
        });

        // Listen for events
        if (webSocketClient) {
            webSocketClient.onEvent(event => {
                if (event.type === 'presence') {
                    // Update status bar or UI with active users
                    logInfo(`$(person) Peer ${event.peerId} is ${event.data.online ? 'online' : 'offline'}`);
                }
            });
        }
    } catch (error) {
        logError(`$(error) WebSocket error: ${error}`);
    }

    // Update status bar
    statusBarItem.text = '$(eye) Forge LSP: Active';
    statusBarItem.tooltip = 'Forge LSP: Monitoring changes';
    statusBarItem.command = 'forge.stop';
    statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.prominentBackground');

    vscode.window.showInformationMessage('$(rocket) Forge LSP started');
}

function stopWatching() {
    if (!forgeWatcher) {
        vscode.window.showInformationMessage('Forge LSP is not running');
        return;
    }

    forgeWatcher.stop();
    forgeWatcher = undefined;

    if (forgeDatabase) {
        forgeDatabase.close();
        forgeDatabase = null;
        logInfo('$(database) Database closed');
    }

    if (webSocketClient) {
        webSocketClient.disconnect();
        webSocketClient = null;
        logInfo('$(plug) WebSocket disconnected');
    }

    logDivider();
    logInfo('$(debug-stop) Forge LSP stopped');

    // Update status bar
    statusBarItem.text = '$(eye) Forge LSP';
    statusBarItem.tooltip = 'Forge LSP: Inactive';
    statusBarItem.command = 'forge.start';
    statusBarItem.backgroundColor = undefined;

    vscode.window.showInformationMessage('$(debug-stop) Forge LSP stopped');
}

async function findForgeBinary(rootPath: string): Promise<string | null> {
    // Check common locations
    const possiblePaths = [
        path.join(rootPath, 'target', 'release', 'forge-cli.exe'),
        path.join(rootPath, 'target', 'release', 'forge-cli'),
        path.join(rootPath, 'target', 'debug', 'forge-cli.exe'),
        path.join(rootPath, 'target', 'debug', 'forge-cli'),
        'forge-cli.exe',
        'forge-cli'
    ];

    const fs = require('fs');
    for (const binPath of possiblePaths) {
        const fullPath = path.isAbsolute(binPath) ? binPath : path.join(rootPath, binPath);
        if (fs.existsSync(fullPath)) {
            return fullPath;
        }
    }

    return null;
}

async function showCurrentFileAST() {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
        vscode.window.showWarningMessage('No active file');
        return;
    }

    if (!forgeWatcher) {
        vscode.window.showWarningMessage('Forge LSP is not running. Start it first.');
        return;
    }

    const document = editor.document;
    const filePath = document.uri.fsPath;

    outputChannel.show();
    logDivider();
    logHeader('$(graph) FILE AST');
    logInfo(`$(file) File: ${path.basename(filePath)}`);
    logInfo(`$(folder) Path: ${filePath}`);
    logInfo(`$(graph-line) Lines: ${document.lineCount}`);
    logInfo(`$(symbol-keyword) Language: ${document.languageId}`);
    logDivider();

    // Use forgeWatcher to get AST
    await forgeWatcher.showFileAST(filePath);

    logDivider();
}

// DX Metrics Display Functions
function showDxMetricDetail(metric: DxMetric) {
    const panel = vscode.window.createWebviewPanel(
        'dxMetricDetail',
        `$(graph) ${metric.name}`,
        vscode.ViewColumn.Beside,
        { enableScripts: true }
    );

    panel.webview.html = getDxMetricHtml(metric);
}

function showAllDxMetrics() {
    const metrics = dxProvider.getAllMetrics();

    outputChannel.show();
    outputChannel.appendLine('');
    logHeader('$(dashboard) ALL DX METRICS');
    outputChannel.appendLine('');

    const avgScore = metrics.reduce((sum, m) => sum + m.score, 0) / metrics.length;
    outputChannel.appendLine(`$(graph-line) Total Metrics: ${metrics.length}`);
    outputChannel.appendLine(`$(pulse) Average Score: ${avgScore.toFixed(1)}/100`);
    outputChannel.appendLine('');
    logDivider();

    metrics.forEach(metric => {
        outputChannel.appendLine('');
        outputChannel.appendLine(`$(symbol-value) ${metric.id} - ${metric.name}`);
        outputChannel.appendLine(`   $(tag) Category: ${metric.category}`);
        outputChannel.appendLine(`   $(graph) Score: ${metric.score}/100`);
        outputChannel.appendLine(`   $(info) ${metric.description}`);
        outputChannel.appendLine(`   $(watch) Build Time: ${metric.details.buildTime}ms`);
        outputChannel.appendLine(`   $(circuit-board) Complexity: ${metric.details.codeComplexity}`);
        outputChannel.appendLine(`   $(pass) Test Coverage: ${metric.details.testCoverage}%`);
        outputChannel.appendLine(`   $(book) Documentation: ${metric.details.documentationQuality}%`);
    });

    outputChannel.appendLine('');
    logDivider();
    outputChannel.appendLine('');
    outputChannel.appendLine('$(info) Note: These are dummy metrics. Real data will be provided later.');
}

function getDxMetricHtml(metric: DxMetric): string {
    const scoreColor = metric.score >= 90 ? '#4CAF50' :
        metric.score >= 75 ? '#8BC34A' :
            metric.score >= 60 ? '#FFC107' : '#FF5722';

    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>${metric.name}</title>
    <style>
        body {
            font-family: var(--vscode-font-family);
            padding: 20px;
            color: var(--vscode-foreground);
            background-color: var(--vscode-editor-background);
        }
        .header {
            border-bottom: 2px solid ${scoreColor};
            padding-bottom: 15px;
            margin-bottom: 20px;
        }
        .metric-id {
            font-size: 14px;
            color: var(--vscode-descriptionForeground);
            font-family: monospace;
        }
        .metric-name {
            font-size: 28px;
            font-weight: bold;
            margin: 10px 0;
        }
        .category {
            display: inline-block;
            padding: 4px 12px;
            background-color: var(--vscode-badge-background);
            color: var(--vscode-badge-foreground);
            border-radius: 12px;
            font-size: 12px;
        }
        .score-section {
            margin: 30px 0;
            text-align: center;
        }
        .score-circle {
            width: 150px;
            height: 150px;
            border-radius: 50%;
            background: conic-gradient(${scoreColor} ${metric.score * 3.6}deg, var(--vscode-input-background) 0deg);
            display: flex;
            align-items: center;
            justify-content: center;
            margin: 0 auto;
            position: relative;
        }
        .score-inner {
            width: 120px;
            height: 120px;
            border-radius: 50%;
            background-color: var(--vscode-editor-background);
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
        }
        .score-value {
            font-size: 36px;
            font-weight: bold;
            color: ${scoreColor};
        }
        .score-label {
            font-size: 12px;
            color: var(--vscode-descriptionForeground);
        }
        .description {
            margin: 20px 0;
            padding: 15px;
            background-color: var(--vscode-textBlockQuote-background);
            border-left: 4px solid ${scoreColor};
            font-size: 14px;
            line-height: 1.6;
        }
        .metrics-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin: 20px 0;
        }
        .metric-card {
            padding: 15px;
            background-color: var(--vscode-input-background);
            border-radius: 8px;
            border: 1px solid var(--vscode-input-border);
        }
        .metric-card-title {
            font-size: 12px;
            color: var(--vscode-descriptionForeground);
            margin-bottom: 8px;
        }
        .metric-card-value {
            font-size: 24px;
            font-weight: bold;
            color: var(--vscode-foreground);
        }
        .metric-card-unit {
            font-size: 14px;
            color: var(--vscode-descriptionForeground);
        }
        .timestamp {
            margin-top: 30px;
            padding-top: 15px;
            border-top: 1px solid var(--vscode-panel-border);
            font-size: 12px;
            color: var(--vscode-descriptionForeground);
        }
        .notice {
            margin-top: 20px;
            padding: 12px;
            background-color: var(--vscode-inputValidation-infoBackground);
            border: 1px solid var(--vscode-inputValidation-infoBorder);
            border-radius: 4px;
            font-size: 13px;
        }
    </style>
</head>
<body>
    <div class="header">
        <div class="metric-id">${metric.id}</div>
        <div class="metric-name">${metric.name}</div>
        <span class="category">${metric.category}</span>
    </div>

    <div class="score-section">
        <div class="score-circle">
            <div class="score-inner">
                <div class="score-value">${metric.score}</div>
                <div class="score-label">/ 100</div>
            </div>
        </div>
    </div>

    <div class="description">
        ${metric.description}
    </div>

    <h3>Detailed Metrics</h3>
    <div class="metrics-grid">
        <div class="metric-card">
            <div class="metric-card-title">⏱️ Build Time</div>
            <div class="metric-card-value">${metric.details.buildTime}<span class="metric-card-unit">ms</span></div>
        </div>
        <div class="metric-card">
            <div class="metric-card-title">🔧 Code Complexity</div>
            <div class="metric-card-value">${metric.details.codeComplexity}</div>
        </div>
        <div class="metric-card">
            <div class="metric-card-title">✅ Test Coverage</div>
            <div class="metric-card-value">${metric.details.testCoverage}<span class="metric-card-unit">%</span></div>
        </div>
        <div class="metric-card">
            <div class="metric-card-title">📚 Documentation</div>
            <div class="metric-card-value">${metric.details.documentationQuality}<span class="metric-card-unit">%</span></div>
        </div>
    </div>

    <div class="timestamp">
        Last updated: ${metric.timestamp.toLocaleString()}
    </div>

    <div class="notice">
        ℹ️ <strong>Note:</strong> This is dummy data for demonstration purposes. Real metrics will be provided later.
    </div>
</body>
</html>`;
}

// Logging helpers
function logHeader(text: string) {
    outputChannel.appendLine('');
    outputChannel.appendLine('═'.repeat(80));
    outputChannel.appendLine(`  ${text}`);
    outputChannel.appendLine(`  $(clock) ${new Date().toLocaleTimeString()}`);
    outputChannel.appendLine('═'.repeat(80));
    outputChannel.appendLine('');
}

function logDivider() {
    outputChannel.appendLine('─'.repeat(80));
}

function logInfo(message: string) {
    const timestamp = new Date().toISOString().substr(11, 12);
    outputChannel.appendLine(`[${timestamp}] $(info) ${message}`);
}

function logSuccess(message: string) {
    const timestamp = new Date().toISOString().substr(11, 12);
    outputChannel.appendLine(`[${timestamp}] $(check) ${message}`);
}

function logError(message: string) {
    const timestamp = new Date().toISOString().substr(11, 12);
    outputChannel.appendLine(`[${timestamp}] $(error) ${message}`);
}

function logWarning(message: string) {
    const timestamp = new Date().toISOString().substr(11, 12);
    outputChannel.appendLine(`[${timestamp}] $(warning) ${message}`);
}

export function deactivate() {
    if (forgeWatcher) {
        forgeWatcher.stop();
    }
}
