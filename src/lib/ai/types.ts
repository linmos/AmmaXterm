// Shared types for the AI assistant (multi-provider, BYO key).

/** A single chat turn, mirroring the Rust `ChatMessage`. */
export interface ChatMessage {
	role: 'system' | 'user' | 'assistant';
	content: string;
}

/** Supported provider ids. */
export type AiProvider = 'claude' | 'openai' | 'ollama';
