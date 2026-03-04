<script lang="ts">
	import type { PhaseStatus } from '$lib/types';

	let {
		status = null,
		size = 'sm'
	}: {
		status?: PhaseStatus | null;
		size?: 'sm' | 'md';
	} = $props();

	const sizeClass = $derived(size === 'md' ? 'w-3 h-3' : 'w-2 h-2');

	const dotStyle = $derived.by(() => {
		switch (status) {
			case 'working':
				return 'background-color: #4ade80;';
			case 'idle':
				return 'background-color: #facc15;';
			case 'ready':
				return 'background-color: #4ade80;';
			case 'exited':
				return 'background-color: #6b7280;';
			default:
				return 'background-color: #6b7280;';
		}
	});

	const animClass = $derived(status === 'working' ? 'status-dot-working' : '');
</script>

<span
	class="inline-flex items-center justify-center rounded-full shrink-0 {sizeClass} {animClass}"
	style={dotStyle}
	title={status ?? 'no session'}
>
	{#if status === 'ready'}
		<svg
			class={size === 'md' ? 'w-2 h-2' : 'w-1.5 h-1.5'}
			viewBox="0 0 12 12"
			fill="none"
			stroke="white"
			stroke-width="2.5"
			stroke-linecap="round"
			stroke-linejoin="round"
		>
			<polyline points="2,6 5,9 10,3" />
		</svg>
	{/if}
</span>
