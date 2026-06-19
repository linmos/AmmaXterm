<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Terminal } from '@xterm/xterm';
	import { FitAddon } from '@xterm/addon-fit';
	import { WebglAddon } from '@xterm/addon-webgl';
	import { SearchAddon } from '@xterm/addon-search';
	import { Unicode11Addon } from '@xterm/addon-unicode11';
	import '@xterm/xterm/css/xterm.css';
	import type { TerminalApi, TerminalSize } from './types';

	interface Props {
		/** Called for each chunk of user input (keystrokes, paste). */
		onData?: (data: string) => void;
		/** Called when the terminal is resized (send window-change to remote). */
		onResize?: (size: TerminalSize) => void;
		/** Called once the terminal is ready, handing back an imperative API. */
		onReady?: (api: TerminalApi) => void;
		fontSize?: number;
		fontFamily?: string;
		scrollback?: number;
	}

	let {
		onData,
		onResize,
		onReady,
		fontSize = 14,
		fontFamily = 'Consolas, "Cascadia Mono", "DejaVu Sans Mono", monospace',
		scrollback = 5000
	}: Props = $props();

	let container: HTMLDivElement;
	let term: Terminal | undefined;
	let fitAddon: FitAddon | undefined;
	let searchAddon: SearchAddon | undefined;
	let resizeObserver: ResizeObserver | undefined;

	function fit() {
		if (!fitAddon || !term) return;
		try {
			fitAddon.fit();
		} catch {
			// Container not measurable yet; ignore until next resize tick.
		}
	}

	onMount(() => {
		term = new Terminal({
			allowProposedApi: true, // required by Unicode11Addon
			cursorBlink: true,
			fontSize,
			fontFamily,
			scrollback,
			theme: {
				background: '#1e1e1e',
				foreground: '#d4d4d4'
			}
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

<div class="terminal-host" bind:this={container}></div>

<style>
	.terminal-host {
		width: 100%;
		height: 100%;
		min-height: 0;
	}
</style>
