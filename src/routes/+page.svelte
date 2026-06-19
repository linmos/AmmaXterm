<script lang="ts">
	import { invoke, Channel } from '@tauri-apps/api/core';
	import Terminal from '$lib/terminal/Terminal.svelte';
	import SftpPanel from '$lib/sftp/SftpPanel.svelte';
	import type { TerminalApi, TerminalSize } from '$lib/terminal/types';

	let api = $state<TerminalApi | undefined>(undefined);
	let sessionId = $state<string | undefined>(undefined);
	let connecting = $state(false);
	let errorMsg = $state<string | undefined>(undefined);
	let showFiles = $state(false);

	// Connect form (M0: password auth).
	let host = $state('');
	let port = $state(22);
	let username = $state('');
	let password = $state('');

	// Latest known terminal size, kept up to date by the Terminal's onResize.
	let size: TerminalSize = { cols: 80, rows: 24 };

	function b64ToBytes(b64: string): Uint8Array {
		const bin = atob(b64);
		const bytes = new Uint8Array(bin.length);
		for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
		return bytes;
	}

	function handleReady(a: TerminalApi) {
		api = a;
	}

	function handleData(data: string) {
		if (sessionId) invoke('ssh_send_input', { id: sessionId, data }).catch(() => {});
	}

	function handleResize(s: TerminalSize) {
		size = s;
		if (sessionId) {
			invoke('ssh_resize', { id: sessionId, cols: s.cols, rows: s.rows }).catch(() => {});
		}
	}

	async function connect(event: Event) {
		event.preventDefault();
		errorMsg = undefined;
		connecting = true;
		try {
			const onOutput = new Channel<string>();
			onOutput.onmessage = (b64) => api?.write(b64ToBytes(b64));
			sessionId = await invoke<string>('ssh_connect', {
				options: {
					host,
					port: Number(port),
					username,
					password,
					cols: size.cols,
					rows: size.rows
				},
				onOutput
			});
			password = ''; // don't keep the password around in component state
			api?.focus();
		} catch (err) {
			const e = err as { message?: string } | undefined;
			errorMsg = e?.message ?? String(err);
		} finally {
			connecting = false;
		}
	}

	async function disconnect() {
		if (!sessionId) return;
		await invoke('ssh_disconnect', { id: sessionId }).catch(() => {});
		sessionId = undefined;
		showFiles = false;
	}
</script>

<div class="app">
	{#if sessionId}
		<div class="toolbar">
			<span class="dot">●</span>
			<span class="who">{username}@{host}:{port}</span>
			<button class="ghost" class:active={showFiles} onclick={() => (showFiles = !showFiles)}>
				Files
			</button>
			<button class="ghost" onclick={disconnect}>Disconnect</button>
		</div>
	{/if}

	<div class="body">
		<div class="terminal">
			<Terminal onReady={handleReady} onData={handleData} onResize={handleResize} />
		</div>
		{#if sessionId && showFiles}
			<div class="sftp-wrap">
				{#key sessionId}
					<SftpPanel {sessionId} />
				{/key}
			</div>
		{/if}
	</div>

	{#if !sessionId}
		<div class="overlay">
			<form class="connect" onsubmit={connect}>
				<h1>AmmaXterm</h1>
				<p class="hint">M0 spike — connect over SSH (password auth)</p>
				<input placeholder="Host" bind:value={host} required />
				<input type="number" placeholder="Port" min="1" max="65535" bind:value={port} />
				<input placeholder="Username" bind:value={username} required />
				<input type="password" placeholder="Password" bind:value={password} />
				<button type="submit" disabled={connecting}>
					{connecting ? 'Connecting…' : 'Connect'}
				</button>
				{#if errorMsg}<p class="error">{errorMsg}</p>{/if}
			</form>
		</div>
	{/if}
</div>

<style>
	:global(html, body) {
		margin: 0;
		height: 100%;
	}
	.app {
		position: fixed;
		inset: 0;
		display: flex;
		flex-direction: column;
		background: #1e1e1e;
	}
	.toolbar {
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 6px 10px;
		color: #ddd;
		background: #252526;
		font: 13px system-ui, sans-serif;
	}
	.toolbar .dot {
		color: #3fb950;
	}
	.toolbar .who {
		opacity: 0.85;
		margin-right: auto;
	}
	.body {
		flex: 1 1 auto;
		min-height: 0;
		display: flex;
	}
	.terminal {
		flex: 1 1 auto;
		min-width: 0;
		padding: 6px;
		box-sizing: border-box;
	}
	.sftp-wrap {
		flex: 0 0 320px;
		min-height: 0;
		border-left: 1px solid #333;
	}
	.overlay {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.6);
	}
	.connect {
		display: flex;
		flex-direction: column;
		gap: 10px;
		width: 320px;
		padding: 24px;
		background: #252526;
		border: 1px solid #333;
		border-radius: 10px;
		color: #eee;
		font: 14px system-ui, sans-serif;
	}
	.connect h1 {
		margin: 0;
		font-size: 20px;
	}
	.connect .hint {
		margin: 0 0 4px;
		font-size: 12px;
		opacity: 0.7;
	}
	.connect input {
		padding: 8px 10px;
		border: 1px solid #3c3c3c;
		border-radius: 6px;
		background: #1e1e1e;
		color: #eee;
		font: inherit;
	}
	.connect button[type='submit'] {
		padding: 9px 10px;
		border: none;
		border-radius: 6px;
		background: #0e639c;
		color: #fff;
		font: inherit;
		cursor: pointer;
	}
	.connect button[type='submit']:disabled {
		opacity: 0.6;
		cursor: default;
	}
	.ghost {
		padding: 4px 10px;
		border: 1px solid #555;
		border-radius: 6px;
		background: transparent;
		color: #ddd;
		cursor: pointer;
	}
	.ghost.active {
		background: #0e639c;
		border-color: #0e639c;
		color: #fff;
	}
	.error {
		margin: 0;
		color: #f48771;
		font-size: 13px;
	}
</style>
