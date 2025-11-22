import * as vscode from 'vscode';
import { ForgeDatabase, TrafficBranch } from './database';

export class TrafficBranchPanel {
    public static currentPanel: TrafficBranchPanel | undefined;
    private readonly _panel: vscode.WebviewPanel;
    private readonly _extensionUri: vscode.Uri;
    private _disposables: vscode.Disposable[] = [];
    private _database: ForgeDatabase | null = null;

    private constructor(panel: vscode.WebviewPanel, extensionUri: vscode.Uri) {
        this._panel = panel;
        this._extensionUri = extensionUri;

        // Set up update on visibility change
        this._panel.onDidChangeViewState(
            e => {
                if (this._panel.visible) {
                    this._update();
                }
            },
            null,
            this._disposables
        );

        // Handle disposal
        this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

        // Initial update
        this._update();
    }

    public static createOrShow(extensionUri: vscode.Uri, database: ForgeDatabase | null): void {
        const column = vscode.ViewColumn.Two;

        if (TrafficBranchPanel.currentPanel) {
            TrafficBranchPanel.currentPanel._panel.reveal(column);
            TrafficBranchPanel.currentPanel._database = database;
            TrafficBranchPanel.currentPanel._update();
            return;
        }

        const panel = vscode.window.createWebviewPanel(
            'trafficBranch',
            'Traffic Branch Status',
            column,
            {
                enableScripts: true,
                retainContextWhenHidden: true
            }
        );

        TrafficBranchPanel.currentPanel = new TrafficBranchPanel(panel, extensionUri);
        TrafficBranchPanel.currentPanel._database = database;
    }

    public dispose(): void {
        TrafficBranchPanel.currentPanel = undefined;

        this._panel.dispose();

        while (this._disposables.length) {
            const disposable = this._disposables.pop();
            if (disposable) {
                disposable.dispose();
            }
        }
    }

    private _update(): void {
        const webview = this._panel.webview;
        this._panel.title = 'Traffic Branch Status';
        this._panel.webview.html = this._getHtmlForWebview(webview);
    }

    private _getHtmlForWebview(webview: vscode.Webview): string {
        let branches: TrafficBranch[] = [];

        if (this._database) {
            branches = this._database.getTrafficBranchStatus();
        }

        return `<!DOCTYPE html>
        <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Traffic Branch Status</title>
            <style>
                body {
                    font-family: var(--vscode-font-family);
                    color: var(--vscode-foreground);
                    background-color: var(--vscode-editor-background);
                    padding: 20px;
                }
                .header {
                    font-size: 24px;
                    font-weight: bold;
                    margin-bottom: 20px;
                }
                .branch-item {
                    border: 1px solid var(--vscode-panel-border);
                    border-radius: 4px;
                    padding: 15px;
                    margin-bottom: 10px;
                    background: var(--vscode-editor-inactiveSelectionBackground);
                }
                .status-indicator {
                    display: inline-blockwidth: 12px;
                    height: 12px;
                    border-radius: 50%;
                    margin-right: 8px;
                }
                .green { background-color: #4CAF50; }
                .yellow { background-color: #FFC107; }
                .red { background-color: #F44336; }
                .file-path {
                    font-family: monospace;
                    font-size: 14px;
                    font-weight: bold;
                    margin-bottom: 8px;
                }
                .conflicts {
                    margin-top: 10px;
                    padding-left: 20px;
                }
                .conflict-item {
                    color: var(--vscode-errorForeground);
                    margin: 5px 0;
                }
                .no-data {
                    text-align: center;
                    padding: 40px;
                    color: var(--vscode-descriptionForeground);
                }
            </style>
        </head>
        <body>
            <div class="header">🚦 Traffic Branch Status</div>
            ${branches.length > 0 ? branches.map(branch => `
                <div class="branch-item">
                    <div class="file-path">
                        <span class="status-indicator ${branch.status}"></span>
                        ${branch.file_path}
                    </div>
                    <div>Status: <strong>${branch.status.toUpperCase()}</strong></div>
                    ${branch.conflicts.length > 0 ? `
                        <div class="conflicts">
                            <div>⚠️ Conflicts:</div>
                            ${branch.conflicts.map(c => `
                                <div class="conflict-item">Line ${c.line}: ${c.reason}</div>
                            `).join('')}
                        </div>
                    ` : ''}
                </div>
            `).join('') : '<div class="no-data">No traffic branch data available</div>'}
        </body>
        </html>`;
    }
}
