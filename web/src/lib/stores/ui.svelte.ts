import type { Task } from '$lib/types';

const COLLAPSED_KEY = 'agtx-collapsed-columns';

export class UiStore {
	sidebarOpen = $state(false);
	collapsedColumns = $state<Set<string>>(new Set());
	createModalOpen = $state(false);
	deleteTarget = $state<Task | null>(null);
	commandPaletteOpen = $state(false);
	searchFocused = $state(false);
	selectedTask = $state<Task | null>(null);
	detailPanelOpen = $derived(this.selectedTask !== null);

	constructor() {
		try {
			const saved = JSON.parse(localStorage.getItem(COLLAPSED_KEY) || '[]');
			if (Array.isArray(saved)) {
				this.collapsedColumns = new Set(saved);
			}
		} catch {
			// Ignore invalid localStorage data
		}
	}

	toggleSidebar(): void {
		this.sidebarOpen = !this.sidebarOpen;
	}

	toggleColumn(status: string): void {
		const next = new Set(this.collapsedColumns);
		if (next.has(status)) {
			next.delete(status);
		} else {
			next.add(status);
		}
		this.collapsedColumns = next;
		localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...next]));
	}

	openCreateModal(): void {
		this.createModalOpen = true;
	}

	closeCreateModal(): void {
		this.createModalOpen = false;
	}

	openDeleteConfirm(task: Task): void {
		this.deleteTarget = task;
	}

	closeDeleteConfirm(): void {
		this.deleteTarget = null;
	}

	toggleCommandPalette(): void {
		this.commandPaletteOpen = !this.commandPaletteOpen;
	}

	focusSearch(): void {
		this.searchFocused = true;
	}

	selectTask(task: Task): void {
		this.selectedTask = task;
	}

	closeDetail(): void {
		this.selectedTask = null;
	}
}

export const uiStore = new UiStore();
