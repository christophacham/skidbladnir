<script lang="ts">
	import type { Task } from '$lib/types';
	import { uiStore } from '$lib/stores/ui.svelte';

	let {
		task,
		dimmed = false,
		onclick
	}: {
		task: Task;
		dimmed?: boolean;
		onclick?: (task: Task) => void;
	} = $props();

	const agentColors: Record<string, string> = {
		claude: 'bg-purple-500/20 text-purple-300',
		codex: 'bg-green-500/20 text-green-300',
		gemini: 'bg-blue-500/20 text-blue-300',
		copilot: 'bg-gray-500/20 text-gray-300',
		opencode: 'bg-amber-500/20 text-amber-300'
	};

	const badgeClass = $derived(
		agentColors[task.agent.toLowerCase()] ?? 'bg-gray-500/20 text-gray-300'
	);

	// Phase 5: replace with live PhaseStatus from WebSocket
	const statusDotColor = 'bg-gray-500';

	function handleDelete(e: MouseEvent) {
		e.stopPropagation();
		uiStore.openDeleteConfirm(task);
	}
</script>

<button
	class="group w-full text-left rounded-lg p-3 cursor-pointer transition-opacity relative"
	class:opacity-30={dimmed}
	style="background-color: var(--color-surface); border: 1px solid var(--color-border);"
	onmouseenter={(e) => {
		(e.currentTarget as HTMLElement).style.backgroundColor = 'var(--color-surface-hover)';
	}}
	onmouseleave={(e) => {
		(e.currentTarget as HTMLElement).style.backgroundColor = 'var(--color-surface)';
	}}
	onclick={() => onclick?.(task)}
>
	<!-- Delete button (visible on hover) -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<span
		class="absolute top-1.5 right-1.5 w-5 h-5 flex items-center justify-center rounded text-xs opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer hover:bg-red-600/30"
		style="color: var(--color-dimmed);"
		onclick={handleDelete}
		role="button"
		tabindex="-1"
		title="Delete task"
	>
		&#x2715;
	</span>

	<!-- Row 1: Title + Status dot -->
	<div class="flex items-start justify-between gap-2">
		<span
			class="text-sm truncate block flex-1"
			style="color: var(--color-text);"
		>
			{task.title}
		</span>
		<span
			class="w-2 h-2 rounded-full shrink-0 mt-1.5 {statusDotColor}"
			title="Phase status"
		></span>
	</div>

	<!-- Row 2: Agent badge -->
	<div class="mt-1.5">
		<span class="text-xs px-2 py-0.5 rounded-full {badgeClass}">
			{task.agent}
		</span>
	</div>
</button>
