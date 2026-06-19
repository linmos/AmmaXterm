// Global application settings (TM-11, ST-1, ST-2): terminal appearance and
// connection defaults, persisted to settings.json via the backend. Theme names
// are resolved to concrete xterm colours here on the frontend.

import { invoke } from '@tauri-apps/api/core';
import type { ITheme } from '@xterm/xterm';

/** Mirrors the Rust `Settings` struct (camelCase). */
export interface Settings {
	schemaVersion: number;
	theme: string;
	fontFamily: string;
	fontSize: number;
	scrollback: number;
	keepaliveSecs: number;
	autoReconnect: boolean;
}

// "system" follows the OS colour scheme; "dark"/"light" pin a concrete palette.
export const THEME_NAMES = ['system', 'dark', 'light'] as const;

const DARK: ITheme = {
	background: '#1e1e1e',
	foreground: '#d4d4d4',
	cursor: '#d4d4d4',
	cursorAccent: '#1e1e1e',
	selectionBackground: '#264f78'
};

const LIGHT: ITheme = {
	background: '#fafafa',
	foreground: '#2d2d2d',
	cursor: '#2d2d2d',
	cursorAccent: '#fafafa',
	selectionBackground: '#add6ff'
};

/** True when the OS reports a dark colour scheme (defaults to dark off-screen). */
function prefersDark(): boolean {
	return typeof window !== 'undefined' && typeof window.matchMedia === 'function'
		? window.matchMedia('(prefers-color-scheme: dark)').matches
		: true;
}

/** Collapse a theme name to the concrete palette it resolves to right now. */
export function resolveTheme(name: string): 'dark' | 'light' {
	if (name === 'light' || name === 'dark') return name;
	return prefersDark() ? 'dark' : 'light'; // "system" or unknown
}

/** Resolve a theme name to an xterm theme (unknown/system → OS preference). */
export function xtermTheme(name: string): ITheme {
	return resolveTheme(name) === 'light' ? LIGHT : DARK;
}

const DEFAULTS: Settings = {
	schemaVersion: 1,
	theme: 'system',
	fontFamily: 'Consolas, "Cascadia Mono", "DejaVu Sans Mono", monospace',
	fontSize: 14,
	scrollback: 5000,
	keepaliveSecs: 30,
	autoReconnect: false
};

class AppSettings {
	s = $state<Settings>({ ...DEFAULTS });
	// Tracks the OS colour scheme so `theme: 'system'` stays reactive when the
	// user flips their desktop theme while the app is open.
	#systemDark = $state(true);

	/** Concrete 'dark' | 'light' after resolving 'system' against the OS. */
	get effectiveTheme(): 'dark' | 'light' {
		if (this.s.theme === 'light' || this.s.theme === 'dark') return this.s.theme;
		return this.#systemDark ? 'dark' : 'light';
	}

	/** The active xterm theme for the current `theme` name. */
	get theme(): ITheme {
		return this.effectiveTheme === 'light' ? LIGHT : DARK;
	}

	/** Watch the OS colour scheme and reflect the resolved theme onto <html>. */
	init() {
		if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
			const mq = window.matchMedia('(prefers-color-scheme: dark)');
			this.#systemDark = mq.matches;
			mq.addEventListener('change', (e) => {
				this.#systemDark = e.matches;
				this.#applyUi();
			});
		}
		this.#applyUi();
	}

	/** Drive the shell palette (vscode.css light overrides) via a root attribute. */
	#applyUi() {
		if (typeof document !== 'undefined') {
			document.documentElement.dataset.theme = this.effectiveTheme;
		}
	}

	async load() {
		try {
			this.s = await invoke<Settings>('settings_get');
		} catch {
			// Keep defaults if the backend isn't ready / file is unreadable.
		}
		this.#applyUi();
	}

	/** Persist new settings and adopt the value the backend echoes back. */
	async save(next: Settings) {
		this.s = await invoke<Settings>('settings_set', { value: next });
		this.#applyUi();
	}
}

export const settings = new AppSettings();
