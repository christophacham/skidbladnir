<script lang="ts">
	type Tab = { id: string; label: string; visible?: boolean };
	let {
		tabs,
		active,
		onchange
	}: {
		tabs: Tab[];
		active: string;
		onchange: (id: string) => void;
	} = $props();
	const visibleTabs = $derived(tabs.filter((t) => t.visible !== false));
</script>

<div class="flex shrink-0" style="border-bottom: 1px solid var(--color-border);">
	{#each visibleTabs as tab}
		<button
			class="px-4 py-2 text-sm cursor-pointer transition-colors"
			class:font-semibold={active === tab.id}
			style="color: {active === tab.id
				? 'var(--color-text)'
				: 'var(--color-dimmed)'}; border-bottom: 2px solid {active === tab.id
				? 'var(--color-accent)'
				: 'transparent'};"
			onclick={() => onchange(tab.id)}
		>
			{tab.label}
		</button>
	{/each}
</div>
