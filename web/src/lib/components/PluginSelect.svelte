<script lang="ts">
	import type { PluginInfo } from '$lib/types';
	import { fetchPlugins } from '$lib/api/workflow';

	let {
		value = $bindable(''),
		label = 'Plugin'
	}: {
		value: string;
		label?: string;
	} = $props();

	let plugins = $state<PluginInfo[]>([]);

	$effect(() => {
		fetchPlugins()
			.then((p) => {
				plugins = p;
			})
			.catch(() => {});
	});
</script>

<label
	for="task-plugin"
	class="block text-xs font-medium mb-1"
	style="color: var(--color-dimmed);"
>
	{label}
</label>
<select
	bind:value
	id="task-plugin"
	class="w-full px-3 py-2 rounded-md text-sm outline-none cursor-pointer"
	style="
		background-color: var(--color-bg);
		color: var(--color-text);
		border: 1px solid var(--color-border);
	"
>
	<option value="">Default</option>
	{#each plugins as plugin}
		<option value={plugin.name}>{plugin.name} -- {plugin.description}</option>
	{/each}
</select>
