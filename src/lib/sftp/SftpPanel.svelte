<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open, save } from '@tauri-apps/plugin-dialog';
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import TransferQueue from './TransferQueue.svelte';
	import LocalPane from './LocalPane.svelte';
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

	// Filter + sort (FT-9) and chmod (FT-8).
	let filter = $state('');
	let sortKey = $state<'name' | 'size' | 'modified'>('name');
	let sortAsc = $state(true);
	let chmodTarget = $state<string | null>(null);
	let chmodValue = $state('');

	// Dual-pane (FT-10): local browser alongside the remote listing.
	let dual = $state(false);
	let localPath = $state('');
	function localJoin(name: string): string {
		const sep = localPath.includes('\\') ? '\\' : '/';
		return localPath.replace(/[\\/]+$/, '') + sep + name;
	}

	const shown = $derived.by(() => {
		const q = filter.trim().toLowerCase();
		const out = entries.filter((e) => !q || e.name.toLowerCase().includes(q));
		const dir = sortAsc ? 1 : -1;
		out.sort((a, b) => {
			if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1; // dirs first always
			let c = 0;
			if (sortKey === 'size') c = a.size - b.size;
			else if (sortKey === 'modified') c = (a.modified ?? 0) - (b.modified ?? 0);
			else c = a.name.toLowerCase().localeCompare(b.name.toLowerCase());
			return c * dir;
		});
		return out;
	});

	function setSort(key: 'name' | 'size' | 'modified') {
		if (sortKey === key) sortAsc = !sortAsc;
		else {
			sortKey = key;
			sortAsc = true;
		}
	}

	/** Render the low 9 permission bits as an `rwxr-xr-x` string. */
	function permString(mode: number | null): string {
		if (mode == null) return '';
		const ch = ['x', 'w', 'r'];
		let s = '';
		for (let i = 8; i >= 0; i--) s += mode & (1 << i) ? ch[i % 3] : '-';
		return s;
	}
	function fmtDate(mtime: number | null): string {
		if (!mtime) return '';
		return new Date(mtime * 1000).toISOString().slice(0, 16).replace('T', ' ');
	}

	function openChmod(entry: FileEntry) {
		chmodTarget = entry.name;
		chmodValue = ((entry.permissions ?? 0) & 0o777).toString(8).padStart(3, '0');
	}
	async function applyChmod(entry: FileEntry) {
		const mode = parseInt(chmodValue, 8);
		chmodTarget = null;
		if (Number.isNaN(mode)) return;
		await run(() => invoke('sftp_chmod', { id: sessionId, path: join(path, entry.name), mode }));
	}

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
		const selected = await open({ multiple: true, title: i18n.t('sftp.upload') });
		const files = Array.isArray(selected) ? selected : typeof selected === 'string' ? [selected] : [];
		for (const f of files) {
			await app.uploadFile(sessionId, f, join(path, basename(f)));
		}
	}

	async function download(entry: FileEntry) {
		// In dual-pane mode download straight into the local pane's folder.
		if (dual && localPath) {
			await app.downloadFile(sessionId, join(path, entry.name), localJoin(entry.name));
			return;
		}
		const target = await save({ defaultPath: entry.name, title: `${i18n.t('sftp.download')} ${entry.name}` });
		if (typeof target !== 'string') return;
		await app.downloadFile(sessionId, join(path, entry.name), target);
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
		<button class:on={dual} onclick={() => (dual = !dual)} title={i18n.t('sftp.dual')}>⇆</button>
		<button onclick={list} title={i18n.t('sftp.refresh')} disabled={loading || busy}>⟳</button>
	</div>

	{#if dual}
		<div class="localwrap">
			<LocalPane {sessionId} remotePath={path} onPath={(p) => (localPath = p)} />
		</div>
	{/if}

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

	<div class="filterbar">
		<input class="filter" placeholder={i18n.t('sftp.filter')} bind:value={filter} />
		<div class="sorts">
			<button class:on={sortKey === 'name'} onclick={() => setSort('name')}>
				{i18n.t('sftp.sortName')}{sortKey === 'name' ? (sortAsc ? ' ↑' : ' ↓') : ''}
			</button>
			<button class:on={sortKey === 'size'} onclick={() => setSort('size')}>
				{i18n.t('sftp.sortSize')}{sortKey === 'size' ? (sortAsc ? ' ↑' : ' ↓') : ''}
			</button>
			<button class:on={sortKey === 'modified'} onclick={() => setSort('modified')}>
				{i18n.t('sftp.sortModified')}{sortKey === 'modified' ? (sortAsc ? ' ↑' : ' ↓') : ''}
			</button>
		</div>
	</div>

	{#if errorMsg}
		<p class="err">{errorMsg}</p>
	{/if}

	<ul class="list">
		{#each shown as entry (entry.name)}
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
						<span class="top">
							<span class="name">{entry.is_dir ? '📁' : '📄'} {entry.name}</span>
							{#if !entry.is_dir}<span class="size">{fmtSize(entry.size)}</span>{/if}
						</span>
						{#if entry.permissions != null || entry.modified}
							<span class="meta">
								{#if entry.permissions != null}<span class="perm">{permString(entry.permissions)}</span>{/if}
								{#if entry.uid != null}<span>{entry.uid}:{entry.gid ?? 0}</span>{/if}
								{#if entry.modified}<span>{fmtDate(entry.modified)}</span>{/if}
							</span>
						{/if}
					</button>
					<div class="ops">
						{#if !entry.is_dir}
							<button class="sm" title={i18n.t('sftp.download')} onclick={() => download(entry)} disabled={busy}
								>⬇</button
							>
						{/if}
						<button class="sm" title={i18n.t('sftp.chmod')} onclick={() => openChmod(entry)} disabled={busy}>⚙</button>
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
				{#if chmodTarget === entry.name}
					<div class="chmod">
						<span class="lbl">{i18n.t('sftp.chmod')}</span>
						<input
							class="octal"
							bind:value={chmodValue}
							onkeydown={(e) => {
								if (e.key === 'Enter') applyChmod(entry);
								else if (e.key === 'Escape') chmodTarget = null;
							}}
						/>
						<button onclick={() => applyChmod(entry)} disabled={busy}>{i18n.t('sftp.apply')}</button>
					</div>
				{/if}
			</li>
		{/each}
		{#if !shown.length && !loading && !errorMsg}
			<li class="empty">{i18n.t('sftp.empty')}</li>
		{/if}
	</ul>

	<TransferQueue {sessionId} />
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
	.bar button.on {
		background: #0e639c;
		border-color: #0e639c;
		color: #fff;
	}
	.localwrap {
		flex: 0 0 40%;
		min-height: 0;
		display: flex;
		overflow: hidden;
	}
	.localwrap :global(.local) {
		flex: 1;
		height: 100%;
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
	.filterbar {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 6px;
		border-bottom: 1px solid #333;
	}
	.filterbar .filter {
		width: 100%;
		box-sizing: border-box;
	}
	.sorts {
		display: flex;
		gap: 4px;
	}
	.sorts button {
		flex: 1;
		padding: 3px 4px;
		border: 1px solid #3c3c3c;
		border-radius: 4px;
		background: #232323;
		color: #bbb;
		font: 11px system-ui, sans-serif;
		cursor: pointer;
	}
	.sorts button.on {
		background: #0e639c;
		border-color: #0e639c;
		color: #fff;
	}
	.list li {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
	}
	.list li:hover {
		background: #2a2a2a;
	}
	.row {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
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
	.row .top {
		display: flex;
		justify-content: space-between;
		gap: 8px;
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
	.row .meta {
		display: flex;
		gap: 8px;
		font-size: 10px;
		opacity: 0.5;
	}
	.row .meta .perm {
		font-family: Consolas, monospace;
	}
	.chmod {
		flex-basis: 100%;
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 8px 6px;
	}
	.chmod .lbl {
		font-size: 11px;
		opacity: 0.7;
	}
	.chmod .octal {
		width: 70px;
	}
	.chmod button {
		padding: 3px 10px;
		border: none;
		border-radius: 4px;
		background: #0e639c;
		color: #fff;
		cursor: pointer;
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
