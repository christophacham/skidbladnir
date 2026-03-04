<script lang="ts">
	import Column from './Column.svelte';
	import { taskStore } from '$lib/stores/tasks.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { COLUMNS } from '$lib/types';

	const gridTemplate = $derived(
		COLUMNS.map((col) =>
			uiStore.collapsedColumns.has(col) ? '48px' : '1fr'
		).join(' ')
	);

	const searchActive = $derived(taskStore.searchQuery.trim().length > 0);
</script>

<div
	class="h-full gap-2 p-2"
	style="display: grid; grid-template-columns: {gridTemplate};"
>
	{#each COLUMNS as status}
		<Column
			{status}
			tasks={taskStore.byStatus[status] ?? []}
			collapsed={uiStore.collapsedColumns.has(status)}
			matchingIds={taskStore.matchingIds}
			{searchActive}
			ontoggle={() => uiStore.toggleColumn(status)}
			ontaskclick={(task) => uiStore.selectTask(task)}
		/>
	{/each}
</div>
