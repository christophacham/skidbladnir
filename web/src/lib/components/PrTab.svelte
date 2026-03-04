<script lang="ts">
	import type { Task, PrResponse } from '$lib/types';
	import { fetchPrStatus } from '$lib/api/workflow';
	import { taskStore } from '$lib/stores/tasks.svelte';
	import PrModal from './PrModal.svelte';

	interface Props {
		task: Task;
	}

	let { task }: Props = $props();

	let prState = $state<string | null>(null);
	let loading = $state(false);
	let showPrModal = $state(false);

	// Fetch PR status when task changes
	$effect(() => {
		const t = task;
		prState = null;
		if (t.pr_number) {
			loading = true;
			fetchPrStatus(t.id)
				.then((r) => {
					prState = r.state;
				})
				.catch(() => {
					prState = 'unknown';
				})
				.finally(() => {
					loading = false;
				});
		}
	});

	function refreshStatus() {
		if (!task.pr_number) return;
		loading = true;
		fetchPrStatus(task.id)
			.then((r) => {
				prState = r.state;
			})
			.catch(() => {
				prState = 'unknown';
			})
			.finally(() => {
				loading = false;
			});
	}

	function handlePrCreated(pr: PrResponse) {
		const updated = { ...task, pr_number: pr.pr_number, pr_url: pr.pr_url };
		taskStore.updateTask(updated);
		showPrModal = false;
		prState = 'open';
	}

	const badgeColor = $derived.by(() => {
		switch (prState) {
			case 'open':
				return 'bg-green-500/20 text-green-300';
			case 'merged':
				return 'bg-purple-500/20 text-purple-300';
			case 'closed':
				return 'bg-red-500/20 text-red-300';
			default:
				return 'bg-gray-500/20 text-gray-300';
		}
	});

	const canCreatePr = $derived(
		task.status === 'Review' || (!!task.worktree_path && !task.pr_number)
	);
</script>

<div class="flex-1 overflow-auto p-4" style="color: var(--color-text);">
	{#if task.pr_number && task.pr_url}
		<!-- PR exists: show status -->
		<div class="space-y-4">
			<div class="flex items-center gap-3">
				<span class="text-xs px-2 py-0.5 rounded-full {badgeColor}">
					{#if loading}
						<span
							class="inline-block w-3 h-3 border border-current border-t-transparent rounded-full animate-spin"
						></span>
					{:else}
						{prState ?? 'unknown'}
					{/if}
				</span>
				<a
					href={task.pr_url}
					target="_blank"
					rel="noopener noreferrer"
					class="text-sm hover:underline"
					style="color: var(--color-accent);"
				>
					#{task.pr_number}
				</a>
			</div>

			<div class="text-sm" style="color: var(--color-dimmed);">
				<a
					href={task.pr_url}
					target="_blank"
					rel="noopener noreferrer"
					class="hover:underline break-all"
					style="color: var(--color-accent);"
				>
					{task.pr_url}
				</a>
			</div>

			<button
				onclick={refreshStatus}
				disabled={loading}
				class="px-3 py-1.5 text-xs rounded cursor-pointer hover:opacity-80 transition-opacity disabled:opacity-50"
				style="color: var(--color-dimmed); border: 1px solid var(--color-border);"
			>
				{#if loading}
					Refreshing...
				{:else}
					Refresh Status
				{/if}
			</button>
		</div>
	{:else}
		<!-- No PR: show create prompt -->
		<div class="flex flex-col items-center justify-center h-full gap-3">
			<span class="text-3xl opacity-40" style="color: var(--color-dimmed);">PR</span>
			<p class="text-sm" style="color: var(--color-dimmed);">No pull request yet</p>
			{#if canCreatePr}
				<button
					onclick={() => (showPrModal = true)}
					class="px-4 py-2 text-sm rounded cursor-pointer hover:opacity-80 transition-opacity"
					style="background-color: var(--color-accent); color: var(--color-bg);"
				>
					Create PR
				</button>
			{/if}
		</div>
	{/if}
</div>

{#if showPrModal}
	<PrModal {task} onclose={() => (showPrModal = false)} oncreated={handlePrCreated} />
{/if}
