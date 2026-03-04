import type { Task, TaskStatus, CreateTaskRequest } from '$lib/types';
import { COLUMNS } from '$lib/types';
import { fetchTasks, createTask as apiCreateTask, deleteTask as apiDeleteTask } from '$lib/api/tasks';

class TaskStore {
	list = $state<Task[]>([]);
	loading = $state(false);
	error = $state<string | null>(null);
	searchQuery = $state('');

	byStatus = $derived.by(() => {
		const groups: Record<TaskStatus, Task[]> = {
			Backlog: [],
			Planning: [],
			Running: [],
			Review: [],
			Done: []
		};
		for (const task of this.list) {
			if (groups[task.status]) {
				groups[task.status].push(task);
			}
		}
		return groups;
	});

	filtered = $derived.by(() => {
		const query = this.searchQuery.toLowerCase().trim();
		if (!query) return this.list;
		return this.list.filter((task) => {
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
}

export const taskStore = new TaskStore();
