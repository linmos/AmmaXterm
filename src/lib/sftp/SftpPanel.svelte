<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import type { FileEntry } from './types';

	interface Props {
		sessionId: string;
	}
	let { sessionId }: Props = $props();

	let path = $state('.');
	let entries = $state<FileEntry[]>([]);
	let errorMsg = $state<string | undefined>(undefined);
	let loading = $state(false);

	async function list() {
		loading = true;
		errorMsg = undefined;
		try {
			entries = await invoke<FileEntry[]>('sftp_list', { id: sessionId, path });
		} catch (err) {
			const e = err as { message?: string } | undefined;
			errorMsg = e?.message ?? String(err);
		} finally {
			loading = false;
		}
	}

	function open(entry: FileEntry) {
		if (!entry.is_dir) return;
		path = path === '/' ? '/' + entry.name : path.replace(/\/$/, '') + '/' + entry.name;
		list();
	}

	function up() {
		const trimmed = path.replace(/\/$/, '');
		const idx = trimmed.lastIndexOf('/');
		path = idx > 0 ? trimmed.slice(0, idx) : idx === 0 ? '/' : '.';
		list();
	}

	function fmtSize(n: number): string {
		if (n < 1024) return `${n} B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
		if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
		return `${(n / 1024 / 1024 / 1024).toFixed(1)} GB`;
	}

	onMount(list);
</script>

<div class="sftp">
	<div class="bar">
		<button onclick={up} title="Up one level">↑</button>
		<input
			aria-label="Remote path"
			bind:value={path}
			onkeydown={(e) => e.key === 'Enter' && list()}
		/>
		<button onclick={list} disabled={loading} title="Refresh">{loading ? '…' : '⟳'}</button>
	</div>

	{#if errorMsg}
		<p class="err">{errorMsg}</p>
	{/if}

	<ul class="list">
		{#each entries as entry (entry.name)}
			<li>
				<button class="row" class:dir={entry.is_dir} onclick={() => open(entry)}>
					<span class="name">{entry.is_dir ? '📁' : '📄'} {entry.name}</span>
					{#if !entry.is_dir}<span class="size">{fmtSize(entry.size)}</span>{/if}
				</button>
			</li>
		{/each}
		{#if !entries.length && !loading && !errorMsg}
			<li class="empty">empty</li>
		{/if}
	</ul>
</div>

<style>
	.sftp {
		display: flex;
		flex-direction: column;
		height: 100%;
		color: #ddd;
		font: 13px system-ui, sans-serif;
		background: #1b1b1b;
	}
	.bar {
		display: flex;
		gap: 4px;
		padding: 6px;
		border-bottom: 1px solid #333;
	}
	.bar input {
		flex: 1;
		min-width: 0;
		padding: 4px 6px;
		border: 1px solid #3c3c3c;
		border-radius: 4px;
		background: #1e1e1e;
		color: #eee;
		font: inherit;
	}
	.bar button {
		padding: 4px 8px;
		border: 1px solid #444;
		border-radius: 4px;
		background: #2a2a2a;
		color: #ddd;
		cursor: pointer;
	}
	.list {
		flex: 1;
		min-height: 0;
		overflow: auto;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.row {
		display: flex;
		justify-content: space-between;
		gap: 8px;
		width: 100%;
		padding: 4px 8px;
		border: none;
		background: transparent;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: default;
	}
	.row.dir {
		cursor: pointer;
		color: #6cb6ff;
	}
	.row:hover {
		background: #2a2a2a;
	}
	.row .size {
		opacity: 0.6;
		font-variant-numeric: tabular-nums;
	}
	.err {
		margin: 6px 8px;
		color: #f48771;
		font-size: 12px;
		word-break: break-word;
	}
	.empty {
		padding: 8px;
		opacity: 0.5;
	}
</style>
