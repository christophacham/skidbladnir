import { projectStore } from '$lib/stores/projects.svelte';
import { uiStore } from '$lib/stores/ui.svelte';
import { COLUMNS } from '$lib/types';

export interface Command {
	id: string;
	label: string;
	shortcut?: string;
	category: string;
	action: () => void;
	keywords?: string[];
}

class CommandStore {
	commands = $state<Command[]>([]);

	constructor() {
		this.register();
	}

	private register() {
		this.commands = [
			// Tasks
			{
				id: 'create-task',
				label: 'Create new task',
				shortcut: 'o',
				category: 'Tasks',
				action: () => uiStore.openCreateModal(),
				keywords: ['new', 'add', 'task']
			},
			{
				id: 'search-tasks',
				label: 'Search tasks',
				shortcut: '/',
				category: 'Tasks',
				action: () => uiStore.focusSearch(),
				keywords: ['find', 'filter']
			},
			// Navigation
			{
				id: 'toggle-sidebar',
				label: 'Toggle project sidebar',
				shortcut: 'e',
				category: 'Navigation',
				action: () => uiStore.toggleSidebar(),
				keywords: ['projects', 'panel']
			},
			// View
			{
				id: 'collapse-all',
				label: 'Collapse all columns',
				category: 'View',
				action: () => {
					for (const col of COLUMNS) {
						if (!uiStore.collapsedColumns.has(col)) {
							uiStore.toggleColumn(col);
						}
					}
				},
				keywords: ['minimize', 'hide']
			},
			{
				id: 'expand-all',
				label: 'Expand all columns',
				category: 'View',
				action: () => {
					for (const col of COLUMNS) {
						if (uiStore.collapsedColumns.has(col)) {
							uiStore.toggleColumn(col);
						}
					}
				},
				keywords: ['maximize', 'show']
			}
		];
	}

	/** Rebuild dynamic commands (project switches) from current project list */
	rebuildProjectCommands() {
		// Remove old project switch commands
		const base = this.commands.filter((c) => !c.id.startsWith('switch-project-'));
		// Add current project list
		const projectCmds: Command[] = projectStore.list.map((p) => ({
			id: `switch-project-${p.id}`,
			label: `Switch to ${p.name}`,
			category: 'Navigation',
			action: () => projectStore.setActive(p.id),
			keywords: ['project', p.name.toLowerCase()]
		}));
		this.commands = [...base, ...projectCmds];
	}
}

export const commandStore = new CommandStore();
