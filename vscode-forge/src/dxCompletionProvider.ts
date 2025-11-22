import * as vscode from 'vscode';

/**
 * DX IntelliSense Provider
 * Provides autocomplete suggestions for DX (Developer Experience) metrics
 */
export class DxCompletionProvider implements vscode.CompletionItemProvider {
    private dxMetrics: Map<string, DxMetric> = new Map();
    private metricCounter = 1;

    constructor() {
        // Initialize with some dummy metrics
        this.initializeDummyMetrics();
    }

    private initializeDummyMetrics() {
        // Pre-populate with incremental dummy DX metrics
        for (let i = 1; i <= 10; i++) {
            const metricId = `dx-${i}`;
            this.dxMetrics.set(metricId, {
                id: metricId,
                name: `DX Metric ${i}`,
                category: this.getCategoryForIndex(i),
                score: Math.floor(Math.random() * 40) + 60, // 60-100 range
                description: `Developer Experience metric ${i}: ${this.getDescriptionForIndex(i)}`,
                timestamp: new Date(),
                details: {
                    buildTime: Math.floor(Math.random() * 5000) + 1000,
                    codeComplexity: Math.floor(Math.random() * 100),
                    testCoverage: Math.floor(Math.random() * 30) + 70,
                    documentationQuality: Math.floor(Math.random() * 30) + 70
                }
            });
        }
    }

    private getCategoryForIndex(index: number): string {
        const categories = ['Performance', 'Code Quality', 'Testing', 'Documentation', 'Architecture'];
        return categories[index % categories.length];
    }

    private getDescriptionForIndex(index: number): string {
        const descriptions = [
            'Build performance optimization',
            'Code maintainability score',
            'Test coverage analysis',
            'API documentation quality',
            'Architecture complexity',
            'Developer onboarding speed',
            'Code review efficiency',
            'Deployment reliability',
            'Error handling patterns',
            'Security compliance'
        ];
        return descriptions[(index - 1) % descriptions.length];
    }

    /**
     * Register a new DX metric (for future real data integration)
     */
    public registerMetric(metric: DxMetric): void {
        this.dxMetrics.set(metric.id, metric);
    }

    /**
     * Provide completion items for DX metrics
     */
    provideCompletionItems(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken,
        context: vscode.CompletionContext
    ): vscode.ProviderResult<vscode.CompletionItem[] | vscode.CompletionList> {
        const lineText = document.lineAt(position).text;
        const beforeCursor = lineText.substring(0, position.character);

        // Trigger on patterns like: dx, DX, dx-, dx-1, dx_foo, or "dx <partial>"
        const triggerMatch = beforeCursor.match(/(?:\b|^)(dx|DX)(?:[-_\s]?[\w-]*)?$/i);
        if (!triggerMatch) {
            return undefined;
        }

        const completionItems: vscode.CompletionItem[] = [];

        // Add all DX metrics as completion items
        this.dxMetrics.forEach((metric, id) => {
            const item = new vscode.CompletionItem(id, vscode.CompletionItemKind.Value);
            item.detail = `$(graph) ${metric.name} - Score: ${metric.score}/100`;
            item.documentation = new vscode.MarkdownString();
            
            item.documentation.appendMarkdown(`### $(dashboard) ${metric.name}\n\n`);
            item.documentation.appendMarkdown(`**Category:** \`${metric.category}\`\n\n`);
            item.documentation.appendMarkdown(`**Score:** \`${metric.score}/100\` ${this.getScoreEmoji(metric.score)}\n\n`);
            item.documentation.appendMarkdown(`**Description:** ${metric.description}\n\n`);
            
            item.documentation.appendMarkdown(`---\n\n`);
            item.documentation.appendMarkdown(`#### $(beaker) Detailed Metrics\n\n`);
            item.documentation.appendMarkdown(`| Metric | Value |\n`);
            item.documentation.appendMarkdown(`|--------|-------|\n`);
            item.documentation.appendMarkdown(`| $(watch) Build Time | ${metric.details.buildTime}ms |\n`);
            item.documentation.appendMarkdown(`| $(circuit-board) Code Complexity | ${metric.details.codeComplexity} |\n`);
            item.documentation.appendMarkdown(`| $(pass) Test Coverage | ${metric.details.testCoverage}% |\n`);
            item.documentation.appendMarkdown(`| $(book) Documentation | ${metric.details.documentationQuality}% |\n`);
            
            item.documentation.appendMarkdown(`\n---\n`);
            item.documentation.appendMarkdown(`*Last updated: ${metric.timestamp.toLocaleString()}*\n`);
            item.documentation.appendMarkdown(`\n$(info) *This is dummy data. Real metrics will be provided later.*\n`);

            item.insertText = id;
            item.sortText = id;
            // Allow filtering to match even when user typed partial tokens after "dx"
            item.filterText = `${id}`;

            // Add command to show more details on selection
            item.command = {
                command: 'forge.showDxMetricDetail',
                title: 'Show DX Metric Detail',
                arguments: [metric]
            };

            completionItems.push(item);
        });

        // Add a general "dx" completion that shows all metrics
        const summaryItem = new vscode.CompletionItem('dx-summary', vscode.CompletionItemKind.Snippet);
        summaryItem.detail = '$(graph-line) View all DX metrics';
        summaryItem.documentation = new vscode.MarkdownString();
        summaryItem.documentation.appendMarkdown(`### $(dashboard) Developer Experience Summary\n\n`);
        summaryItem.documentation.appendMarkdown(`Available DX metrics: **${this.dxMetrics.size}**\n\n`);
        summaryItem.documentation.appendMarkdown(`Use autocomplete to explore individual metrics.\n`);
        summaryItem.insertText = 'dx-summary';
        summaryItem.sortText = '0'; // Sort first
        
        completionItems.unshift(summaryItem);

        return completionItems;
    }

