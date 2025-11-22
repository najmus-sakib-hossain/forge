import * as path from 'path';
import Database from 'better-sqlite3';

export interface CrdtOperation {
    id: number;
    operation_type: string;
    file_path: string;
    position: number;
    content: string;
    timestamp: number;
    lamport_clock: number;
    peer_id: string;
}

export interface TrafficBranch {
    file_path: string;
    status: 'green' | 'yellow' | 'red';
    conflicts: Conflict[];
}

export interface Conflict {
    line: number;
    reason: string;
}

export class ForgeDatabase {
    private db: Database.Database | null = null;

    constructor(private forgePath: string) { }

    open(): boolean {
        try {
            const dbPath = path.join(this.forgePath, 'forge.db');
            this.db = new Database(dbPath, { readonly: true });
            return true;
        } catch (error) {
            console.error('Failed to open forge database:', error);
            return false;
        }
    }

    close(): void {
        if (this.db) {
            this.db.close();
            this.db = null;
        }
    }

    getCrdtOperations(limit: number = 100): CrdtOperation[] {
        if (!this.db) {
            return [];
        }

        try {
            const stmt = this.db.prepare(`
                SELECT id, operation_type, file_path, position, content, 
                       timestamp, lamport_clock, peer_id
                FROM crdt_operations
                ORDER BY timestamp DESC
                LIMIT ?
            `);

            return stmt.all(limit) as CrdtOperation[];
        } catch (error) {
            console.error('Error querying CRDT operations:', error);
            return [];
        }
    }

    getFileHistory(filePath: string, limit: number = 50): CrdtOperation[] {
        if (!this.db) {
            return [];
        }

        try {
            const stmt = this.db.prepare(`
                SELECT id, operation_type, file_path, position, content,
                       timestamp, lamport_clock, peer_id
                FROM crdt_operations
                WHERE file_path = ?
                ORDER BY timestamp DESC
                LIMIT ?
            `);

            return stmt.all(filePath, limit) as CrdtOperation[];
        } catch (error) {
            console.error('Error querying file history:', error);
            return [];
        }
    }

    getTrafficBranchStatus(): TrafficBranch[] {
        if (!this.db) {
            return [];
        }

        try {
            const stmt = this.db.prepare(`
                SELECT file_path, status, conflicts
                FROM traffic_branches
                ORDER BY file_path
            `);

            const rows = stmt.all() as any[];
            return rows.map(row => ({
                file_path: row.file_path,
                status: row.status as 'green' | 'yellow' | 'red',
                conflicts: row.conflicts ? JSON.parse(row.conflicts) : []
            }));
        } catch (error) {
            console.error('Error querying traffic branches:', error);
            return [];
        }
    }

    getLamportTimestamp(): number {
        if (!this.db) {
            return 0;
        }

        try {
            const stmt = this.db.prepare(`
                SELECT MAX(lamport_clock) as max_lamport
                FROM crdt_operations
            `);

            const result = stmt.get() as any;
            return result?.max_lamport || 0;
        } catch (error) {
            console.error('Error getting Lamport timestamp:', error);
            return 0;
        }
    }
}
