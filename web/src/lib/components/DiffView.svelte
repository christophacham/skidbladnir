<script lang="ts">
	import { fetchDiff } from '$lib/api/workflow';
	import { createHighlighter, type Highlighter } from 'shiki';

	interface Props {
		taskId: string;
	}

	let { taskId }: Props = $props();

	// --- Diff parsing types ---

	interface DiffLine {
		content: string;
		type: 'add' | 'remove' | 'context';
	}

	interface DiffHunk {
		header: string;
		lines: DiffLine[];
	}

	interface DiffFile {
		path: string;
		hunks: DiffHunk[];
	}

	// --- Extension to Shiki language map ---

	const EXT_MAP: Record<string, string> = {
		rs: 'rust',
		ts: 'typescript',
		tsx: 'tsx',
		js: 'javascript',
		jsx: 'jsx',
		py: 'python',
		svelte: 'svelte',
		toml: 'toml',
		yaml: 'yaml',
		yml: 'yaml',
		json: 'json',
		md: 'markdown',
		css: 'css',
		html: 'html',
		sh: 'bash',
		go: 'go',
		rb: 'ruby',
		java: 'java',
		c: 'c',
		cpp: 'cpp',
		h: 'c'
	};

	function detectLang(path: string): string {
		const ext = path.split('.').pop() || '';
		return EXT_MAP[ext] || 'text';
	}

	// --- Shiki highlighter singleton ---

	let highlighterPromise: Promise<Highlighter> | null = null;
	function getHighlighter(): Promise<Highlighter> {
		if (!highlighterPromise) {
			highlighterPromise = createHighlighter({
				themes: ['github-dark'],
				langs: [
					'rust',
					'typescript',
					'javascript',
					'python',
					'svelte',
					'toml',
					'yaml',
					'json',
					'markdown',
					'css',
					'html',
					'bash',
					'tsx',
					'jsx'
				]
			});
		}
		return highlighterPromise;
	}

	// --- Unified diff parser ---

	function parseDiff(raw: string): DiffFile[] {
		const files: DiffFile[] = [];
		// Split on "diff --git" markers
		const fileSections = raw.split(/^diff --git /m).filter((s) => s.trim());

		for (const section of fileSections) {
			const lines = section.split('\n');
			let path = '';

			// Extract path from "+++ b/..." line
			for (const line of lines) {
				if (line.startsWith('+++ b/')) {
					path = line.slice(6);
					break;
				} else if (line.startsWith('+++ /dev/null')) {
					// Deleted file - use "--- a/" path
					for (const l of lines) {
						if (l.startsWith('--- a/')) {
							path = l.slice(6) + ' (deleted)';
							break;
						}
					}
					break;
				}
			}

			if (!path) {
				// Try to extract from the first line "a/path b/path"
				const match = lines[0]?.match(/a\/(.*?) b\//);
				if (match) path = match[1];
				else continue;
			}

			// Parse hunks
			const hunks: DiffHunk[] = [];
			let currentHunk: DiffHunk | null = null;

			for (const line of lines) {
				if (line.startsWith('@@')) {
					if (currentHunk) hunks.push(currentHunk);
					currentHunk = { header: line, lines: [] };
				} else if (currentHunk) {
					if (line.startsWith('+')) {
						currentHunk.lines.push({ content: line.slice(1), type: 'add' });
					} else if (line.startsWith('-')) {
						currentHunk.lines.push({ content: line.slice(1), type: 'remove' });
					} else if (line.startsWith(' ')) {
						currentHunk.lines.push({ content: line.slice(1), type: 'context' });
					} else if (line === '\\ No newline at end of file') {
						// Ignore this marker
					} else if (line.length === 0 && currentHunk.lines.length > 0) {
						// Empty line in diff context
						currentHunk.lines.push({ content: '', type: 'context' });
					}
				}
			}
			if (currentHunk) hunks.push(currentHunk);

			files.push({ path, hunks });
		}

		return files;
	}

	// --- Highlighting logic ---

	async function highlightHunk(
		highlighter: Highlighter,
		hunk: DiffHunk,
		lang: string
	): Promise<string> {
		const code = hunk.lines.map((l) => l.content).join('\n');

		// Use decorations API for line-level coloring
		const decorations = hunk.lines.map((line, i) => ({
			start: { line: i, character: 0 },
			end: { line: i, character: line.content.length || 1 },
			properties: { class: `diff-${line.type}` }
		}));

		// Check if language is loaded, fall back to text
		let effectiveLang = lang;
		try {
			highlighter.getLoadedLanguages();
			if (!highlighter.getLoadedLanguages().includes(lang)) {
				effectiveLang = 'text';
			}
		} catch {
			effectiveLang = 'text';
		}

		try {
			return highlighter.codeToHtml(code, {
				lang: effectiveLang,
				theme: 'github-dark',
				decorations
			});
		} catch {
			// Fallback: render without decorations
			return highlighter.codeToHtml(code, {
				lang: effectiveLang,
				theme: 'github-dark'
			});
		}
	}

	// --- State ---

	let loading = $state(true);
	let error = $state<string | null>(null);
	let files = $state<DiffFile[]>([]);
	let highlightedHunks = $state<Map<string, string>>(new Map());

	// Fetch and highlight on taskId change
	$effect(() => {
		const id = taskId;
		loading = true;
		error = null;
		files = [];
		highlightedHunks = new Map();

		(async () => {
			try {
				const resp = await fetchDiff(id);
				if (!resp.diff || resp.diff.trim() === '') {
					files = [];
					loading = false;
					return;
				}

				const parsed = parseDiff(resp.diff);
				files = parsed;

				// Highlight all hunks
				const hl = await getHighlighter();
				const newMap = new Map<string, string>();

				for (let fi = 0; fi < parsed.length; fi++) {
					const file = parsed[fi];
					const lang = detectLang(file.path);
					for (let hi = 0; hi < file.hunks.length; hi++) {
						const html = await highlightHunk(hl, file.hunks[hi], lang);
						newMap.set(`${fi}-${hi}`, html);
					}
				}

				highlightedHunks = newMap;
			} catch (e) {
				error = e instanceof Error ? e.message : 'Unable to load diff';
			} finally {
				loading = false;
			}
		})();
	});
</script>

<div class="diff-view flex-1 overflow-auto" style="background-color: var(--color-bg);">
	{#if loading}
		<div
			class="flex items-center justify-center h-full gap-2"
			style="color: var(--color-dimmed);"
		>
			<span class="inline-block w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
			<span class="text-sm">Loading diff...</span>
		</div>
	{:else if error}
		<div
			class="flex flex-col items-center justify-center h-full gap-2"
			style="color: var(--color-dimmed);"
		>
			<span class="text-3xl opacity-40">!</span>
			<p class="text-sm">{error}</p>
		</div>
	{:else if files.length === 0}
		<div
			class="flex flex-col items-center justify-center h-full gap-2"
			style="color: var(--color-dimmed);"
		>
			<span class="text-3xl opacity-40">=</span>
			<p class="text-sm">No changes</p>
		</div>
	{:else}
		<div class="p-3 space-y-4">
			{#each files as file, fi}
				<div class="diff-file rounded-lg overflow-hidden" style="border: 1px solid var(--color-border);">
					<!-- File header -->
					<div
						class="px-3 py-2 text-xs font-mono font-semibold"
						style="background-color: rgba(255,255,255,0.05); color: var(--color-text); border-bottom: 1px solid var(--color-border);"
					>
						{file.path}
					</div>

					<!-- Hunks -->
					{#each file.hunks as hunk, hi}
						<div
							class="text-xs font-mono px-3 py-1"
							style="background-color: rgba(130,180,255,0.06); color: var(--color-dimmed);"
						>
							{hunk.header}
						</div>
						<div class="diff-hunk-content text-sm font-mono">
							{#if highlightedHunks.has(`${fi}-${hi}`)}
								{@html highlightedHunks.get(`${fi}-${hi}`)}
							{:else}
								<!-- Fallback plain rendering -->
								{#each hunk.lines as line}
									<div
										class="px-3 leading-relaxed {line.type === 'add'
											? 'diff-line-add'
											: line.type === 'remove'
												? 'diff-line-remove'
												: 'diff-line-context'}"
									>
										<span class="inline-block w-4 select-none opacity-50"
											>{line.type === 'add' ? '+' : line.type === 'remove' ? '-' : ' '}</span
										>{line.content}
									</div>
								{/each}
							{/if}
						</div>
					{/each}
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.diff-view :global(.shiki) {
		background-color: transparent !important;
		margin: 0;
		padding: 0;
	}

	.diff-view :global(.shiki code) {
		display: block;
	}

	.diff-view :global(.shiki .line) {
		padding: 0 0.75rem;
		display: block;
		line-height: 1.625;
	}

	.diff-view :global(.diff-add) {
		background-color: rgba(46, 160, 67, 0.15);
	}

	.diff-view :global(.diff-remove) {
		background-color: rgba(248, 81, 73, 0.15);
	}

	.diff-view :global(.diff-context) {
		opacity: 0.7;
	}

	.diff-line-add {
		background-color: rgba(46, 160, 67, 0.15);
	}

	.diff-line-remove {
		background-color: rgba(248, 81, 73, 0.15);
	}

	.diff-line-context {
		opacity: 0.7;
	}
</style>
