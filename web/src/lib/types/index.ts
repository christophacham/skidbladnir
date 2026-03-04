export type TaskStatus = 'Backlog' | 'Planning' | 'Running' | 'Review' | 'Done';
export type PhaseStatus = 'working' | 'idle' | 'ready' | 'exited';
export type ConnectionStatus = 'connected' | 'reconnecting' | 'disconnected';
export type OutputBlockType = 'text' | 'tool_call' | 'error';

export interface OutputBlock {
	id: number;
	text: string;
	type: OutputBlockType;
	timestamp: number;
}

export type ServerMessage =
	| { type: 'output'; data: string; offset: number }
	| { type: 'state'; session_state: string; exit_code?: number }
	| { type: 'metrics'; cpu_percent: number; rss_bytes: number; uptime_secs: number }
	| { type: 'error'; message: string }
	| { type: 'connected'; session_id: string; total_bytes: number };

export type ClientMessage =
	| { type: 'write'; input: string }
	| { type: 'resize'; rows: number; cols: number };

export interface Task {
	id: string;
	title: string;
	description: string | null;
	status: TaskStatus;
	agent: string;
	project_id: string;
	session_id: string | null;
	session_name: string | null;
	worktree_path: string | null;
	branch_name: string | null;
	pr_number: number | null;
	pr_url: string | null;
	plugin: string | null;
	cycle: number;
	created_at: string;
	updated_at: string;
}

export interface Project {
	id: string;
	name: string;
	path: string;
	github_url: string | null;
	default_agent: string | null;
	last_opened: string;
}

export interface CreateTaskRequest {
	title: string;
	agent: string;
	project_id: string;
	description?: string;
}

export interface UpdateTaskRequest {
	title?: string;
	description?: string;
	status?: string;
	agent?: string;
}

export const COLUMNS = ['Backlog', 'Planning', 'Running', 'Review', 'Done'] as const;

export const COLUMN_LABELS: Record<TaskStatus, string> = {
	Backlog: 'Backlog',
	Planning: 'Planning',
	Running: 'Running',
	Review: 'Review',
	Done: 'Done'
};
