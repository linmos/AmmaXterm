<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Terminal, type ITheme } from '@xterm/xterm';
	import { FitAddon } from '@xterm/addon-fit';
	import { WebglAddon } from '@xterm/addon-webgl';
	import { SearchAddon } from '@xterm/addon-search';
	import { Unicode11Addon } from '@xterm/addon-unicode11';
	import '@xterm/xterm/css/xterm.css';
	import { i18n } from '$lib/i18n.svelte';
	import type { TerminalApi, TerminalSize } from './types';

	interface Props {
		/** Called for each chunk of user input (keystrokes, paste). */
		onData?: (data: string) => void;
		/** Called when the terminal is resized (send window-change to remote). */
		onResize?: (size: TerminalSize) => void;
		/** Called once the terminal is ready, handing back an imperative API. */
		onReady?: (api: TerminalApi) => void;
		/** Called with the shell's reported cwd (OSC 7 / OSC 9;9), for follow-cd. */
		onCwd?: (path: string) => void;
		fontSize?: number;
		fontFamily?: string;
		scrollback?: number;
		theme?: ITheme;
	}

	let {
		onData,
		onResize,
		onReady,
		onCwd,
		fontSize = 14,
		fontFamily = 'Consolas, "Cascadia Mono", "DejaVu Sans Mono", monospace',
		scrollback = 5000,
		theme = { background: '#1e1e1e', foreground: '#d4d4d4' }
	}: Props = $props();

	let container: HTMLDivElement;
	let term: Terminal | undefined;
	let fitAddon: FitAddon | undefined;
	let searchAddon: SearchAddon | undefined;
	let resizeObserver: ResizeObserver | undefined;

	// In-terminal search (TM-10).
	let showSearch = $state(false);
	let query = $state('');
	let caseSensitive = $state(false);
	let searchInput = $state<HTMLInputElement | undefined>();

	function fit() {
		if (!fitAddon || !term) return;
		try {
			fitAddon.fit();
		} catch {
			// Container not measurable yet; ignore until next resize tick.
		}
	}

	function doFind(forward: boolean) {
		if (!query) return;
		const opts = { caseSensitive };
		if (forward) searchAddon?.findNext(query, opts);
		else searchAddon?.findPrevious(query, opts);
	}

	function openSearch() {
		showSearch = true;
		queueMicrotask(() => searchInput?.select());
	}

	function closeSearch() {
		showSearch = false;
		searchAddon?.clearDecorations();
		term?.focus();
	}

	function searchKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			doFind(!e.shiftKey);
		} else if (e.key === 'Escape') {
			e.preventDefault();
			closeSearch();
		}
	}

	// Apply appearance settings live when they change (TM-11, ST-1/2).
	$effect(() => {
		const t = term;
		const opts = { fontSize, fontFamily, scrollback, theme };
		if (!t) return;
		t.options.fontSize = opts.fontSize;
		t.options.fontFamily = opts.fontFamily;
		t.options.scrollback = opts.scrollback;
		t.options.theme = opts.theme;
		fit();
	});

	onMount(() => {
		term = new Terminal({
			allowProposedApi: true, // required by Unicode11Addon
			cursorBlink: true,
			fontSize,
			fontFamily,
			scrollback,
			theme
		});

		fitAddon = new FitAddon();
		searchAddon = new SearchAddon();
		term.loadAddon(fitAddon);
		term.loadAddon(searchAddon);

		const unicode11 = new Unicode11Addon();
		term.loadAddon(unicode11);
		term.unicode.activeVersion = '11';

		term.open(container);

		// WebGL renderer for high-throughput output (PRD §6.2); fall back gracefully.
		try {
			const webgl = new WebglAddon();
			webgl.onContextLoss(() => webgl.dispose());
			term.loadAddon(webgl);
		} catch (e) {
			console.warn('WebGL addon unavailable, falling back to canvas/DOM renderer', e);
		}

		fit();

		term.onData((data) => onData?.(data));
		term.onResize((size) => onResize?.(size));

		// Follow-cd (FT-6): capture the shell's reported working directory.
		// OSC 7 carries a file:// URI; ConEmu's OSC 9;9 carries a raw path.
		term.parser.registerOscHandler(7, (data) => {
			try {
				const url = new URL(data);
				if (url.pathname) onCwd?.(decodeURIComponent(url.pathname));
			} catch {
				// Not a URI we understand; ignore.
			}
			return true;
		});
		term.parser.registerOscHandler(9, (data) => {
			if (data.startsWith('9;')) {
				onCwd?.(data.slice(2));
				return true;
			}
			return false;
		});

		// Copy / paste (TM-4) + search toggle (TM-10): Ctrl+Shift+C/V/F.
		term.attachCustomKeyEventHandler((e) => {
			if (e.type === 'keydown' && e.ctrlKey && e.shiftKey && term) {
				const k = e.key.toLowerCase();
				if (k === 'c' && term.hasSelection()) {
					void navigator.clipboard.writeText(term.getSelection());
					return false;
				}
				if (k === 'v') {
					void navigator.clipboard.readText().then((t) => t && term?.paste(t));
					return false;
				}
				if (k === 'f') {
					openSearch();
					return false;
				}
			}
			return true;
		});
		container.addEventListener('contextmenu', (e) => {
			e.preventDefault();
			if (term?.hasSelection()) {
				void navigator.clipboard.writeText(term.getSelection());
				term.clearSelection();
			} else {
				void navigator.clipboard.readText().then((t) => t && term?.paste(t));
			}
		});

		resizeObserver = new ResizeObserver(() => fit());
		resizeObserver.observe(container);

		const api: TerminalApi = {
			write: (data) => term?.write(data),
			clear: () => term?.clear(),
			focus: () => term?.focus(),
			fit,
			dispose: () => term?.dispose(),
			findNext: (q, opts) => {
				searchAddon?.findNext(q, opts);
			},
			findPrevious: (q, opts) => {
				searchAddon?.findPrevious(q, opts);
			}
		};
		onReady?.(api);
		term.focus();
	});

	onDestroy(() => {
		resizeObserver?.disconnect();
		term?.dispose();
	});
