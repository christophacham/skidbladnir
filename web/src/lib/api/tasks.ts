import { api } from './client';
import type { Task, CreateTaskRequest } from '$lib/types';

export async function fetchTasks(): Promise<Task[]> {
	return api<Task[]>('/tasks');
}

export async function createTask(req: CreateTaskRequest): Promise<Task> {
	return api<Task>('/tasks', {
		method: 'POST',
		body: JSON.stringify(req)
	});
}

export async function deleteTask(id: string): Promise<void> {
	return api<void>(`/tasks/${id}`, {
		method: 'DELETE'
	});
}
