import { describe, it, expect } from 'vitest';

describe('TaskStore', () => {
	it.todo('groups tasks by status in byStatus');
	it.todo('filters tasks by searchQuery case-insensitively');
	it.todo('computes matchingIds as set of filtered task IDs');
	it.todo('handles empty searchQuery as match-all');
	it.todo('handles null description in search filter');
});
