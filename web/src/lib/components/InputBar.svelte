<script lang="ts">
	import { wsStore } from '$lib/stores/websocket.svelte';

	let inputText = $state('');

	const disabled = $derived(wsStore.connectionStatus !== 'connected');

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			if (inputText.trim() || inputText.length > 0) {
				wsStore.send({ type: 'write', input: inputText + '\n' });
				inputText = '';
			}
		}
	}
</script>

<div
	class="shrink-0 flex items-center gap-2 px-3 py-2"
	style="border-top: 1px solid var(--color-border); font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;"
>
	<input
		type="text"
		bind:value={inputText}
		onkeydown={handleKeydown}
		placeholder={disabled ? 'Disconnected' : 'Send to agent...'}
		{disabled}
		class="flex-1 px-2 py-1.5 rounded text-sm outline-none transition-opacity"
		class:opacity-50={disabled}
		style="
			background-color: color-mix(in srgb, var(--color-surface) 60%, transparent);
			color: var(--color-text);
			border: 1px solid var(--color-border);
		"
	/>
	<button
		onclick={() => {
			if (inputText.trim() || inputText.length > 0) {
				wsStore.send({ type: 'write', input: inputText + '\n' });
				inputText = '';
			}
		}}
		{disabled}
		class="px-2 py-1.5 rounded text-xs cursor-pointer transition-opacity hover:opacity-80"
		class:opacity-30={disabled}
		style="color: var(--color-accent); border: 1px solid var(--color-accent);"
		title="Send"
	>
		&#x27A4;
	</button>
</div>