</script>

<div class="terminal-wrap">
	{#if showSearch}
		<div class="search">
			<input
				bind:this={searchInput}
				bind:value={query}
				oninput={() => doFind(true)}
				onkeydown={searchKeydown}
				placeholder={i18n.t('search.placeholder')}
			/>
			<button title={i18n.t('search.prev')} onclick={() => doFind(false)}>↑</button>
			<button title={i18n.t('search.next')} onclick={() => doFind(true)}>↓</button>
			<button
				class="toggle"
				class:on={caseSensitive}
				title={i18n.t('search.matchCase')}
				onclick={() => {
					caseSensitive = !caseSensitive;
					doFind(true);
				}}>Aa</button
			>
			<button title={i18n.t('search.close')} onclick={closeSearch}>×</button>
		</div>
	{/if}
	<div class="terminal-host" bind:this={container}></div>
</div>

<style>
	.terminal-wrap {
		position: relative;
		width: 100%;
		height: 100%;
		min-height: 0;
	}
	.terminal-host {
		width: 100%;
		height: 100%;
		min-height: 0;
	}
	.search {
		position: absolute;
		top: 6px;
		right: 16px;
		z-index: 6;
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 4px;
		background: #2a2a2a;
		border: 1px solid #454545;
		border-radius: 6px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
	}
	.search input {
		width: 160px;
		padding: 4px 6px;
		border: 1px solid #3c3c3c;
		border-radius: 4px;
		background: #1e1e1e;
		color: #eee;
		font: 12px system-ui, sans-serif;
	}
	.search button {
		padding: 4px 7px;
		border: none;
		border-radius: 4px;
		background: transparent;
		color: #ddd;
		font: 12px system-ui, sans-serif;
		cursor: pointer;
	}
	.search button:hover {
		background: #3c3c3c;
	}
	.search button.toggle.on {
		background: #0e639c;
		color: #fff;
	}
</style>
