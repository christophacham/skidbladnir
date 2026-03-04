import { describe, it, expect } from 'vitest';
import { classifyBlock, WebSocketStore, MAX_BLOCKS } from '$lib/stores/websocket.svelte';
import type { ServerMessage } from '$lib/types';

describe('classifyBlock', () => {
	it('returns error for "Error: something failed"', () => {
		expect(classifyBlock('Error: something failed')).toBe('error');
	});

	it('returns error for lines containing "traceback"', () => {
		expect(classifyBlock('  File "test.py", Traceback (most recent call last)')).toBe('error');
	});

	it('returns error for lines containing "exception"', () => {
		expect(classifyBlock('RuntimeException occurred during processing')).toBe('error');
	});

	it('returns tool_call for "Read(/path/to/file)"', () => {
		expect(classifyBlock('Read(/path/to/file)')).toBe('tool_call');
	});

	it('returns tool_call for "Bash(command)"', () => {
		expect(classifyBlock('Bash(ls -la)')).toBe('tool_call');
	});

	it('returns tool_call for lines starting with "$ "', () => {
		expect(classifyBlock('  $ npm install')).toBe('tool_call');
	});

	it('returns text for normal output lines', () => {
		expect(classifyBlock('Hello, world!')).toBe('text');
		expect(classifyBlock('Building project...')).toBe('text');
		expect(classifyBlock('')).toBe('text');
	});
});

describe('WebSocketStore', () => {
	function createStore(): WebSocketStore {
		return new WebSocketStore();
	}

	describe('handleMessage', () => {
		it('decodes base64 output and appends OutputBlock', () => {
			const store = createStore();
			const text = 'Hello, world!';
			const data = btoa(text);
			const msg: ServerMessage = { type: 'output', data, offset: 0 };

			store.handleMessage(msg);

			expect(store.outputBlocks.length).toBeGreaterThan(0);
			expect(store.outputBlocks[0].text).toBe(text);
			expect(store.outputBlocks[0].type).toBe('text');
		});

		it('updates phaseStatuses on state message', () => {
			const store = createStore();
			store.activeSessionId = 'session-1';
			const msg: ServerMessage = { type: 'state', session_state: 'running' };

			store.handleMessage(msg);

			expect(store.phaseStatuses.get('session-1')).toBe('working');
		});

		it('sets connectionStatus to connected on connected message', () => {
			const store = createStore();
			const msg: ServerMessage = { type: 'connected', session_id: 'sess-1', total_bytes: 1024 };

			store.handleMessage(msg);

			expect(store.connectionStatus).toBe('connected');
			expect(store.totalBytes).toBe(1024);
		});

		it('creates error OutputBlock on error message', () => {
			const store = createStore();
			const msg: ServerMessage = { type: 'error', message: 'Connection lost' };

			store.handleMessage(msg);

			expect(store.outputBlocks.length).toBe(1);
			expect(store.outputBlocks[0].type).toBe('error');
			expect(store.outputBlocks[0].text).toBe('Connection lost');
		});

		it('caps output blocks at MAX_BLOCKS, dropping oldest', () => {
			const store = createStore();

			// Fill with MAX_BLOCKS entries
			for (let i = 0; i < MAX_BLOCKS + 50; i++) {
				const data = btoa(`line ${i}`);
				store.handleMessage({ type: 'output', data, offset: i });
			}

			expect(store.outputBlocks.length).toBeLessThanOrEqual(MAX_BLOCKS);
		});

		it('maps exited state to exited PhaseStatus', () => {
			const store = createStore();
			store.activeSessionId = 'session-2';
			store.handleMessage({ type: 'state', session_state: 'exited', exit_code: 0 });

			expect(store.phaseStatuses.get('session-2')).toBe('exited');
		});

		it('maps spawning state to working PhaseStatus', () => {
			const store = createStore();
			store.activeSessionId = 'session-3';
			store.handleMessage({ type: 'state', session_state: 'spawning' });

			expect(store.phaseStatuses.get('session-3')).toBe('working');
		});
	});
});
