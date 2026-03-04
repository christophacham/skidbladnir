import { describe, it, expect } from 'vitest';

describe('Workflow API client', () => {
	it.todo('advanceTask sends POST with direction and returns updated task');
	it.todo('fetchPlugins returns array of PluginInfo');
	it.todo('fetchDiff returns DiffResponse with diff string');
	it.todo('createPr sends title/body and returns pr_number/pr_url');
	it.todo('generatePrDescription returns title and body strings');
	it.todo('fetchPrStatus returns state string');
});

describe('TaskStore.advance', () => {
	it.todo('updates task in list after successful advance');
	it.todo('sets error on failed advance');
	it.todo('passes direction parameter for cyclic advance');
});

describe('Command palette workflow actions', () => {
	it.todo('includes Advance Task action when task is selected');
	it.todo('includes Create PR action when task is selected');
});
