<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';

	interface LocalEntry {
		name: string;
		is_dir: boolean;
		size: number;
	}
	interface Props {
		sessionId: string;
		/** Current remote directory — uploads land here. */
		remotePath: string;
		/** Reports the resolved local path so the parent can target downloads. */
		onPath: (p: string) => void;
	}
	let { sessionId, remotePath, onPath }: Props = $props();

	let lpath = $state('');
	let entries = $state<LocalEntry[]>([]);
	let errorMsg = $state<string | undefined>(undefined);

	const sep = $derived(lpath.includes('\\') ? '\\' : '/');
	function joinLocal(name: string): string {
		return lpath.replace(/[\\/]+$/, '') + sep + name;
	}
	function parent(p: string): string {
		const trimmed = p.replace(/[\\/]+$/, '');
		const idx = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'));
		return idx > 0 ? trimmed.slice(0, idx) : trimmed.slice(0, idx + 1) || trimmed;
	}
	function joinRemote(dir: string, name: string): string {
		if (dir === '.' || dir === '') return name;
		return dir.replace(/\/+$/, '') + '/' + name;
	}

	async function list(p?: string) {
		errorMsg = undefined;
		try {
			const r = await invoke<{ path: string; entries: LocalEntry[] }>('local_list', {
				path: p ?? null
			});
			lpath = r.path;
			entries = r.entries;
			onPath(r.path);
		} catch (err) {
			errorMsg = (err as { message?: string })?.message ?? String(err);
		}
	}

	function fmtSize(n: number): string {
		if (n < 1024) return `${n} B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
		return `${(n / 1024 / 1024).toFixed(1)} MB`;
	}

	onMount(() => list());
</script>

<div class="local">
	<div class="bar">
		<span class="lbl">{i18n.t('local.title')}</span>
		<button onclick={() => list(parent(lpath))} title={i18n.t('sftp.up')}>↑</button>
		<input class="path" bind:value={lpath} onkeydown={(e) => e.key === 'Enter' && list(lpath)} />
		<button onclick={() => list(lpath)} title={i18n.t('sftp.refresh')}>⟳</button>
	</div>
	{#if errorMsg}<p class="err">{errorMsg}</p>{/if}
	<ul class="list">
		{#each entries as e (e.name)}
			<li>
				<button class="row" class:dir={e.is_dir} onclick={() => e.is_dir && list(joinLocal(e.name))}>
					<span class="name">{e.is_dir ? '📁' : '📄'} {e.name}</span>
					{#if !e.is_dir}<span class="size">{fmtSize(e.size)}</span>{/if}
				</button>
				{#if !e.is_dir}
					<button
						class="up"
						title={i18n.t('sftp.upload')}
						onclick={() => app.uploadFile(sessionId, joinLocal(e.name), joinRemote(remotePath, e.name))}
					>⬆</button>
				{/if}
			</li>
		{/each}
		{#if !entries.length && !errorMsg}<li class="empty">{i18n.t('sftp.empty')}</li>{/if}
	</ul>
</div>

<style>
	.local {
		display: flex;
		flex-direction: column;
		min-height: 0;
		border-bottom: 2px solid #333;
		background: #181818;
	}
	.bar {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 5px 6px;
		border-bottom: 1px solid #2c2c2c;
	}
	.lbl {
		font-size: 11px;
		font-weight: 600;
		opacity: 0.7;
		flex: none;
	}
	.bar .path {
		flex: 1;
		min-width: 0;
		padding: 3px 6px;
		border: 1px solid #3c3c3c;
		border-radius: 4px;
		background: #1e1e1e;
		color: #eee;
		font: 12px system-ui, sans-serif;
	}
	.bar button {
		flex: none;
		padding: 3px 7px;
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
	.list li {
		display: flex;
		align-items: center;
	}
	.list li:hover {
		background: #2a2a2a;
	}
	.row {
		flex: 1;
		min-width: 0;
		display: flex;
		justify-content: space-between;
		gap: 8px;
		padding: 3px 8px;
		border: none;
		background: transparent;
		color: inherit;
		font: 12px system-ui, sans-serif;
		text-align: left;
		cursor: default;
	}
	.row.dir {
		cursor: pointer;
		color: #6cb6ff;
	}
	.row .name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.row .size {
		opacity: 0.6;
		flex: none;
	}
	.up {
		flex: none;
		padding: 2px 7px;
		border: none;
		background: transparent;
		color: #ccc;
		cursor: pointer;
	}
	.up:hover {
		background: #3a3a3a;
	}
	.err {
		margin: 4px 8px;
		color: #f48771;
		font-size: 12px;
	}
	.empty {
		padding: 8px;
		opacity: 0.5;
	}
</style>
