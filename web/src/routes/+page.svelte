<script lang="ts">
	import NavBar from '$lib/components/NavBar.svelte';
	import Board from '$lib/components/Board.svelte';
	import CreateTaskModal from '$lib/components/CreateTaskModal.svelte';
	import DeleteConfirmModal from '$lib/components/DeleteConfirmModal.svelte';
</script>

<div class="flex flex-col h-full">
	<NavBar />
	<main class="flex-1 overflow-hidden">
		<Board />
	</main>
</div>

<CreateTaskModal />
<DeleteConfirmModal />
