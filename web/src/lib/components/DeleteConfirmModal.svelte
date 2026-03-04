<script lang="ts">
	import { taskStore } from '$lib/stores/tasks.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';

	let submitting = $state(false);

	function close() {
		uiStore.closeDeleteConfirm();
	}

	async function confirmDelete() {
		if (!uiStore.deleteTarget || submitting) return;
		submitting = true;
		try {
			await taskStore.remove(uiStore.deleteTarget.id);
			close();
		} catch {
			// Error already handled in taskStore
			close();
		} finally {
			submitting = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			close();
		}
		if (e.key === 'Enter') {
			e.preventDefault();
			confirmDelete();
		}
	}

	function handleOverlayClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			close();
		}
	}
</script>

{#if uiStore.deleteTarget}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center"
		style="background-color: rgba(0, 0, 0, 0.5);"
		onclick={handleOverlayClick}
		onkeydown={handleKeydown}
		role="dialog"
		aria-modal="true"
		aria-label="Delete confirmation"
	>
		<div
			class="w-full max-w-sm rounded-lg p-6 shadow-xl"
			style="
				background-color: var(--color-surface);
				border: 1px solid var(--color-popup-border);
			"
		>
			<!-- Content -->
			<p
				class="text-base font-medium mb-2"
				style="color: var(--color-text);"
			>
				Delete '{uiStore.deleteTarget.title}'?
			</p>
			<p
				class="text-sm mb-6"
				style="color: var(--color-dimmed);"
			>
				This action cannot be undone.
			</p>

			<!-- Footer buttons -->
			<div class="flex justify-end gap-3">
				<button
					onclick={close}
					class="px-4 py-2 text-sm rounded-md cursor-pointer hover:opacity-80 transition-opacity"
					style="color: var(--color-dimmed);"
				>
					Cancel
				</button>
				<button
					onclick={confirmDelete}
					disabled={submitting}
					class="px-4 py-2 text-sm rounded-md cursor-pointer hover:bg-red-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
					style="background-color: rgb(220, 38, 38); color: white;"
				>
					{submitting ? 'Deleting...' : 'Delete'}
				</button>
			</div>
		</div>
	</div>
{/if}
