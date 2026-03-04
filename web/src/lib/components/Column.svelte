<script lang="ts">
	import TaskCard from './TaskCard.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { COLUMN_LABELS } from '$lib/types';
	import type { TaskStatus, Task } from '$lib/types';

	let {
		status,
		tasks,
		collapsed,
		matchingIds,
		searchActive,
		ontoggle
	}: {
		status: TaskStatus;
		tasks: Task[];
		collapsed: boolean;
		matchingIds: Set<string>;
		searchActive: boolean;
		ontoggle: () => void;
	} = $props();
</script>

<div
	class="flex flex-col rounded-lg overflow-hidden h-full"
	style="background-color: color-mix(in srgb, var(--color-surface) 80%, transparent);"
>
	{#if collapsed}
		<!-- Collapsed column -->
		<button
			onclick={ontoggle}
			class="flex flex-col items-center gap-2 py-3 px-1 h-full cursor-pointer hover:opacity-80 transition-opacity"
		>
			<span
				class="text-xs font-bold uppercase"
				style="color: var(--color-column-header); writing-mode: vertical-lr; text-orientation: mixed;"
			>
				{COLUMN_LABELS[status]}
			</span>
			{#if tasks.length > 0}
				<span
					class="inline-flex items-center justify-center w-5 h-5 rounded-full text-xs font-medium"
					style="background-color: var(--color-accent); color: var(--color-bg);"
				>
					{tasks.length}
				</span>
			{/if}
		</button>
	{:else}
		<!-- Expanded column -->
		<div
			class="flex items-center justify-between px-3 py-2 shrink-0"
			style="border-bottom: 1px solid var(--color-border);"
		>
			<div class="flex items-center gap-2">
				<button
					onclick={ontoggle}
					class="text-xs opacity-60 hover:opacity-100 transition-opacity cursor-pointer"
					style="color: var(--color-column-header);"
					title="Collapse column"
				>
					&#x276E;
				</button>
				<h2
					class="text-sm font-semibold uppercase tracking-wide"
					style="color: var(--color-column-header);"
				>
					{COLUMN_LABELS[status]}
				</h2>
				<span
					class="text-xs px-1.5 py-0.5 rounded-full"
					style="color: var(--color-dimmed); background-color: color-mix(in srgb, var(--color-dimmed) 20%, transparent);"
				>
					{tasks.length}
				</span>
			</div>
			{#if status === 'Backlog'}
				<button
					onclick={() => uiStore.openCreateModal()}
					class="w-6 h-6 flex items-center justify-center rounded text-sm cursor-pointer hover:opacity-80 transition-opacity"
					style="color: var(--color-accent); border: 1px solid var(--color-accent);"
					title="Create task"
				>
					+
				</button>
			{/if}
		</div>

		<div class="flex-1 overflow-y-auto p-2 space-y-2">
			{#if tasks.length === 0}
				<p
					class="text-xs text-center py-4 italic"
					style="color: var(--color-dimmed);"
				>
					No tasks
				</p>
			{:else}
				{#each tasks as task (task.id)}
					<TaskCard
						{task}
						dimmed={searchActive && !matchingIds.has(task.id)}
					/>
				{/each}
			{/if}
		</div>
	{/if}
</div>
