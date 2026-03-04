<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { projectStore } from '$lib/stores/projects.svelte';
	import { taskStore } from '$lib/stores/tasks.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import Sidebar from '$lib/components/Sidebar.svelte';

	let { children } = $props();

	onMount(() => {
		projectStore.load();
		taskStore.load();
	});

	function handleKeydown(e: KeyboardEvent) {
		const target = e.target as HTMLElement;
		const isInput = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;

		if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
			e.preventDefault();
			uiStore.toggleCommandPalette();
			return;
		}

		if (isInput) return;

		switch (e.key) {
			case 'o':
				uiStore.openCreateModal();
				break;
			case 'e':
				uiStore.toggleSidebar();
				break;
			case '/':
				e.preventDefault();
				uiStore.focusSearch();
				break;
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex h-screen overflow-hidden">
	<Sidebar />
	<div class="flex-1 flex flex-col min-w-0 h-full">
		{@render children()}
	</div>
</div>
