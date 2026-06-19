<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open, save } from '@tauri-apps/plugin-dialog';
	import { i18n } from '$lib/i18n.svelte';
	import type { FileEntry } from './types';

	interface Props {
		sessionId: string;
	}
	let { sessionId }: Props = $props();

	let path = $state('.');
	let entries = $state<FileEntry[]>([]);
	let errorMsg = $state<string | undefined>(undefined);
	let loading = $state(false);
	let busy = $state(false);

	let newFolder = $state<string | null>(null);
	let renaming = $state<string | null>(null);
	let renameValue = $state('');
	let confirmingDelete = $state<string | null>(null);

	function join(dir: string, name: string): string {
		if (dir === '.' || dir === '') return name;
		return dir.replace(/\/+$/, '') + '/' + name;
	}
	function basename(p: string): string {
		return p.split(/[\\/]/).pop() ?? p;
	}

	async function list() {
		loading = true;
		errorMsg = undefined;
		try {
			entries = await invoke<FileEntry[]>('sftp_list', { id: sessionId, path });
		} catch (err) {
			errorMsg = (err as { message?: string })?.message ?? String(err);
		} finally {
			loading = false;
		}
	}

	async function run(fn: () => Promise<void>) {
		busy = true;
		errorMsg = undefined;
		try {
			await fn();
			await list();
		} catch (err) {
			errorMsg = (err as { message?: string })?.message ?? String(err);
		} finally {
			busy = false;
		}
	}

	function openEntry(entry: FileEntry) {
		if (!entry.is_dir) return;
		path = join(path, entry.name);
		list();
	}
	function up() {
		const trimmed = path.replace(/\/$/, '');
		const idx = trimmed.lastIndexOf('/');
		path = idx > 0 ? trimmed.slice(0, idx) : idx === 0 ? '/' : '.';
		list();
	}

	async function createFolder() {
		const name = (newFolder ?? '').trim();
		if (!name) {
			newFolder = null;
			return;
		}
		await run(() => invoke('sftp_mkdir', { id: sessionId, path: join(path, name) }));
		newFolder = null;
	}

	async function commitRename(entry: FileEntry) {
		const name = renameValue.trim();
		renaming = null;
		if (!name || name === entry.name) return;
		await run(() =>
			invoke('sftp_rename', { id: sessionId, from: join(path, entry.name), to: join(path, name) })
		);
	}

	async function del(entry: FileEntry) {
		if (confirmingDelete !== entry.name) {
			confirmingDelete = entry.name;
			return;
		}
		confirmingDelete = null;
		await run(() =>
			invoke('sftp_delete', { id: sessionId, path: join(path, entry.name), isDir: entry.is_dir })
		);
	}

	async function upload() {
		const selected = await open({ multiple: false, title: i18n.t('sftp.upload') });
		if (typeof selected !== 'string') return;
		await run(() =>
			invoke('sftp_upload', {
				id: sessionId,
				localPath: selected,
				remotePath: join(path, basename(selected))
			})
		);
	}

	async function download(entry: FileEntry) {
		const target = await save({ defaultPath: entry.name, title: `${i18n.t('sftp.download')} ${entry.name}` });
		if (typeof target !== 'string') return;
		await run(() =>
			invoke('sftp_download', {
				id: sessionId,
				remotePath: join(path, entry.name),
				localPath: target
			})
		);
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
		<button onclick={up} title={i18n.t('sftp.up')} disabled={busy}>↑</button>
		<input
			class="path"
			aria-label="path"
			bind:value={path}
			onkeydown={(e) => e.key === 'Enter' && list()}
		/>
		<button onclick={() => (newFolder = '')} title={i18n.t('sftp.newFolder')} disabled={busy}>＋</button>
		<button onclick={upload} title={i18n.t('sftp.upload')} disabled={busy}>⬆</button>
		<button onclick={list} title={i18n.t('sftp.refresh')} disabled={loading || busy}>⟳</button>
	</div>

	{#if newFolder !== null}
		<div class="new-folder">
			<input
				placeholder={i18n.t('sftp.folderName')}
				bind:value={newFolder}
				onkeydown={(e) => {
					if (e.key === 'Enter') createFolder();
					else if (e.key === 'Escape') newFolder = null;
				}}
			/>
			<button onclick={createFolder}>{i18n.t('sftp.create')}</button>
		</div>
	{/if}

	{#if errorMsg}
		<p class="err">{errorMsg}</p>
	{/if}

	<ul class="list">
		{#each entries as entry (entry.name)}
			<li>
				{#if renaming === entry.name}
					<input
						class="rename"
						bind:value={renameValue}
						onkeydown={(e) => {
							if (e.key === 'Enter') commitRename(entry);
							else if (e.key === 'Escape') renaming = null;
						}}
					/>
				{:else}
					<button class="row" class:dir={entry.is_dir} onclick={() => openEntry(entry)}>
						<span class="name">{entry.is_dir ? '📁' : '📄'} {entry.name}</span>
						{#if !entry.is_dir}<span class="size">{fmtSize(entry.size)}</span>{/if}
					</button>
					<div class="ops">
						{#if !entry.is_dir}
							<button class="sm" title={i18n.t('sftp.download')} onclick={() => download(entry)} disabled={busy}
								>⬇</button
							>
						{/if}
						<button
							class="sm"
							title={i18n.t('sftp.rename')}
							onclick={() => {
								renaming = entry.name;
								renameValue = entry.name;
							}}
							disabled={busy}>✎</button
						>
						<button
							class="sm"
							class:danger={confirmingDelete === entry.name}
							title={i18n.t('common.delete')}
							onclick={() => del(entry)}
							disabled={busy}
						>
							{confirmingDelete === entry.name ? i18n.t('common.sure') : '🗑'}
						</button>
					</div>
				{/if}
			</li>
		{/each}
		{#if !entries.length && !loading && !errorMsg}
			<li class="empty">{i18n.t('sftp.empty')}</li>
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
	.bar .path {
		flex: 1;
		min-width: 0;
	}
	input {
		padding: 4px 6px;
		border: 1px solid #3c3c3c;
		border-radius: 4px;
		background: #1e1e1e;
		color: #eee;
		font: 13px system-ui, sans-serif;
	}
	.bar button,
	.new-folder button {
		padding: 4px 8px;
		border: 1px solid #444;
		border-radius: 4px;
		background: #2a2a2a;
		color: #ddd;
		cursor: pointer;
	}
	.new-folder {
		display: flex;
		gap: 4px;
		padding: 6px;
		border-bottom: 1px solid #333;
		background: #161616;
	}
	.new-folder input {
		flex: 1;
		min-width: 0;
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
	.row .name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.row .size {
		opacity: 0.6;
		font-variant-numeric: tabular-nums;
		flex: none;
	}
	.rename {
		flex: 1;
		margin: 2px 6px;
	}
	.ops {
		display: flex;
		gap: 2px;
		padding-right: 4px;
	}
	.ops .sm {
		padding: 2px 6px;
		border: none;
		border-radius: 4px;
		background: transparent;
		color: #ccc;
		cursor: pointer;
	}
	.ops .sm:hover {
		background: #3a3a3a;
	}
	.ops .sm.danger {
		color: #f48771;
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
