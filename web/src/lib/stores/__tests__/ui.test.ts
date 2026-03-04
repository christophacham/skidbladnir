import { describe, it, expect } from 'vitest';
import type { Task } from '$lib/types';

// Import the class to create fresh instances per test
// The UiStore constructor reads localStorage, so we test with fresh instances
const { UiStore } = await import('$lib/stores/ui.svelte');

function makeTask(overrides: Partial<Task> = {}): Task {
	return {
		id: 'task-1',
		title: 'Test Task',
		description: null,
		status: 'Backlog',
		agent: 'claude',
		project_id: 'proj-1',
		session_id: null,
		session_name: null,
		worktree_path: null,
		branch_name: null,
		pr_number: null,
		pr_url: null,
		plugin: null,
		cycle: 0,
		created_at: '2026-01-01T00:00:00Z',
		updated_at: '2026-01-01T00:00:00Z',
		...overrides
	};
}

describe('UiStore', () => {
	it('selectTask sets selectedTask to the given task', () => {
		const store = new UiStore();
		const task = makeTask();
		store.selectTask(task);
		expect(store.selectedTask).toEqual(task);
	});

	it('closeDetail sets selectedTask back to null', () => {
		const store = new UiStore();
		store.selectTask(makeTask());
		store.closeDetail();
		expect(store.selectedTask).toBeNull();
	});

	it('detailPanelOpen is true when selectedTask is set, false when null', () => {
		const store = new UiStore();
		expect(store.detailPanelOpen).toBe(false);
		store.selectTask(makeTask());
		expect(store.detailPanelOpen).toBe(true);
		store.closeDetail();
		expect(store.detailPanelOpen).toBe(false);
	});

	it('selecting a different task replaces the previous selectedTask', () => {
		const store = new UiStore();
		const task1 = makeTask({ id: 'task-1', title: 'First' });
		const task2 = makeTask({ id: 'task-2', title: 'Second' });
		store.selectTask(task1);
		expect(store.selectedTask?.id).toBe('task-1');
		store.selectTask(task2);
		expect(store.selectedTask?.id).toBe('task-2');
	});
});
