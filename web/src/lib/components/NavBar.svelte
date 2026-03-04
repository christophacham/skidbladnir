<script lang="ts">
	import { projectStore } from '$lib/stores/projects.svelte';
	import { taskStore } from '$lib/stores/tasks.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';

	let searchInput = $state<HTMLInputElement | null>(null);

	const searchActive = $derived(taskStore.searchQuery.trim().length > 0);
	const matchCount = $derived(taskStore.matchingIds.size);
	const totalCount = $derived(taskStore.list.length);

	const placeholder = $derived(
		searchActive
			? `${matchCount} of ${totalCount} matches`
			: 'Search tasks... (/)'
	);

	$effect(() => {
		if (uiStore.searchFocused && searchInput) {
			searchInput.focus();
			uiStore.searchFocused = false;
		}
	});

	function handleSearchKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			taskStore.searchQuery = '';
			uiStore.searchFocused = false;
			searchInput?.blur();
		}
	}
</script>

<nav
	class="flex items-center justify-between px-4 shrink-0"
	style="height: 48px; border-bottom: 1px solid var(--color-border);"
>
	<!-- Left: Project name -->
	<div class="flex items-center gap-2">
		<span
			class="text-base font-bold tracking-wide"
			style="color: var(--color-accent);"
		>
			{projectStore.active?.name ?? 'AGTX'}
		</span>
	</div>

	<!-- Center: Search -->
	<div class="flex-1 max-w-md mx-4">
		<input
			bind:this={searchInput}
			bind:value={taskStore.searchQuery}
			onkeydown={handleSearchKeydown}
			type="text"
			placeholder={placeholder}
			class="w-full px-3 py-1.5 rounded-md text-sm outline-none transition-colors"
			style="
				background-color: color-mix(in srgb, var(--color-surface) 60%, transparent);
				color: var(--color-text);
				border: 1px solid var(--color-border);
			"
		/>
	</div>

	<!-- Right: Create button -->
	<button
		onclick={() => uiStore.openCreateModal()}
		class="w-8 h-8 flex items-center justify-center rounded-md text-sm cursor-pointer hover:opacity-80 transition-opacity"
		style="color: var(--color-accent); border: 1px solid var(--color-accent);"
		title="Create task (o)"
	>
		+
	</button>
</nav>