    /**
     * Provide hover information for DX metric references
     */
    provideHover(
        document: vscode.TextDocument,
        position: vscode.Position,
        token: vscode.CancellationToken
    ): vscode.ProviderResult<vscode.Hover> {
        const range = document.getWordRangeAtPosition(position, /dx-\d+/i);
        if (!range) {
            return undefined;
        }

        const word = document.getText(range);
        const metric = this.dxMetrics.get(word);

        if (!metric) {
            return undefined;
        }

        const markdown = new vscode.MarkdownString();
        markdown.appendMarkdown(`### $(graph) ${metric.name}\n\n`);
        markdown.appendMarkdown(`**Category:** \`${metric.category}\`\n\n`);
        markdown.appendMarkdown(`**Score:** ${this.getScoreBar(metric.score)} **${metric.score}/100**\n\n`);
        markdown.appendMarkdown(`${metric.description}\n\n`);
        markdown.appendMarkdown(`---\n\n`);
        markdown.appendMarkdown(`| Metric | Value |\n`);
        markdown.appendMarkdown(`|--------|-------|\n`);
        markdown.appendMarkdown(`| Build Time | ${metric.details.buildTime}ms |\n`);
        markdown.appendMarkdown(`| Complexity | ${metric.details.codeComplexity} |\n`);
        markdown.appendMarkdown(`| Test Coverage | ${metric.details.testCoverage}% |\n`);
        markdown.appendMarkdown(`| Documentation | ${metric.details.documentationQuality}% |\n`);
        markdown.appendMarkdown(`\n*Last updated: ${metric.timestamp.toLocaleString()}*\n`);

        return new vscode.Hover(markdown, range);
    }

    private getScoreEmoji(score: number): string {
        if (score >= 90) return '$(check-all)';
        if (score >= 75) return '$(check)';
        if (score >= 60) return '$(info)';
        return '$(warning)';
    }

    private getScoreBar(score: number): string {
        const filled = Math.floor(score / 10);
        const empty = 10 - filled;
        return '▓'.repeat(filled) + '░'.repeat(empty);
    }

    /**
     * Get a specific metric by ID
     */
    public getMetric(id: string): DxMetric | undefined {
        return this.dxMetrics.get(id);
    }

    /**
     * Get all metrics
     */
    public getAllMetrics(): DxMetric[] {
        return Array.from(this.dxMetrics.values());
    }

    /**
     * Add a new incremental dummy metric (for testing)
     */
    public addDummyMetric(): DxMetric {
        const metricId = `dx-${this.metricCounter++}`;
        const metric: DxMetric = {
            id: metricId,
            name: `DX Metric ${this.metricCounter - 1}`,
            category: this.getCategoryForIndex(this.metricCounter - 1),
            score: Math.floor(Math.random() * 40) + 60,
            description: `Developer Experience metric ${this.metricCounter - 1}`,
            timestamp: new Date(),
            details: {
                buildTime: Math.floor(Math.random() * 5000) + 1000,
                codeComplexity: Math.floor(Math.random() * 100),
                testCoverage: Math.floor(Math.random() * 30) + 70,
                documentationQuality: Math.floor(Math.random() * 30) + 70
            }
        };
        
        this.dxMetrics.set(metricId, metric);
        return metric;
    }
}

/**
 * DX Metric interface
 */
export interface DxMetric {
    id: string;
    name: string;
    category: string;
    score: number;
    description: string;
    timestamp: Date;
    details: {
        buildTime: number;
        codeComplexity: number;
        testCoverage: number;
        documentationQuality: number;
    };
}
