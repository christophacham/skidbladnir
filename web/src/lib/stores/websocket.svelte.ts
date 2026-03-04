import type {
	ConnectionStatus,
	PhaseStatus,
	OutputBlock,
	OutputBlockType,
	ServerMessage,
	ClientMessage
} from '$lib/types';

export const MAX_BLOCKS = 2000;

const ERROR_PATTERNS = [
	/^(Error|error|ERROR|FATAL|panic|FAIL)[:!]/,
	/(traceback|exception|failed|denied)/i
];

const TOOL_CALL_PATTERNS = [
	/^(Read|Write|Edit|Bash|Glob|Grep|WebSearch|WebFetch)\(/,
	/^\s*\$ /,
	/^(Creating|Updating|Deleting|Reading) /
];

export function classifyBlock(text: string): OutputBlockType {
	for (const pattern of ERROR_PATTERNS) {
		if (pattern.test(text)) return 'error';
	}
	for (const pattern of TOOL_CALL_PATTERNS) {
		if (pattern.test(text)) return 'tool_call';
	}
	return 'text';
}

function derivePhaseStatus(sessionState: string): PhaseStatus {
	switch (sessionState) {
		case 'running':
		case 'spawning':
			return 'working';
		case 'exited':
			return 'exited';
		case 'idle':
			return 'idle';
		case 'ready':
			return 'ready';
		default:
			return 'working';
	}
}

export class WebSocketStore {
	socket: WebSocket | null = $state(null);
	connectionStatus: ConnectionStatus = $state('disconnected');
	activeSessionId: string | null = $state(null);
	outputBlocks: OutputBlock[] = $state([]);
	totalBytes: number = $state(0);
	phaseStatuses: Map<string, PhaseStatus> = $state(new Map());
	hasError: boolean = $state(false);

	private cursor: number = 0;
	private blockIdCounter: number = 0;
	private retryCount: number = 0;
	private retryTimer: ReturnType<typeof setTimeout> | null = null;
	private decoder: TextDecoder | null = null;

	connect(sessionId: string): void {
		this.disconnect();
		this.decoder = new TextDecoder('utf-8', { fatal: false });
		this.activeSessionId = sessionId;
		this.connectionStatus = 'reconnecting';

		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		const wsUrl = `${protocol}//${window.location.host}/api/v1/sessions/${sessionId}/ws`;

		const ws = new WebSocket(wsUrl);

		ws.onopen = () => {
			this.retryCount = 0;
			// connectionStatus will be set to 'connected' by the 'connected' server message
		};

		ws.onmessage = (event: MessageEvent) => {
			try {
				const msg: ServerMessage = JSON.parse(event.data);
				this.handleMessage(msg);
			} catch {
				// Ignore malformed messages
			}
		};

		ws.onclose = () => {
			if (this.activeSessionId === sessionId) {
				this.scheduleReconnect(sessionId);
			}
		};

		ws.onerror = () => {
			this.hasError = true;
		};

		this.socket = ws;
	}

	disconnect(): void {
		if (this.retryTimer !== null) {
			clearTimeout(this.retryTimer);
			this.retryTimer = null;
		}
		if (this.socket) {
			this.socket.onclose = null;
			this.socket.onerror = null;
			this.socket.onmessage = null;
			if (
				this.socket.readyState === WebSocket.OPEN ||
				this.socket.readyState === WebSocket.CONNECTING
			) {
				this.socket.close();
			}
		}
		this.socket = null;
		this.decoder = null;
		this.connectionStatus = 'disconnected';
		this.activeSessionId = null;
	}

	send(message: ClientMessage): void {
		if (this.socket && this.socket.readyState === WebSocket.OPEN) {
			this.socket.send(JSON.stringify(message));
		}
	}

	handleMessage(msg: ServerMessage): void {
		switch (msg.type) {
			case 'output': {
				const bytes = Uint8Array.from(atob(msg.data), (c) => c.charCodeAt(0));
				const decoder = this.decoder ?? new TextDecoder('utf-8', { fatal: false });
				const text = decoder.decode(bytes, { stream: true });
				const lines = text.split('\n');

				for (const line of lines) {
					if (line === '' && lines.length > 1 && lines[lines.length - 1] === '') {
						// Skip trailing empty line from split
						continue;
					}
					const block: OutputBlock = {
						id: this.blockIdCounter++,
						text: line,
						type: classifyBlock(line),
						timestamp: Date.now()
					};
					this.outputBlocks.push(block);
				}

				this.cursor = msg.offset + msg.data.length;

				// Cap at MAX_BLOCKS
				if (this.outputBlocks.length > MAX_BLOCKS) {
					this.outputBlocks = this.outputBlocks.slice(this.outputBlocks.length - MAX_BLOCKS);
				}
				break;
			}
			case 'state': {
				if (this.activeSessionId) {
					const next = new Map(this.phaseStatuses);
					next.set(this.activeSessionId, derivePhaseStatus(msg.session_state));
					this.phaseStatuses = next;
				}
				break;
			}
			case 'connected': {
				this.connectionStatus = 'connected';
				this.totalBytes = msg.total_bytes;
				break;
			}
			case 'error': {
				const block: OutputBlock = {
					id: this.blockIdCounter++,
					text: msg.message,
					type: 'error',
					timestamp: Date.now()
				};
				this.outputBlocks.push(block);
				this.hasError = true;
				break;
			}
			case 'metrics': {
				// Store for future use - no-op for now
				break;
			}
		}
	}

	clearOutput(): void {
		this.outputBlocks = [];
		this.cursor = 0;
		this.blockIdCounter = 0;
	}

	private scheduleReconnect(sessionId: string): void {
		const delay = Math.min(1000 * Math.pow(2, this.retryCount), 30000);
		this.retryCount++;
		this.connectionStatus = 'reconnecting';
		this.retryTimer = setTimeout(() => {
			this.retryTimer = null;
			this.connect(sessionId);
		}, delay);
	}
}

export const wsStore = new WebSocketStore();
