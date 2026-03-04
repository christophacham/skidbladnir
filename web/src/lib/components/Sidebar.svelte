<script lang="ts">
	import { projectStore } from '$lib/stores/projects.svelte';
	import { taskStore } from '$lib/stores/tasks.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';

	const taskCountsByProject = $derived.by(() => {
		const counts = new Map<string, number>();
		for (const task of taskStore.allTasks) {
			const pid = task.project_id;
			counts.set(pid, (counts.get(pid) ?? 0) + 1);
		}
		return counts;
	});

	function selectProject(id: string) {
		projectStore.setActive(id);
		uiStore.toggleSidebar();
	}
</script>

{#if uiStore.sidebarOpen}
	<aside
		class="shrink-0 flex flex-col h-full overflow-hidden"
		style="
			width: 200px;
			background-color: #141428;
			border-right: 1px solid var(--color-border);
		"
	>
		<!-- Header -->
		<div
			class="flex items-center justify-between px-3 shrink-0"
			style="height: 48px; border-bottom: 1px solid var(--color-border);"
		>
			<span
				class="text-xs font-semibold uppercase tracking-wider"
				style="color: var(--color-column-header);"
			>
				Projects
			</span>
			<button
				onclick={() => uiStore.toggleSidebar()}
				class="w-6 h-6 flex items-center justify-center rounded cursor-pointer hover:opacity-80 transition-opacity"
				style="color: var(--color-dimmed);"
				title="Close sidebar (e)"
			>
				&times;
			</button>
		</div>

		<!-- Project list -->
		<div class="flex-1 overflow-y-auto py-1">
			{#each projectStore.list as project}
				<button
					onclick={() => selectProject(project.id)}
					class="w-full text-left px-3 py-2 text-sm cursor-pointer transition-colors flex items-center justify-between"
					style="
						color: {project.id === projectStore.activeId ? 'var(--color-selected)' : 'var(--color-text)'};
						border-left: 3px solid {project.id === projectStore.activeId ? 'var(--color-accent)' : 'transparent'};
						background-color: {project.id === projectStore.activeId ? 'var(--color-surface-hover)' : 'transparent'};
					"
				>
					<span class="truncate">{project.name}</span>
					<span
						class="text-xs px-1.5 py-0.5 rounded-full shrink-0 ml-2"
						style="
							background-color: var(--color-surface-hover);
							color: var(--color-dimmed);
						"
					>
						{taskCountsByProject.get(project.id) ?? 0}
					</span>
				</button>
			{/each}

			{#if projectStore.list.length === 0}
				<p
					class="px-3 py-4 text-xs text-center"
					style="color: var(--color-dimmed);"
				>
					No projects found
				</p>
			{/if}
		</div>
	</aside>
{/if}
