<script lang="ts">
	import NavBar from '$lib/components/NavBar.svelte';
	import Board from '$lib/components/Board.svelte';
	import DetailPanel from '$lib/components/DetailPanel.svelte';
	import CreateTaskModal from '$lib/components/CreateTaskModal.svelte';
	import DeleteConfirmModal from '$lib/components/DeleteConfirmModal.svelte';
	import CommandPalette from '$lib/components/CommandPalette.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
</script>

<div class="flex flex-col h-full">
	<NavBar />
	<main
		class="flex-1 overflow-hidden"
		style="display: grid; grid-template-columns: {uiStore.detailPanelOpen ? '2fr 3fr' : '1fr'}; transition: grid-template-columns 0.3s ease-in-out;"
	>
		<Board />
		{#if uiStore.selectedTask}
			<DetailPanel />
		{/if}
	</main>
</div>

<CreateTaskModal />
<DeleteConfirmModal />
<CommandPalette />
