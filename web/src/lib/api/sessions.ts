import { api } from '$lib/api/client';

export interface SessionOutputResponse {
	data: string;
	total_bytes: number;
}

export interface SessionInfo {
	id: string;
	pid: number;
	state: string;
	created_at: string;
	total_bytes: number;
	metrics?: {
		cpu_percent: number;
		rss_bytes: number;
		uptime_secs: number;
	};
}

export async function fetchSessionOutput(
	sessionId: string,
	offset?: number,
	limit?: number
): Promise<SessionOutputResponse> {
	const params = new URLSearchParams();
	if (offset !== undefined) params.set('offset', String(offset));
	if (limit !== undefined) params.set('limit', String(limit));
	const qs = params.toString() ? `?${params.toString()}` : '';
	return api<SessionOutputResponse>(`/sessions/${sessionId}/output${qs}`);
}

export async function fetchSessions(): Promise<SessionInfo[]> {
	return api<SessionInfo[]>('/sessions');
}
