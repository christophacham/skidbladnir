import { api } from './client';
import type { Project } from '$lib/types';

export async function fetchProjects(): Promise<Project[]> {
	return api<Project[]>('/projects');
}
