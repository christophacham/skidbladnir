<script lang="ts">
	import { taskStore } from '$lib/stores/tasks.svelte';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { wsStore } from '$lib/stores/websocket.svelte';
	import { fetchPrStatus } from '$lib/api/workflow';
	import OutputView from './OutputView.svelte';
	import InputBar from './InputBar.svelte';
	import StatusDot from './StatusDot.svelte';
	import TabBar from './TabBar.svelte';

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

	let activeTab = $state<string>('output');
	let prState = $state<string | null>(null);

	const tabs = $derived([
		{ id: 'output', label: 'Output', visible: true },
		{ id: 'diff', label: 'Diff', visible: !!task?.worktree_path },
		{ id: 'pr', label: 'PR', visible: task?.status === 'Review' || !!task?.pr_number }
	]);

	// Reset tab and fetch PR status when selected task changes
	$effect(() => {
		const currentTask = uiStore.selectedTask;
		activeTab = 'output';
		prState = null;
		if (currentTask?.pr_url) {
			fetchPrStatus(currentTask.id)
				.then((r) => {
					prState = r.state;
				})
				.catch(() => {});
		}
	});

	const prBadgeColor = $derived.by(() => {
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
			{#if task.pr_url}
				<a
					href={task.pr_url}
					target="_blank"
					rel="noopener noreferrer"
					class="text-xs px-2 py-0.5 rounded-full shrink-0 hover:opacity-80 transition-opacity {prBadgeColor}"
				>
					PR #{task.pr_number} {prState ?? ''}
				</a>
			{/if}
			<span class="text-xs px-2 py-0.5 rounded-full shrink-0 {badgeClass}">
				{task.agent}
			</span>
			{#if task.status !== 'Done'}
				<button
					onclick={() => taskStore.advance(task.id)}
					class="px-3 py-1.5 text-xs rounded-md cursor-pointer hover:opacity-80 transition-opacity shrink-0"
					style="background-color: var(--color-accent); color: var(--color-bg);"
					title="Advance task"
				>
					Advance &#x25B6;
				</button>
			{/if}
			<button
				onclick={() => uiStore.closeDetail()}
				class="w-7 h-7 flex items-center justify-center rounded text-sm cursor-pointer hover:opacity-80 transition-opacity shrink-0"
				style="color: var(--color-dimmed); border: 1px solid var(--color-border);"
				title="Close (Esc)"
			>
				&#x2715;
			</button>
		</div>

		<!-- Tab bar -->
		{#if task.session_id || task.worktree_path}
			<TabBar {tabs} active={activeTab} onchange={(id) => (activeTab = id)} />
		{/if}

		<!-- Tab content -->
		{#if activeTab === 'output'}
			{#if task.session_id}
				<OutputView />
				<InputBar />
			{:else}
				<div
					class="flex-1 flex flex-col items-center justify-center gap-3"
					style="color: var(--color-dimmed);"
				>
					<span class="text-4xl opacity-40">&#x1F4CB;</span>
					<p class="text-sm">No active session</p>
					{#if task.description}
						<p class="text-xs max-w-sm text-center opacity-70">{task.description}</p>
					{/if}
				</div>
			{/if}
		{:else if activeTab === 'diff'}
			<div
				style="color: var(--color-dimmed);"
				class="flex-1 flex items-center justify-center"
			>
				Diff view coming soon
			</div>
		{:else if activeTab === 'pr'}
			<div
				style="color: var(--color-dimmed);"
				class="flex-1 flex items-center justify-center"
			>
				PR view coming soon
			</div>
		{/if}
	{/if}
</div>
