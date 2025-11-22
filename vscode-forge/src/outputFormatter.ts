import * as vscode from 'vscode';
import { Operation } from './types';

export class OutputFormatter {
    private static getConfig() {
        return vscode.workspace.getConfiguration('forge');
    }

    static logHeader(channel: vscode.OutputChannel, text: string) {
        const timestamp = this.formatTimestamp(new Date());
        channel.appendLine('');
        channel.appendLine('═'.repeat(80));
        channel.appendLine(`  ${text}`);
        channel.appendLine(`  ${timestamp}`);
        channel.appendLine('═'.repeat(80));
        channel.appendLine('');
    }

    static logDivider(channel: vscode.OutputChannel) {
        channel.appendLine('─'.repeat(80));
    }

    static logInfo(channel: vscode.OutputChannel, message: string) {
        const timestamp = this.getTimestampPrefix();
        channel.appendLine(`${timestamp}$(info) ${message}`);
    }

    static logSuccess(channel: vscode.OutputChannel, message: string) {
        const timestamp = this.getTimestampPrefix();
        channel.appendLine(`${timestamp}$(check) ${message}`);
    }

    static logError(channel: vscode.OutputChannel, message: string) {
        const timestamp = this.getTimestampPrefix();
        channel.appendLine(`${timestamp}$(error) ${message}`);
    }

    static logWarning(channel: vscode.OutputChannel, message: string) {
        const timestamp = this.getTimestampPrefix();
        channel.appendLine(`${timestamp}$(warning) ${message}`);
    }

    static logOperation(channel: vscode.OutputChannel, operation: Operation) {
        const config = this.getConfig();
        const showTimestamps = config.get('showTimestamps', true);
        const showDuration = config.get('showDuration', true);
        const showDiffs = config.get('showDiffs', true);

        channel.appendLine('');

        // Operation header with timestamp
        const timestamp = new Date(operation.timestamp);
        const timeStr = this.formatTimestamp(timestamp);
        const relativeTime = this.formatRelativeTime(timestamp);

        // Icon based on operation type
        const icon = this.getOperationIcon(operation.op_type);
        const actionText = this.getActionText(operation.op_type);
        
        // Main operation line
        let mainLine = `${icon} ${actionText.toUpperCase()}`;
        
        if (showTimestamps) {
            mainLine += ` │ ${timeStr} (${relativeTime})`;
        }

        channel.appendLine(mainLine);

        // File path
        const fileName = this.getFileName(operation.file_path);
        const filePath = operation.file_path;
        channel.appendLine(`   $(file) ${fileName}`);
        channel.appendLine(`   $(folder) ${filePath}`);

        // Duration (if available)
        if (showDuration && operation.duration_ms !== undefined && operation.duration_ms > 0) {
            const durationStr = this.formatDuration(operation.duration_ms);
            channel.appendLine(`   $(watch) ${durationStr}`);
        }

        // Actor info
        if (operation.actor_id) {
            channel.appendLine(`   $(account) ${operation.actor_id}`);
        }

        // Content preview (if available)
        if (showDiffs && operation.content) {
            channel.appendLine('');
            this.logContentPreview(channel, operation.content, operation.op_type);
        }

        // Additional metadata
        if (operation.line !== undefined && operation.column !== undefined) {
            channel.appendLine(`   $(location) Line ${operation.line}, Column ${operation.column}`);
        }

        if (operation.length !== undefined) {
            channel.appendLine(`   $(ruler) Length: ${operation.length} characters`);
        }

        channel.appendLine('');
    }

    private static logContentPreview(
        channel: vscode.OutputChannel,
        content: string,
        opType: string
    ) {
        const maxLines = 10;
        const lines = content.split('\n').slice(0, maxLines);

        const prefix = opType === 'insert' ? '   + ' : 
                      opType === 'delete' ? '   - ' : 
                      '   ~ ';

        for (const line of lines) {
            if (line.trim()) {
                channel.appendLine(`${prefix}${line}`);
            }
        }

        if (content.split('\n').length > maxLines) {
            channel.appendLine(`   ... (${content.split('\n').length - maxLines} more lines)`);
        }
    }

    private static getOperationIcon(opType: string): string {
        const icons: Record<string, string> = {
            'insert': '$(add)',
            'delete': '$(trash)',
            'replace': '$(replace)',
            'created': '$(new-file)',
            'modified': '$(edit)',
            'deleted': '$(trash)',
            'renamed': '$(file-symlink-file)',
            'FileCreate': '$(new-file)',
            'FileDelete': '$(trash)',
            'FileRename': '$(file-symlink-file)',
            'Insert': '$(add)',
            'Delete': '$(trash)',
            'Replace': '$(replace)'
        };

        return icons[opType] || '$(symbol-misc)';
    }

    private static getActionText(opType: string): string {
        const actions: Record<string, string> = {
            'insert': 'Insert',
            'delete': 'Delete',
            'replace': 'Replace',
            'created': 'Created',
            'modified': 'Modified',
            'deleted': 'Deleted',
            'renamed': 'Renamed',
            'FileCreate': 'File Created',
            'FileDelete': 'File Deleted',
            'FileRename': 'File Renamed',
            'Insert': 'Insert',
            'Delete': 'Delete',
            'Replace': 'Replace'
        };

        return actions[opType] || opType;
    }

    private static getFileName(filePath: string): string {
        const parts = filePath.split(/[/\\]/);
        return parts[parts.length - 1] || filePath;
    }

    private static formatTimestamp(date: Date): string {
        const hours = String(date.getHours()).padStart(2, '0');
        const minutes = String(date.getMinutes()).padStart(2, '0');
        const seconds = String(date.getSeconds()).padStart(2, '0');
        const ms = String(date.getMilliseconds()).padStart(3, '0');
        
        return `${hours}:${minutes}:${seconds}.${ms}`;
    }

    private static formatRelativeTime(date: Date): string {
        const now = new Date();
        const diffMs = now.getTime() - date.getTime();
        
        if (diffMs < 1000) {
            return 'just now';
        } else if (diffMs < 60000) {
            const seconds = Math.floor(diffMs / 1000);
            return `${seconds}s ago`;
        } else if (diffMs < 3600000) {
            const minutes = Math.floor(diffMs / 60000);
            return `${minutes}m ago`;
        } else {
            const hours = Math.floor(diffMs / 3600000);
            return `${hours}h ago`;
        }
    }

    private static formatDuration(ms: number): string {
        if (ms < 1) {
            return `${(ms * 1000).toFixed(0)}µs`;
        } else if (ms < 1000) {
            return `${ms.toFixed(2)}ms`;
        } else {
            return `${(ms / 1000).toFixed(2)}s`;
        }
    }

    private static getTimestampPrefix(): string {
        const config = this.getConfig();
        const showTimestamps = config.get('showTimestamps', true);
        
        if (!showTimestamps) {
            return '';
        }

        const timestamp = this.formatTimestamp(new Date());
        return `[${timestamp}] `;
    }
}
