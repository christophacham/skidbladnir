<script lang="ts">
	import { wsStore } from '$lib/stores/websocket.svelte';

	let container = $state<HTMLDivElement | null>(null);
	let userScrolledUp = $state(false);

	function handleScroll() {
		if (!container) return;
		const { scrollHeight, scrollTop, clientHeight } = container;
		userScrolledUp = scrollHeight - scrollTop - clientHeight > 50;
	}

	function scrollToBottom() {
		if (container) {
			container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
			userScrolledUp = false;
		}
	}

	$effect(() => {
		// Track output length changes to auto-scroll
		const _len = wsStore.outputBlocks.length;
		if (!userScrolledUp && container) {
			queueMicrotask(() => {
				container?.scrollTo({ top: container.scrollHeight });
			});
		}
	});

	function borderClass(type: string): string {
		switch (type) {
			case 'tool_call':
				return 'border-l-2 border-cyan-500';
			case 'error':
				return 'border-l-2 border-red-500';
			default:
				return '';
		}
	}
</script>

<div class="relative flex-1 overflow-hidden">
	<div
		bind:this={container}
		onscroll={handleScroll}
		class="h-full overflow-y-auto"
		style="font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;"
	>
		{#each wsStore.outputBlocks as block (block.id)}
			<div
				class="px-3 py-0.5 whitespace-pre-wrap break-words text-sm leading-relaxed {borderClass(block.type)}"
				style={block.type === 'error' ? 'color: var(--color-error, #f87171);' : ''}
			>
				{block.text}
			</div>
		{/each}
	</div>

	{#if userScrolledUp}
		<button
			onclick={scrollToBottom}
			class="absolute bottom-4 right-4 w-8 h-8 rounded-full flex items-center justify-center cursor-pointer transition-opacity hover:opacity-100"
			style="background-color: rgba(255, 255, 255, 0.15); color: var(--color-text);"
			title="Jump to bottom"
		>
			<svg class="w-4 h-4" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
				<polyline points="4,6 8,10 12,6" />
			</svg>
		</button>
	{/if}
</div>
