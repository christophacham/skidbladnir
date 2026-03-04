import type { Task, TaskStatus, CreateTaskRequest, AdvanceResult } from '$lib/types';
import { COLUMNS } from '$lib/types';
import { fetchTasks, createTask as apiCreateTask, deleteTask as apiDeleteTask } from '$lib/api/tasks';
import { advanceTask } from '$lib/api/workflow';
import { projectStore } from '$lib/stores/projects.svelte';

class TaskStore {
	list = $state<Task[]>([]);
	loading = $state(false);
	error = $state<string | null>(null);
	searchQuery = $state('');

	/** All tasks unfiltered -- used for sidebar task counts across projects */
	get allTasks(): Task[] {
		return this.list;
	}

	/** Tasks filtered by the active project */
	projectTasks = $derived.by(() => {
		const activeId = projectStore.activeId;
		if (!activeId) return this.list;
		return this.list.filter((t) => t.project_id === activeId);
	});

	byStatus = $derived.by(() => {
		const groups: Record<TaskStatus, Task[]> = {
			Backlog: [],
			Planning: [],
			Running: [],
			Review: [],
			Done: []
		};
		for (const task of this.projectTasks) {
			if (groups[task.status]) {
				groups[task.status].push(task);
			}
		}
		return groups;
	});

	filtered = $derived.by(() => {
		const query = this.searchQuery.toLowerCase().trim();
		if (!query) return this.projectTasks;
		return this.projectTasks.filter((task) => {
			const titleMatch = task.title.toLowerCase().includes(query);
			const descMatch = task.description?.toLowerCase().includes(query) ?? false;
			return titleMatch || descMatch;
		});
	});

	matchingIds = $derived(new Set(this.filtered.map((t) => t.id)));

	async load(): Promise<void> {
		this.loading = true;
		this.error = null;
		try {
			this.list = await fetchTasks();
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Failed to load tasks';
		} finally {
			this.loading = false;
		}
	}

	async create(req: CreateTaskRequest): Promise<Task | undefined> {
		try {
			const task = await apiCreateTask(req);
			this.list.push(task);
			return task;
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Failed to create task';
			return undefined;
		}
	}

	async remove(id: string): Promise<void> {
		try {
			await apiDeleteTask(id);
			this.list = this.list.filter((t) => t.id !== id);
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Failed to delete task';
		}
	}

	async advance(taskId: string, direction: string = 'next'): Promise<AdvanceResult | undefined> {
		try {
			const result = await advanceTask(taskId, direction);
			const idx = this.list.findIndex((t) => t.id === taskId);
			if (idx !== -1) {
				this.list[idx] = result.task;
			}
			return result;
		} catch (e) {
			this.error = e instanceof Error ? e.message : 'Failed to advance task';
			return undefined;
		}
	}

	updateTask(updated: Task): void {
		const idx = this.list.findIndex((t) => t.id === updated.id);
		if (idx !== -1) {
			this.list[idx] = updated;
		}
	}
}

export const taskStore = new TaskStore();
