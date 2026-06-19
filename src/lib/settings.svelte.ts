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
}

export const THEME_NAMES = ['dark', 'light'] as const;

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

/** Resolve a theme name to an xterm theme (unknown → dark). */
export function xtermTheme(name: string): ITheme {
	return name === 'light' ? LIGHT : DARK;
}

const DEFAULTS: Settings = {
	schemaVersion: 1,
	theme: 'dark',
	fontFamily: 'Consolas, "Cascadia Mono", "DejaVu Sans Mono", monospace',
	fontSize: 14,
	scrollback: 5000,
	keepaliveSecs: 30
};

class AppSettings {
	s = $state<Settings>({ ...DEFAULTS });

	/** The active xterm theme for the current `theme` name. */
	get theme(): ITheme {
		return xtermTheme(this.s.theme);
	}

	async load() {
		try {
			this.s = await invoke<Settings>('settings_get');
		} catch {
			// Keep defaults if the backend isn't ready / file is unreadable.
		}
	}

	/** Persist new settings and adopt the value the backend echoes back. */
	async save(next: Settings) {
		this.s = await invoke<Settings>('settings_set', { value: next });
	}
}

export const settings = new AppSettings();
