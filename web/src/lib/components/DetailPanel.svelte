<script lang="ts">
	import { uiStore } from '$lib/stores/ui.svelte';
	import { wsStore } from '$lib/stores/websocket.svelte';
	import OutputView from './OutputView.svelte';
	import InputBar from './InputBar.svelte';
	import StatusDot from './StatusDot.svelte';

	const agentColors: Record<string, string> = {
		claude: 'bg-purple-500/20 text-purple-300',
		codex: 'bg-green-500/20 text-green-300',
		gemini: 'bg-blue-500/20 text-blue-300',
		copilot: 'bg-gray-500/20 text-gray-300',
		opencode: 'bg-amber-500/20 text-amber-300'
	};

	const task = $derived(uiStore.selectedTask);

	const badgeClass = $derived(
		task ? (agentColors[task.agent.toLowerCase()] ?? 'bg-gray-500/20 text-gray-300') : ''
	);

	const phaseStatus = $derived(
		task?.session_id ? (wsStore.phaseStatuses.get(task.session_id) ?? null) : null
	);

	// Reactive connection management
	$effect(() => {
		const currentTask = uiStore.selectedTask;
		if (currentTask?.session_id) {
			wsStore.connect(currentTask.session_id);
		} else {
			wsStore.disconnect();
			wsStore.clearOutput();
		}

		return () => {
			wsStore.disconnect();
		};
	});

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			uiStore.closeDetail();
		}
	}

	let panel = $state<HTMLDivElement | null>(null);

	$effect(() => {
		if (panel && uiStore.selectedTask) {
			panel.focus();
		}
	});
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
	bind:this={panel}
	onkeydown={handleKeydown}
	tabindex="0"
	class="flex flex-col h-full outline-none"
	style="border-left: 1px solid var(--color-border); background-color: var(--color-bg);"
>
	{#if task}
		<!-- Header -->
		<div
			class="flex items-center gap-3 px-4 py-3 shrink-0"
			style="border-bottom: 1px solid var(--color-border);"
		>
			<StatusDot status={phaseStatus} size="md" />
			<h2
				class="text-lg font-semibold truncate flex-1"
				style="color: var(--color-text);"
			>
				{task.title}
			</h2>
			<span class="text-xs px-2 py-0.5 rounded-full shrink-0 {badgeClass}">
				{task.agent}
			</span>
			<button
				onclick={() => uiStore.closeDetail()}
				class="w-7 h-7 flex items-center justify-center rounded text-sm cursor-pointer hover:opacity-80 transition-opacity shrink-0"
				style="color: var(--color-dimmed); border: 1px solid var(--color-border);"
				title="Close (Esc)"
			>
				&#x2715;
			</button>
		</div>

		<!-- Body -->
		{#if task.session_id}
			<OutputView />
			<InputBar />
		{:else}
			<div class="flex-1 flex flex-col items-center justify-center gap-3" style="color: var(--color-dimmed);">
				<span class="text-4xl opacity-40">&#x1F4CB;</span>
				<p class="text-sm">No active session</p>
				{#if task.description}
					<p class="text-xs max-w-sm text-center opacity-70">{task.description}</p>
				{/if}
			</div>
		{/if}
	{/if}
</div>
