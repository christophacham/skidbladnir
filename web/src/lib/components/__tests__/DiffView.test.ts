import { describe, it, expect } from 'vitest';

describe('DiffView', () => {
	it.todo('renders loading state while fetching diff');
	it.todo('renders "No changes" when diff is empty');
	it.todo('renders error message when fetchDiff fails');
	it.todo('parses unified diff into file sections');
	it.todo('classifies lines as add, remove, or context by prefix');
	it.todo('detects language from file extension (rs -> rust, ts -> typescript)');
	it.todo('applies green background to added lines');
	it.todo('applies red background to removed lines');
});

describe('DiffView diff parser', () => {
	it.todo('splits multi-file diff on "diff --git" markers');
	it.todo('extracts file path from "+++ b/path" line');
	it.todo('splits hunks on @@ markers');
	it.todo('handles new file diffs (from /dev/null)');
});
