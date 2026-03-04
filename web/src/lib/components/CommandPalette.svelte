<script lang="ts">
	import Fuse from 'fuse.js';
	import { uiStore } from '$lib/stores/ui.svelte';
	import { commandStore, type Command } from '$lib/stores/commands.svelte';
	import { projectStore } from '$lib/stores/projects.svelte';

	let query = $state('');
	let selectedIndex = $state(0);
	let searchInput = $state<HTMLInputElement | null>(null);

	// Rebuild dynamic project commands when project list changes
	$effect(() => {
		if (projectStore.list.length > 0) {
			commandStore.rebuildProjectCommands();
		}
	});

	const fuse = $derived(
		new Fuse(commandStore.commands, {
			keys: ['label', 'keywords', 'category'],
			threshold: 0.4
		})
	);

	const results = $derived.by((): Command[] => {
		if (!query.trim()) {
			return commandStore.commands;
		}
		return fuse.search(query).map((r) => r.item);
	});

	// Group results by category
	const grouped = $derived.by(() => {
		const groups = new Map<string, Command[]>();
		for (const cmd of results) {
			const group = groups.get(cmd.category) ?? [];
			group.push(cmd);
			groups.set(cmd.category, group);
		}
		return groups;
	});

	// Flat list for keyboard navigation
	const flatResults = $derived(results);

	// Reset state when palette opens
	$effect(() => {
		if (uiStore.commandPaletteOpen) {
			query = '';
			selectedIndex = 0;
			queueMicrotask(() => searchInput?.focus());
		}
	});

	// Clamp selectedIndex when results change
	$effect(() => {
		if (selectedIndex >= flatResults.length) {
			selectedIndex = Math.max(0, flatResults.length - 1);
		}
	});

	function close() {
		uiStore.toggleCommandPalette();
	}

	function execute(cmd: Command) {
		cmd.action();
		close();
	}

	function handleKeydown(e: KeyboardEvent) {
		switch (e.key) {
			case 'ArrowDown':
				e.preventDefault();
				selectedIndex = (selectedIndex + 1) % Math.max(1, flatResults.length);
				break;
			case 'ArrowUp':
				e.preventDefault();
				selectedIndex =
					(selectedIndex - 1 + flatResults.length) % Math.max(1, flatResults.length);
				break;
			case 'Enter':
				e.preventDefault();
				if (flatResults[selectedIndex]) {
					execute(flatResults[selectedIndex]);
				}
				break;
			case 'Escape':
				e.preventDefault();
				close();
				break;
		}
	}

	function handleOverlayClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			close();
		}
	}

	function getItemIndex(cmd: Command): number {
		return flatResults.indexOf(cmd);
	}
</script>

{#if uiStore.commandPaletteOpen}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_interactive_supports_focus -->
	<div
		class="fixed inset-0 z-50 flex justify-center"
		style="background-color: rgba(0, 0, 0, 0.5); padding-top: 20vh;"
		onclick={handleOverlayClick}
		onkeydown={handleKeydown}
		role="dialog"
		aria-modal="true"
		aria-label="Command palette"
	>
		<div
			class="w-full max-w-lg rounded-lg shadow-xl overflow-hidden self-start"
			style="
				background-color: var(--color-surface);
				border: 1px solid var(--color-popup-border);
			"
		>
			<!-- Search input -->
			<div style="border-bottom: 1px solid var(--color-border);">
				<input
					bind:this={searchInput}
					bind:value={query}
					type="text"
					placeholder="Type a command..."
					class="w-full px-4 py-3 text-sm outline-none"
					style="
						background-color: var(--color-bg);
						color: var(--color-text);
					"
				/>
			</div>

			<!-- Results -->
			<div class="overflow-y-auto" style="max-height: 320px;">
				{#if flatResults.length === 0}
					<p
						class="px-4 py-6 text-center text-sm"
						style="color: var(--color-dimmed);"
					>
						No matching commands
					</p>
				{:else}
					{#each [...grouped] as [category, cmds]}
						<div class="px-3 pt-2 pb-1">
							<span
								class="text-[10px] font-semibold uppercase tracking-wider"
								style="color: var(--color-dimmed);"
							>
								{category}
							</span>
						</div>
						{#each cmds as cmd}
							{@const idx = getItemIndex(cmd)}
							<button
								onclick={() => execute(cmd)}
								onmouseenter={() => (selectedIndex = idx)}
								class="w-full text-left px-4 py-2 text-sm flex items-center justify-between cursor-pointer transition-colors"
								style="
									color: var(--color-text);
									background-color: {idx === selectedIndex ? 'var(--color-surface-hover)' : 'transparent'};
								"
							>
								<span>{cmd.label}</span>
								{#if cmd.shortcut}
									<kbd
										class="text-[10px] px-1.5 py-0.5 rounded"
										style="
											background-color: var(--color-bg);
											color: var(--color-dimmed);
											border: 1px solid var(--color-border);
										"
									>
										{cmd.shortcut}
									</kbd>
								{/if}
							</button>
						{/each}
					{/each}
				{/if}
			</div>
		</div>
	</div>
{/if}
