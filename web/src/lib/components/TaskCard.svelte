<script lang="ts">
	import type { Task } from '$lib/types';
	import { taskStore } from '$lib/stores/tasks.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { wsStore } from '$lib/stores/websocket.svelte';
	import StatusDot from './StatusDot.svelte';

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

	const phaseStatus = $derived(
		task.session_id ? (wsStore.phaseStatuses.get(task.session_id) ?? null) : null
	);

	const isSelected = $derived(uiStore.selectedTask?.id === task.id);

	let settingUp = $state(false);

	function handleDelete(e: MouseEvent) {
		e.stopPropagation();
		uiStore.openDeleteConfirm(task);
	}

	async function handleAdvance(e: MouseEvent) {
		e.stopPropagation();
		settingUp = true;
		await taskStore.advance(task.id);
		settingUp = false;
	}
</script>

<button
	class="group w-full text-left rounded-lg p-3 cursor-pointer transition-opacity duration-150 relative"
	class:opacity-30={dimmed}
	style="background-color: var(--color-surface); border: 1px solid {isSelected ? 'var(--color-accent)' : 'var(--color-border)'}; {isSelected ? 'border-left: 3px solid var(--color-accent);' : ''}"
	onmouseenter={(e) => {
		(e.currentTarget as HTMLElement).style.backgroundColor = 'var(--color-surface-hover)';
	}}
	onmouseleave={(e) => {
		(e.currentTarget as HTMLElement).style.backgroundColor = 'var(--color-surface)';
	}}
	onclick={() => onclick?.(task)}
>
	<!-- Action buttons (visible on hover) -->
	{#if task.status !== 'Done'}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<span
			class="absolute top-1.5 right-8 w-5 h-5 flex items-center justify-center rounded text-xs opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer hover:bg-blue-600/30"
			style="color: var(--color-dimmed);"
			onclick={handleAdvance}
			role="button"
			tabindex="-1"
			title="Advance task"
		>
			&#x25B6;
		</span>
	{/if}
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
		<span class="shrink-0 mt-1.5">
			<StatusDot status={phaseStatus} />
		</span>
	</div>

	<!-- Row 2: Agent badge -->
	<div class="mt-1.5">
		<span class="text-xs px-2 py-0.5 rounded-full {badgeClass}">
			{task.agent}
		</span>
	</div>

	<!-- Setting up overlay -->
	{#if settingUp}
		<div
			class="absolute inset-0 rounded-lg flex items-center justify-center"
			style="background-color: rgba(0, 0, 0, 0.6);"
		>
			<span class="text-xs animate-pulse" style="color: var(--color-accent);"
				>Setting up...</span
			>
		</div>
	{/if}
</button>
