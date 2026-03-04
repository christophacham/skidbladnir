<script lang="ts">
	import { taskStore } from '$lib/stores/tasks.svelte';
	import { projectStore } from '$lib/stores/projects.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import PluginSelect from './PluginSelect.svelte';

	const AGENTS = ['claude', 'codex', 'gemini', 'copilot', 'opencode'];

	let title = $state('');
	let agent = $state(projectStore.active?.default_agent ?? 'claude');
	let plugin = $state('');
	let description = $state('');
	let error = $state<string | null>(null);
	let submitting = $state(false);

	let titleInput = $state<HTMLInputElement | null>(null);

	// Reset form when modal opens
	$effect(() => {
		if (uiStore.createModalOpen) {
			title = '';
			agent = projectStore.active?.default_agent ?? 'claude';
			plugin = '';
			description = '';
			error = null;
			submitting = false;
			// Focus title input after DOM update
			queueMicrotask(() => titleInput?.focus());
		}
	});

	function close() {
		uiStore.closeCreateModal();
	}

	async function submit() {
		if (!title.trim() || submitting) return;
		submitting = true;
		error = null;
		try {
			await taskStore.create({
				title: title.trim(),
				agent,
				project_id: projectStore.active?.id ?? '',
				description: description.trim() || undefined,
				plugin: plugin || undefined
			});
			close();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create task';
		} finally {
			submitting = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			close();
		}
	}

	function handleTitleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			submit();
		}
	}

	function handleOverlayClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			close();
		}
	}
</script>

{#if uiStore.createModalOpen}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<div
		class="fixed inset-0 z-50 flex items-center justify-center"
		style="background-color: rgba(0, 0, 0, 0.5);"
		onclick={handleOverlayClick}
		onkeydown={handleKeydown}
		role="dialog"
		aria-modal="true"
		aria-label="Create task"
	>
		<div
			class="w-full max-w-md rounded-lg p-6 shadow-xl"
			style="
				background-color: var(--color-surface);
				border: 1px solid var(--color-popup-border);
			"
		>
			<!-- Header -->
			<h2
				class="text-lg font-semibold mb-4"
				style="color: var(--color-popup-header);"
			>
				Create Task
			</h2>

			<!-- Form -->
			<form
				onsubmit={(e) => {
					e.preventDefault();
					submit();
				}}
				class="space-y-4"
			>
				<!-- Title -->
				<div>
					<label
						for="task-title"
						class="block text-xs font-medium mb-1"
						style="color: var(--color-dimmed);"
					>
						Title
					</label>
					<input
						bind:this={titleInput}
						bind:value={title}
						onkeydown={handleTitleKeydown}
						id="task-title"
						type="text"
						placeholder="Task title"
						required
						class="w-full px-3 py-2 rounded-md text-sm outline-none"
						style="
							background-color: var(--color-bg);
							color: var(--color-text);
							border: 1px solid var(--color-border);
						"
					/>
				</div>

				<!-- Agent -->
				<div>
					<label
						for="task-agent"
						class="block text-xs font-medium mb-1"
						style="color: var(--color-dimmed);"
					>
						Agent
					</label>
					<select
						bind:value={agent}
						id="task-agent"
						class="w-full px-3 py-2 rounded-md text-sm outline-none cursor-pointer"
						style="
							background-color: var(--color-bg);
							color: var(--color-text);
							border: 1px solid var(--color-border);
						"
					>
						{#each AGENTS as a}
							<option value={a}>{a}</option>
						{/each}
					</select>
				</div>

				<!-- Plugin -->
				<div>
					<PluginSelect bind:value={plugin} />
				</div>

				<!-- Description -->
				<div>
					<label
						for="task-description"
						class="block text-xs font-medium mb-1"
						style="color: var(--color-dimmed);"
					>
						Description
					</label>
					<textarea
						bind:value={description}
						id="task-description"
						placeholder="Description (optional)"
						rows="3"
						class="w-full px-3 py-2 rounded-md text-sm outline-none resize-y"
						style="
							background-color: var(--color-bg);
							color: var(--color-text);
							border: 1px solid var(--color-border);
						"
					></textarea>
				</div>

				<!-- Error -->
				{#if error}
					<p class="text-sm text-red-400">{error}</p>
				{/if}

				<!-- Footer buttons -->
				<div class="flex justify-end gap-3 pt-2">
					<button
						type="button"
						onclick={close}
						class="px-4 py-2 text-sm rounded-md cursor-pointer hover:opacity-80 transition-opacity"
						style="color: var(--color-dimmed);"
					>
						Cancel
					</button>
					<button
						type="submit"
						disabled={!title.trim() || submitting}
						class="px-4 py-2 text-sm rounded-md cursor-pointer hover:opacity-80 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
						style="
							background-color: var(--color-accent);
							color: var(--color-bg);
						"
					>
						{submitting ? 'Creating...' : 'Create'}
					</button>
				</div>
			</form>
		</div>
	</div>
{/if}
