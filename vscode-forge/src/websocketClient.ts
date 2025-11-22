import * as vscode from 'vscode';
import * as WebSocket from 'ws';

export interface CollaborationEvent {
    type: 'cursor' | 'selection' | 'operation' | 'presence';
    peerId: string;
    data: any;
}

export interface PeerInfo {
    id: string;
    name: string;
    color: string;
    online: boolean;
}

export class WebSocketClient {
    private ws: WebSocket | null = null;
    private serverUrl: string;
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5;
    private reconnectDelay = 1000;
    private eventEmitter = new vscode.EventEmitter<CollaborationEvent>();

    public readonly onEvent = this.eventEmitter.event;

    constructor(serverUrl: string) {
        this.serverUrl = serverUrl;
    }

    connect(): Promise<boolean> {
        return new Promise((resolve, reject) => {
            try {
                this.ws = new WebSocket(this.serverUrl);

                this.ws.on('open', () => {
                    console.log('✅ WebSocket connected to Forge server');
                    this.reconnectAttempts = 0;
                    resolve(true);
                });

                this.ws.on('message', (data: WebSocket.Data) => {
                    try {
                        const event = JSON.parse(data.toString()) as CollaborationEvent;
                        this.eventEmitter.fire(event);
                    } catch (error) {
                        console.error('Failed to parse WebSocket message:', error);
                    }
                });

                this.ws.on('error', (error) => {
                    console.error('WebSocket error:', error);
                    reject(error);
                });

                this.ws.on('close', () => {
                    console.log('WebSocket connection closed');
                    this.attemptReconnect();
                });

            } catch (error) {
                console.error('Failed to create WebSocket:', error);
                reject(error);
            }
        });
    }

    private attemptReconnect(): void {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('Max reconnection attempts reached');
            return;
        }

        this.reconnectAttempts++;
        const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

        console.log(`Attempting to reconnect in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

        setTimeout(() => {
            this.connect().catch(err => {
                console.error('Reconnection failed:', err);
            });
        }, delay);
    }

    sendCursorPosition(line: number, character: number): void {
        this.send({
            type: 'cursor',
            peerId: this.getPeerId(),
            data: { line, character }
        });
    }

    sendSelection(startLine: number, startChar: number, endLine: number, endChar: number): void {
        this.send({
            type: 'selection',
            peerId: this.getPeerId(),
            data: { startLine, startChar, endLine, endChar }
        });
    }

    sendPresence(online: boolean): void {
        this.send({
            type: 'presence',
            peerId: this.getPeerId(),
            data: { online }
        });
    }

    private send(event: CollaborationEvent): void {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(event));
        }
    }

    private getPeerId(): string {
        // In a real implementation, this would be generated/stored persistently
        return 'peer-' + Math.random().toString(36).substr(2, 9);
    }

    disconnect(): void {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }

    isConnected(): boolean {
        return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
    }
}
