import type { Project } from '$lib/types';
import { fetchProjects } from '$lib/api/projects';

const ACTIVE_PROJECT_KEY = 'agtx-active-project';

class ProjectStore {
	list = $state<Project[]>([]);
	activeId = $state<string | null>(null);

	active = $derived(
		this.list.find((p) => p.id === this.activeId) ?? this.list[0] ?? null
	);

	async load(): Promise<void> {
		try {
			this.list = await fetchProjects();
			const saved = localStorage.getItem(ACTIVE_PROJECT_KEY);
			if (saved && this.list.some((p) => p.id === saved)) {
				this.activeId = saved;
			} else if (this.list.length > 0) {
				this.activeId = this.list[0].id;
			}
		} catch {
			// Projects may not be available yet
		}
	}

	setActive(id: string): void {
		this.activeId = id;
		localStorage.setItem(ACTIVE_PROJECT_KEY, id);
	}
}

export const projectStore = new ProjectStore();
