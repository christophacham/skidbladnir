<script lang="ts">
	import type { Task, PrResponse } from '$lib/types';
	import { createPr, generatePrDescription } from '$lib/api/workflow';

	interface Props {
		task: Task;
		onclose: () => void;
		oncreated: (pr: PrResponse) => void;
	}

	let { task, onclose, oncreated }: Props = $props();

	let title = $state('');
	let body = $state('');
	let base = $state('main');
	let generating = $state(false);
	let creating = $state(false);
	let error = $state<string | null>(null);

	// Auto-generate PR description on mount
	$effect(() => {
		generating = true;
		generatePrDescription(task.id)
			.then((result) => {
				title = result.title;
				body = result.body;
			})
			.catch(() => {
				// Leave fields empty for manual entry (per locked decision)
			})
			.finally(() => {
				generating = false;
			});
	});

	async function handleSubmit() {
		if (!title.trim()) {
			error = 'Title is required';
			return;
		}
		creating = true;
		error = null;
		try {
			const pr = await createPr(task.id, title.trim(), body.trim(), base.trim() || undefined);
			oncreated(pr);
			onclose();
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create PR';
		} finally {
			creating = false;
		}
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onclose();
		}
	}

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			onclose();
		}
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	class="fixed inset-0 z-50 flex items-center justify-center"
	style="background-color: rgba(0,0,0,0.6);"
	onkeydown={handleKeydown}
	onclick={handleBackdropClick}
	role="dialog"
	aria-modal="true"
	aria-label="Create Pull Request"
>
	<div
		class="w-full max-w-lg mx-4 rounded-lg overflow-hidden"
		style="background-color: var(--color-bg); border: 1px solid var(--color-border);"
	>
		<!-- Header -->
		<div
			class="flex items-center justify-between px-4 py-3"
			style="border-bottom: 1px solid var(--color-border);"
		>
			<h2 class="text-base font-semibold" style="color: var(--color-text);">
				Create Pull Request
			</h2>
			<button
				onclick={onclose}
				class="w-7 h-7 flex items-center justify-center rounded text-sm cursor-pointer hover:opacity-80 transition-opacity"
				style="color: var(--color-dimmed); border: 1px solid var(--color-border);"
				title="Close (Esc)"
			>
				&#x2715;
			</button>
		</div>

		<!-- Body -->
		<div class="p-4 space-y-4 relative">
			{#if generating}
				<div
					class="absolute inset-0 flex items-center justify-center z-10 rounded"
					style="background-color: rgba(0,0,0,0.4);"
				>
					<div class="flex items-center gap-2" style="color: var(--color-dimmed);">
						<span
							class="inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin"
						></span>
						<span class="text-sm">Generating description...</span>
					</div>
				</div>
			{/if}

			<!-- Title -->
			<div>
				<label
					for="pr-title"
					class="block text-xs mb-1 font-medium"
					style="color: var(--color-dimmed);">Title</label
				>
				<input
					id="pr-title"
					type="text"
					bind:value={title}
					placeholder="PR title"
					class="w-full px-3 py-2 rounded text-sm outline-none"
					style="background-color: rgba(255,255,255,0.05); color: var(--color-text); border: 1px solid var(--color-border);"
				/>
			</div>

			<!-- Body -->
			<div>
				<label
					for="pr-body"
					class="block text-xs mb-1 font-medium"
					style="color: var(--color-dimmed);">Description</label
				>
				<textarea
					id="pr-body"
					bind:value={body}
					placeholder="PR description (markdown supported)"
					rows="6"
					class="w-full px-3 py-2 rounded text-sm outline-none resize-y"
					style="background-color: rgba(255,255,255,0.05); color: var(--color-text); border: 1px solid var(--color-border); min-height: 150px;"
				></textarea>
			</div>

			<!-- Base branch -->
			<div>
				<label
					for="pr-base"
					class="block text-xs mb-1 font-medium"
					style="color: var(--color-dimmed);">Base branch</label
				>
				<input
					id="pr-base"
					type="text"
					bind:value={base}
					placeholder="main"
					class="w-full px-3 py-2 rounded text-sm outline-none"
					style="background-color: rgba(255,255,255,0.05); color: var(--color-text); border: 1px solid var(--color-border);"
				/>
			</div>

			<!-- Error message -->
			{#if error}
				<p class="text-xs text-red-400">{error}</p>
			{/if}
		</div>

		<!-- Footer -->
		<div
			class="flex items-center justify-end gap-3 px-4 py-3"
			style="border-top: 1px solid var(--color-border);"
		>
			<button
				onclick={onclose}
				class="px-4 py-2 text-sm rounded cursor-pointer hover:opacity-80 transition-opacity"
				style="color: var(--color-dimmed); border: 1px solid var(--color-border);"
			>
				Cancel
			</button>
			<button
				onclick={handleSubmit}
				disabled={creating || generating}
				class="px-4 py-2 text-sm rounded cursor-pointer hover:opacity-80 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
				style="background-color: var(--color-accent); color: var(--color-bg);"
			>
				{#if creating}
					Creating...
				{:else}
					Create PR
				{/if}
			</button>
		</div>
	</div>
</div>
