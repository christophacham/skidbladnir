import { api } from './client';
import type {
	AdvanceResult,
	DiffResponse,
	PluginInfo,
	PrResponse,
	PrGenerateResponse,
	PrStatusResponse
} from '$lib/types';

export async function advanceTask(
	taskId: string,
	direction: string = 'next'
): Promise<AdvanceResult> {
	return api<AdvanceResult>(`/workflow/tasks/${taskId}/advance`, {
		method: 'POST',
		body: JSON.stringify({ direction })
	});
}

export async function fetchPlugins(): Promise<PluginInfo[]> {
	return api<PluginInfo[]>('/workflow/plugins');
}

export async function fetchDiff(taskId: string): Promise<DiffResponse> {
	return api<DiffResponse>(`/workflow/tasks/${taskId}/diff`);
}

export async function createPr(
	taskId: string,
	title: string,
	body: string,
	base?: string
): Promise<PrResponse> {
	return api<PrResponse>(`/workflow/tasks/${taskId}/pr`, {
		method: 'POST',
		body: JSON.stringify({ title, body, base })
	});
}

export async function generatePrDescription(taskId: string): Promise<PrGenerateResponse> {
	return api<PrGenerateResponse>(`/workflow/tasks/${taskId}/pr/generate`);
}

export async function fetchPrStatus(taskId: string): Promise<PrStatusResponse> {
	return api<PrStatusResponse>(`/workflow/tasks/${taskId}/pr/status`);
}
