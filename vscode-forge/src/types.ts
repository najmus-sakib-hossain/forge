export type OperationType = 
    | 'insert'
    | 'delete'
    | 'replace'
    | 'created'
    | 'modified'
    | 'deleted'
    | 'renamed'
    | 'Insert'
    | 'Delete'
    | 'Replace'
    | 'FileCreate'
    | 'FileDelete'
    | 'FileRename';

export interface Operation {
    id: string;
    timestamp: string;
    actor_id: string;
    file_path: string;
    op_type: OperationType;
    duration_ms?: number;
    line?: number;
    column?: number;
    length?: number;
    content?: string;
    old_content?: string;
    new_content?: string;
}

export interface ForgeConfig {
    version: string;
    actor_id: string;
    repo_id: string;
    created_at: string;
}
