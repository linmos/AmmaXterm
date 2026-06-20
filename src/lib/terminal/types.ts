import type { ISearchOptions } from '@xterm/addon-search';

/**
 * Imperative handle exposed by the Terminal component so a parent can push
 * bytes into the terminal (e.g. from an SSH stream) and drive search/focus.
 */
export interface TerminalApi {
	/** Write data to the terminal (accepts UTF-8 strings or raw bytes). */
	write(data: string | Uint8Array): void;
	/** Clear the viewport. */
	clear(): void;
	/** Focus the terminal so it receives keyboard input. */
	focus(): void;
	/** Re-fit the terminal to its container. */
	fit(): void;
	/** Dispose the underlying xterm instance. */
	dispose(): void;
	/** Search forward for a query (TM-10). */
	findNext(query: string, options?: ISearchOptions): void;
	/** Search backward for a query (TM-10). */
	findPrevious(query: string, options?: ISearchOptions): void;
	/** Current text selection, or '' when nothing is selected (AI-3). */
	getSelection(): string;
	/** Last `lines` rows of the buffer as text, for AI context (AI-4). */
	getRecentText(lines: number): string;
	/** Insert text at the shell prompt (paste; no auto-newline) (AI-2 / AI-N2). */
	insert(text: string): void;
}

/** Terminal dimensions in character cells. */
export interface TerminalSize {
	cols: number;
	rows: number;
}
