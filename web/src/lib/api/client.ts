const BASE_URL = '/api/v1';

export class ApiError extends Error {
	status: number;

	constructor(message: string, status: number) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
	}
}

export async function api<T>(path: string, options?: RequestInit): Promise<T> {
	const url = `${BASE_URL}${path}`;
	const response = await fetch(url, {
		headers: {
			'Content-Type': 'application/json',
			...options?.headers
		},
		...options
	});

	if (response.status === 204) {
		return undefined as T;
	}

	if (!response.ok) {
		let message = response.statusText;
		try {
			const body = await response.json();
			if (body.error) {
				message = body.error;
			}
		} catch {
			// Use statusText if body parsing fails
		}
		throw new ApiError(message, response.status);
	}

	return response.json() as Promise<T>;
}
