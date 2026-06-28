<script lang="ts">
	import { onMount, onDestroy, untrack } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { getCurrentWebview } from '@tauri-apps/api/webview';
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

	// File operations need a live session, so track the owning terminal tab's state
	// directly: when it drops the panel goes offline (operations disabled) rather
	// than failing silently against a dead session. A reconnect assigns a new
	// session id, which rebuilds this whole panel (it is keyed on the id), so a
	// fresh listing happens automatically on revival.
	const offline = $derived(
		app.tabs.find((t) => t.sessionId === sessionId)?.status !== 'connected'
	);

	let path = $state('.');
	let entries = $state<FileEntry[]>([]);
	let errorMsg = $state<string | undefined>(undefined);
	let loading = $state(false);
	let busy = $state(false);

	let newFolder = $state<string | null>(null);
	let renaming = $state<string | null>(null);
	let renameValue = $state('');
	// Entries pending deletion; non-null shows the confirm dialog. Covers both the
	// context-menu (single entry) and the selection-bar (batch) delete flows.
	let deleteTargets = $state<FileEntry[] | null>(null);

	// Multi-select (ctrl/shift) over files for batch operations like download.
	// Holds selected file names in the current directory; `anchorIdx` is the
	// shift-range pivot, an index into `shown`.
	let selected = $state<Set<string>>(new Set());
	let anchorIdx = $state<number | null>(null);
	// The keyboard "cursor" row (index into `shown`); arrow keys move it.
	let cursorIdx = $state<number | null>(null);
	const selectedCount = $derived(selected.size);

	// Filter + sort (FT-9) and chmod (FT-8).
	let filter = $state('');
	let sortKey = $state<'name' | 'size' | 'modified'>('name');
	let sortAsc = $state(true);
	let chmodTarget = $state<string | null>(null);
	let chmodValue = $state('');

	// Dual-pane (FT-10): local browser alongside the remote listing.
	let dual = $state(false);
	let localPath = $state('');

	// Follow-cd (FT-6): navigate to the shell's cwd when it changes.
	let followCd = $state(false);
	const tabCwd = $derived(app.tabs.find((t) => t.sessionId === sessionId)?.cwd);
	$effect(() => {
		const cwd = tabCwd;
		if (!followCd || !cwd) return;
		untrack(() => {
			if (cwd !== path) {
				path = cwd;
				list();
			}
		});
	});
	// Auto-refresh the listing when an upload for this session finishes. Uploads
	// run async through the transfer queue (polled by TransferQueue), so without
	// this a just-uploaded file doesn't appear until a manual refresh. Track the
	// completed uploads we've already reacted to; the ones present on first run
	// are ignored so mounting doesn't fire a redundant reload.
	let seenUploads = new Set<string>();
	let uploadsPrimed = false;
	$effect(() => {
		const done = app.transfers.filter(
			(t) => t.sessionId === sessionId && t.direction === 'upload' && t.status === 'done'
		);
		let fresh = false;
		for (const t of done) {
			if (!seenUploads.has(t.id)) {
				seenUploads.add(t.id);
				if (uploadsPrimed) fresh = true;
			}
		}
		uploadsPrimed = true;
		if (fresh) untrack(() => list());
	});

	function joinLocal(dir: string, name: string): string {
		const sep = dir.includes('\\') ? '\\' : '/';
		return dir.replace(/[\\/]+$/, '') + sep + name;
	}
	function localJoin(name: string): string {
		return joinLocal(localPath, name);
	}

	// Most shells don't emit OSC 7 over plain SSH (the cwd report follow-cd needs).
	// When enabling follow-cd, install a prompt hook so the shell reports its cwd:
	// bash via PROMPT_COMMAND, zsh via precmd.
	function toggleFollowCd() {
		followCd = !followCd;
		if (!followCd) return;
		const setup =
			'if [ -n "$ZSH_VERSION" ]; then precmd(){ printf \'\\033]7;file://%s%s\\033\\\\\' "$HOST" "$PWD"; }; ' +
			'else PROMPT_COMMAND=\'printf "\\033]7;file://%s%s\\033\\\\" "$HOSTNAME" "$PWD"\'; fi\n';
		invoke('ssh_send_input', { id: sessionId, data: setup }).catch(() => {});
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
	async function applyChmod() {
		const name = chmodTarget;
		const mode = parseInt(chmodValue, 8);
		chmodTarget = null;
		if (!name || Number.isNaN(mode)) return;
		await run(() => invoke('sftp_chmod', { id: sessionId, path: join(path, name), mode }));
	}

	// Virtual scrolling for large directories (FT-9): render only visible rows.
	const ROW_H = 40;
	const OVERSCAN = 8;
	let scroller = $state<HTMLDivElement | undefined>();
	let scrollTop = $state(0);
	let viewH = $state(0);
	const vStart = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN));
	const vEnd = $derived(Math.min(shown.length, Math.ceil((scrollTop + viewH) / ROW_H) + OVERSCAN));
	const visible = $derived(shown.slice(vStart, vEnd));

	function join(dir: string, name: string): string {
		if (dir === '.' || dir === '') return name;
		return dir.replace(/\/+$/, '') + '/' + name;
	}

	async function list() {
		if (offline) return;
		loading = true;
		errorMsg = undefined;
		clearSelection();
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
		// Ignore a second click that arrives while the first navigation is still
		// loading: a double-click would otherwise re-join the now-stale entry onto
		// the already-updated path (…/runtimes/runtimes → "No such file").
		if (!entry.is_dir || loading) return;
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

	async function commitRename() {
		const oldName = renaming;
		const name = renameValue.trim();
		renaming = null;
		if (!oldName || !name || name === oldName) return;
		await run(() =>
			invoke('sftp_rename', { id: sessionId, from: join(path, oldName), to: join(path, name) })
		);
	}

	/** Open the confirm dialog for the given entries (single or batch). */
	function requestDelete(targets: FileEntry[]) {
		if (targets.length) deleteTargets = targets;
	}
	/** Context-menu delete: act on the whole selection when the clicked row is
	 *  part of it, otherwise just that row — so it matches "delete selected". */
	function requestDeleteEntry(entry: FileEntry) {
		requestDelete(selected.has(entry.name) ? shown.filter((e) => selected.has(e.name)) : [entry]);
	}

	// Per-row actions: hidden until hover (⋯ button) and on right-click, shown as
	// a context menu instead of an always-visible icon column.
	let menu = $state<{ entry: FileEntry; x: number; y: number } | null>(null);
	function openMenu(entry: FileEntry, e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		const W = 180;
		const H = 168;
		menu = {
			entry,
			x: Math.min(e.clientX, window.innerWidth - W),
			y: Math.min(e.clientY, window.innerHeight - H)
		};
	}
	function closeMenu() {
		menu = null;
	}
	function startRename(entry: FileEntry) {
		renaming = entry.name;
		renameValue = entry.name;
	}
	/** Run a menu action and close the menu. */
	function act(fn: () => void) {
		fn();
		closeMenu();
	}

	async function upload() {
		const selected = await open({ multiple: true, title: i18n.t('sftp.upload') });
		const files = Array.isArray(selected) ? selected : typeof selected === 'string' ? [selected] : [];
		uploadFiles(files);
	}

	// Overwrite prompt (FT-5): files whose remote target already exists are held
	// back and resolved one-by-one (or "apply to all") instead of silently
	// clobbering. Non-conflicting files start uploading immediately.
	let conflicts = $state<{ local: string; remote: string; name: string }[]>([]);
	let conflictIdx = $state(0);
	let conflictApplyAll = $state(false);

	async function uploadFiles(paths: string[]) {
		if (offline) return;
		// Expand any dropped folders into their files + the dirs to create, so a
		// whole directory can be uploaded (not just flat files).
		let plan: { dirs: string[]; files: { local: string; rel: string }[] };
		try {
			plan = await invoke('expand_uploads', { paths });
		} catch (err) {
			errorMsg = (err as { message?: string })?.message ?? String(err);
			return;
		}
		// Create remote directories first, shallow-first; "already exists" is fine
		// (we merge into it), so swallow errors here.
		for (const d of plan.dirs) {
			await invoke('sftp_mkdir', { id: sessionId, path: join(path, d) }).catch(() => {});
		}
		// Top-level files prompt before overwriting (this dir's listing is loaded);
		// files inside uploaded folders go straight in.
		const existing = new Set(entries.map((e) => e.name));
		const pending: { local: string; remote: string; name: string }[] = [];
		for (const f of plan.files) {
			const remote = join(path, f.rel);
			if (!f.rel.includes('/') && existing.has(f.rel)) {
				pending.push({ local: f.local, remote, name: f.rel });
			} else {
				app.uploadFile(sessionId, f.local, remote);
			}
		}
		if (pending.length) {
			conflicts = pending;
			conflictIdx = 0;
			conflictApplyAll = false;
		}
		if (plan.dirs.length) list(); // surface the newly created folders
	}

	function endConflicts() {
		conflicts = [];
		conflictIdx = 0;
		conflictApplyAll = false;
	}

	async function resolveConflict(action: 'overwrite' | 'skip') {
		if (conflictApplyAll) {
			if (action === 'overwrite') {
				for (let i = conflictIdx; i < conflicts.length; i++) {
					await app.uploadFile(sessionId, conflicts[i].local, conflicts[i].remote);
				}
			}
			endConflicts();
			return;
		}
		if (action === 'overwrite') {
			const c = conflicts[conflictIdx];
			await app.uploadFile(sessionId, c.local, c.remote);
		}
		conflictIdx += 1;
		if (conflictIdx >= conflicts.length) endConflicts();
	}

	function clearSelection() {
		selected = new Set();
		anchorIdx = null;
		cursorIdx = null;
	}

	/** All entry names in the inclusive `shown` index range (folders included). */
	function rangeNames(a: number, b: number): string[] {
		const [lo, hi] = a <= b ? [a, b] : [b, a];
		const names: string[] = [];
		for (let i = lo; i <= hi; i++) {
			const en = shown[i];
			if (en) names.push(en.name);
		}
		return names;
	}

	/** Row click with selection semantics. A plain click selects the row (folders
	 *  and files alike) — navigation is now on double-click, so building a
	 *  multi-selection never accidentally enters a folder. Ctrl toggles, Shift
	 *  selects a range. */
	function rowClick(entry: FileEntry, idx: number, e: MouseEvent) {
		const mod = e.ctrlKey || e.metaKey;
		scroller?.focus(); // take keyboard focus so arrow keys drive selection
		cursorIdx = idx;
		const name = entry.name;
		if (e.shiftKey && anchorIdx !== null) {
			// Range from the anchor to here; ctrl extends, otherwise replaces.
			selected = new Set([...(mod ? selected : []), ...rangeNames(anchorIdx, idx)]);
		} else if (mod) {
			const next = new Set(selected);
			if (next.has(name)) next.delete(name);
			else next.add(name);
			selected = next;
			anchorIdx = idx;
		} else {
			selected = new Set([name]);
			anchorIdx = idx;
		}
	}

	/** Double-click activates a row: open a folder, or download a file. */
	function rowDblClick(entry: FileEntry) {
		if (entry.is_dir) openEntry(entry);
		else download(entry);
	}

	/** Keyboard navigation over the list: Up/Down move the cursor (selecting that
	 *  file); Shift+Up/Down extend the selection from the anchor; Enter opens a
	 *  folder; Escape clears. */
	function onListKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			clearSelection();
			return;
		}
		if (e.key === 'Enter') {
			const en = cursorIdx !== null ? shown[cursorIdx] : null;
			if (en?.is_dir) {
				e.preventDefault();
				openEntry(en);
			}
			return;
		}
		if ((e.key !== 'ArrowDown' && e.key !== 'ArrowUp') || !shown.length) return;
		e.preventDefault();
		const delta = e.key === 'ArrowDown' ? 1 : -1;
		const from = cursorIdx ?? (delta === 1 ? -1 : shown.length);
		const next = Math.max(0, Math.min(shown.length - 1, from + delta));
		cursorIdx = next;
		if (e.shiftKey) {
			if (anchorIdx === null) anchorIdx = next;
			selected = new Set(rangeNames(anchorIdx, next));
		} else {
			anchorIdx = next;
			const en = shown[next];
			selected = en ? new Set([en.name]) : new Set();
		}
		scrollToRow(next);
	}

	/** Keep row `i` within the virtual scroller's viewport. */
	function scrollToRow(i: number) {
		if (!scroller) return;
		const top = i * ROW_H;
		if (top < scroller.scrollTop) scroller.scrollTop = top;
		else if (top + ROW_H > scroller.scrollTop + scroller.clientHeight) {
			scroller.scrollTop = top + ROW_H - scroller.clientHeight;
		}
	}

	/** Join a '/'-separated relative path onto a local destination folder, using
	 *  the destination's own separator so nested folder downloads land correctly. */
	function destPath(root: string, rel: string): string {
		const sep = root.includes('\\') ? '\\' : '/';
		return root.replace(/[\\/]+$/, '') + sep + rel.split('/').join(sep);
	}

	/** Download the given entries (files and/or folders) into `destDir`, recursing
	 *  remote folders so their whole tree is fetched (mirrors folder upload). */
	async function downloadInto(targets: FileEntry[], destDir: string) {
		if (offline) return;
		const paths = targets.map((e) => join(path, e.name));
		let plan: { dirs: string[]; files: { remote: string; rel: string }[] };
		try {
			plan = await invoke('expand_downloads', { id: sessionId, paths });
		} catch (err) {
			errorMsg = (err as { message?: string })?.message ?? String(err);
			return;
		}
		// Create the local directory tree first (covers empty folders too); files
		// also create their own parents server-side, so swallow errors here.
		if (plan.dirs.length) {
			const dirs = plan.dirs.map((d) => destPath(destDir, d));
			await invoke('make_local_dirs', { paths: dirs }).catch(() => {});
		}
		for (const f of plan.files) {
			await app.downloadFile(sessionId, f.remote, destPath(destDir, f.rel));
		}
	}

	/** Batch-download every selected entry (files and folders) into one folder. */
	async function downloadSelected() {
		const items = shown.filter((e) => selected.has(e.name));
		if (!items.length) return;
		let dir = dual && localPath ? localPath : null;
		if (!dir) {
			const picked = await open({
				directory: true,
				multiple: false,
				title: i18n.t('sftp.downloadSelected')
			});
			if (typeof picked !== 'string') return;
			dir = picked;
		}
		await downloadInto(items, dir);
		clearSelection();
	}

	// Runs on confirm: deletes every pending entry (files and folders). Used by
	// both the single-entry (context menu) and batch (selection bar) flows.
	async function confirmDelete() {
		const targets = deleteTargets ?? [];
		deleteTargets = null;
		if (!targets.length) return;
		await run(async () => {
			for (const e of targets) {
				await invoke('sftp_delete', { id: sessionId, path: join(path, e.name), isDir: e.is_dir });
			}
		});
		clearSelection();
	}

	async function download(entry: FileEntry) {
		if (offline) return;
		// A folder downloads its whole tree into a chosen destination directory.
		if (entry.is_dir) {
			let dir = dual && localPath ? localPath : null;
			if (!dir) {
				const picked = await open({
					directory: true,
					multiple: false,
					title: `${i18n.t('sftp.download')} ${entry.name}`
				});
				if (typeof picked !== 'string') return;
				dir = picked;
			}
			await downloadInto([entry], dir);
			return;
		}
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

	// Drag-drop upload (FT-5): OS files dropped onto the panel upload here.
	let panelEl = $state<HTMLDivElement | undefined>();
	let dragOver = $state(false);
	let unlistenDrop: (() => void) | undefined;

	function inBounds(pos: { x: number; y: number }): boolean {
		if (!panelEl) return false;
		const r = panelEl.getBoundingClientRect();
		const dpr = window.devicePixelRatio || 1;
		const x = pos.x / dpr;
		const y = pos.y / dpr;
		return x >= r.left && x <= r.right && y >= r.top && y <= r.bottom;
	}

	onMount(() => {
		list();
		getCurrentWebview()
			.onDragDropEvent((event) => {
				const p = event.payload;
				if (p.type === 'over' || p.type === 'enter') {
					dragOver = inBounds(p.position);
				} else if (p.type === 'leave') {
					dragOver = false;
				} else if (p.type === 'drop') {
					const over = inBounds(p.position);
					dragOver = false;
					if (over) uploadFiles(p.paths);
				}
			})
			.then((un) => (unlistenDrop = un));
	});
	onDestroy(() => unlistenDrop?.());
</script>

<div class="sftp" bind:this={panelEl}>
	{#if dragOver}
		<div class="dropzone">{i18n.t('sftp.drop')}</div>
	{/if}
	<div class="bar">
		<button onclick={up} title={i18n.t('sftp.up')} disabled={busy || offline}>↑</button>
		<input
			class="path"
			aria-label="path"
			bind:value={path}
			disabled={offline}
			onkeydown={(e) => e.key === 'Enter' && list()}
		/>
		<button onclick={() => (newFolder = '')} title={i18n.t('sftp.newFolder')} disabled={busy || offline}>＋</button>
		<button onclick={upload} title={i18n.t('sftp.upload')} disabled={busy || offline}>⬆</button>
		<button class:on={dual} onclick={() => (dual = !dual)} title={i18n.t('sftp.dual')} disabled={offline}>⇆</button>
		<button class:on={followCd} onclick={toggleFollowCd} title={i18n.t('sftp.followCd')} disabled={offline}>📍</button>
		<button onclick={list} title={i18n.t('sftp.refresh')} disabled={loading || busy || offline}>⟳</button>
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

	<!-- The selection bar floats over the bottom of the list (absolute, not inline)
	     so showing it on first-select never reflows the list — otherwise the
	     rows shift between the two clicks of a double-click and the open lands on
	     the wrong row. -->
	<div class="listarea">
	{#if offline}
		<div class="offline">{i18n.t('sftp.disconnected')}</div>
	{/if}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div
		class="listwrap"
		class:has-selbar={selectedCount > 0}
		bind:this={scroller}
		bind:clientHeight={viewH}
		tabindex="0"
		onkeydown={onListKeydown}
		onscroll={() => (scrollTop = scroller?.scrollTop ?? 0)}
	>
		{#if !shown.length && !loading && !errorMsg}
			<div class="empty">{i18n.t('sftp.empty')}</div>
		{:else}
			<!-- Spacer sized to the full list; only the visible window is rendered. -->
			<div class="spacer" style="height:{shown.length * ROW_H}px">
				{#each visible as entry, vi (entry.name)}
					<div class="vrow" style="top:{(vStart + vi) * ROW_H}px; height:{ROW_H}px">
						<button
							class="row"
							class:dir={entry.is_dir}
							class:selected={selected.has(entry.name)}
							class:cursor={cursorIdx === vStart + vi}
							onclick={(e) => rowClick(entry, vStart + vi, e)}
							ondblclick={() => rowDblClick(entry)}
							oncontextmenu={(e) => openMenu(entry, e)}
						>
							<span class="top">
								<span class="name">{entry.is_dir ? '📁' : '📄'} {entry.name}{#if entry.is_symlink}<span class="link" title="symlink"> 🔗</span>{/if}</span>
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
					</div>
				{/each}
			</div>
		{/if}
	</div>

	{#if selectedCount > 0}
		<div class="selbar">
			<span class="selcount">{i18n.t('sftp.selected').replace('{n}', String(selectedCount))}</span>
			<button class="primary" onclick={downloadSelected} disabled={busy}>⬇ {i18n.t('sftp.downloadSelected')}</button>
			<button onclick={() => requestDelete(shown.filter((e) => selected.has(e.name)))} disabled={busy}>
				🗑 {i18n.t('sftp.deleteSelected')}
			</button>
			<button onclick={clearSelection}>{i18n.t('sftp.clearSelection')}</button>
		</div>
	{/if}
	</div>

	{#if menu}
		<button
			class="ctx-scrim"
			aria-label="close menu"
			onclick={closeMenu}
			oncontextmenu={(e) => {
				e.preventDefault();
				closeMenu();
			}}
		></button>
		{@const target = menu.entry}
		<div class="ctx" style="left:{menu.x}px; top:{menu.y}px">
			<button class="ctx-item" onclick={() => act(() => download(target))}>⬇ {i18n.t('sftp.download')}</button>
			<button class="ctx-item" onclick={() => act(() => openChmod(target))}>⚙ {i18n.t('sftp.chmod')}</button>
			<button class="ctx-item" onclick={() => act(() => startRename(target))}>✎ {i18n.t('sftp.rename')}</button>
			<button class="ctx-item danger" onclick={() => act(() => requestDeleteEntry(target))}>
				🗑 {i18n.t('common.delete')}
			</button>
		</div>
	{/if}

	{#if renaming !== null}
		<div
			class="modal-backdrop"
			role="presentation"
			onclick={(e) => {
				if (e.target === e.currentTarget) renaming = null;
			}}
		>
			<div class="modal">
				<h3>{i18n.t('sftp.rename')}</h3>
				<!-- svelte-ignore a11y_autofocus -->
				<input
					autofocus
					bind:value={renameValue}
					onkeydown={(e) => {
						if (e.key === 'Enter') commitRename();
						else if (e.key === 'Escape') renaming = null;
					}}
				/>
				<div class="acts">
					<button class="ghost" onclick={() => (renaming = null)}>{i18n.t('common.cancel')}</button>
					<button onclick={commitRename} disabled={busy}>{i18n.t('common.save')}</button>
				</div>
			</div>
		</div>
	{/if}

	{#if chmodTarget}
		<div
			class="modal-backdrop"
			role="presentation"
			onclick={(e) => {
				if (e.target === e.currentTarget) chmodTarget = null;
			}}
		>
			<div class="modal">
				<h3>{i18n.t('sftp.chmod')}</h3>
				<p class="sub">{chmodTarget}</p>
				<!-- svelte-ignore a11y_autofocus -->
				<input
					class="octal"
					autofocus
					bind:value={chmodValue}
					onkeydown={(e) => {
						if (e.key === 'Enter') applyChmod();
						else if (e.key === 'Escape') chmodTarget = null;
					}}
				/>
				<div class="acts">
					<button class="ghost" onclick={() => (chmodTarget = null)}>{i18n.t('common.cancel')}</button>
					<button onclick={applyChmod} disabled={busy}>{i18n.t('sftp.apply')}</button>
				</div>
			</div>
		</div>
	{/if}

	{#if deleteTargets}
		<div
			class="modal-backdrop"
			role="presentation"
			onclick={(e) => {
				if (e.target === e.currentTarget) deleteTargets = null;
			}}
		>
			<div class="modal">
				<h3>{i18n.t('common.delete')}</h3>
				<p class="sub">
					{deleteTargets.length === 1
						? i18n.t('sftp.deleteConfirmOne').replace('{name}', deleteTargets[0].name)
						: i18n.t('sftp.deleteConfirm').replace('{n}', String(deleteTargets.length))}
				</p>
				<div class="acts">
					<button class="ghost" onclick={() => (deleteTargets = null)}>{i18n.t('common.cancel')}</button>
					<button class="danger" onclick={confirmDelete} disabled={busy}>{i18n.t('common.delete')}</button>
				</div>
			</div>
		</div>
	{/if}

	{#if conflicts.length && conflictIdx < conflicts.length}
		{@const c = conflicts[conflictIdx]}
		<div class="modal-backdrop" role="presentation">
			<div class="modal">
				<h3>{i18n.t('sftp.overwriteTitle')}</h3>
				<p class="sub">{i18n.t('sftp.overwritePrompt').replace('{name}', c.name)}</p>
				{#if conflicts.length - conflictIdx > 1}
					<label class="applyall">
						<input type="checkbox" bind:checked={conflictApplyAll} />
						{i18n.t('sftp.applyToAll').replace('{n}', String(conflicts.length - conflictIdx))}
					</label>
				{/if}
				<div class="acts">
					<button class="ghost" onclick={() => resolveConflict('skip')}>{i18n.t('sftp.skip')}</button>
					<button onclick={() => resolveConflict('overwrite')}>{i18n.t('sftp.overwrite')}</button>
				</div>
			</div>
		</div>
	{/if}

	<TransferQueue {sessionId} />
</div>

<style>
	.sftp {
		position: relative;
		display: flex;
		flex-direction: column;
		/* Fill the space left under `.view-head`, not the whole sidebar — using
		   height:100% here pushed the bottom (the transfer queue) off-screen. */
		flex: 1;
		min-height: 0;
		color: var(--vsc-sidebar-fg);
		font: 13px var(--vsc-font);
		background: var(--vsc-sidebar-bg);
	}
	.dropzone {
		position: absolute;
		inset: 0;
		z-index: 8;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(14, 99, 156, 0.25);
		border: 2px dashed var(--vsc-button-bg);
		border-radius: 6px;
		color: #cfe6ff;
		font-size: 14px;
		pointer-events: none;
	}
	.bar {
		display: flex;
		gap: 4px;
		padding: 6px;
		border-bottom: 1px solid var(--vsc-border);
	}
	.bar .path {
		flex: 1;
		min-width: 0;
	}
	input {
		padding: 4px 6px;
		border: 1px solid var(--vsc-input-border);
		border-radius: 4px;
		background: var(--vsc-input-bg);
		color: var(--vsc-input-fg);
		font: 13px var(--vsc-font);
	}
	input:focus {
		outline: 1px solid var(--vsc-focus-border);
		outline-offset: -1px;
	}
	.bar button,
	.new-folder button {
		padding: 4px 8px;
		border: none;
		border-radius: 3px;
		background: var(--vsc-button-secondary-bg);
		color: var(--vsc-button-secondary-fg);
		cursor: pointer;
	}
	.bar button:hover,
	.new-folder button:hover {
		background: var(--vsc-button-secondary-hover);
	}
	.bar button.on {
		background: var(--vsc-button-bg);
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
		border-bottom: 1px solid var(--vsc-border);
		background: rgba(0, 0, 0, 0.18);
	}
	.new-folder input {
		flex: 1;
		min-width: 0;
	}
	.listarea {
		position: relative;
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
	}
	.selbar {
		position: absolute;
		left: 0;
		right: 0;
		bottom: 0;
		z-index: 6;
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 5px 8px;
		border-top: 1px solid var(--vsc-border);
		background: var(--vsc-sidebar-bg);
		box-shadow: 0 -2px 8px rgba(0, 0, 0, 0.25);
	}
	.selbar .selcount {
		flex: 1;
		min-width: 0;
		font-size: 12px;
		color: var(--vsc-muted);
	}
	.selbar button {
		padding: 4px 8px;
		border: none;
		border-radius: 3px;
		background: var(--vsc-button-secondary-bg);
		color: var(--vsc-button-secondary-fg);
		font: 12px var(--vsc-font);
		cursor: pointer;
	}
	.selbar button:hover {
		background: var(--vsc-button-secondary-hover);
	}
	.selbar button.primary {
		background: var(--vsc-button-bg);
		color: #fff;
	}
	.selbar button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.listwrap {
		flex: 1;
		min-height: 0;
		overflow: auto;
		position: relative;
	}
	/* Reserve room so the floating selbar never hides the last rows. Padding on
	   the scroll container doesn't move rows on screen, so it can't disturb a
	   double-click in progress. */
	.listwrap.has-selbar {
		padding-bottom: 40px;
	}
	.spacer {
		position: relative;
		width: 100%;
	}
	.filterbar {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 6px;
		border-bottom: 1px solid var(--vsc-border);
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
		border: none;
		border-radius: 3px;
		background: var(--vsc-button-secondary-bg);
		color: var(--vsc-sidebar-fg);
		font: 11px var(--vsc-font);
		cursor: pointer;
	}
	.sorts button:hover {
		background: var(--vsc-button-secondary-hover);
	}
	.sorts button.on {
		background: var(--vsc-button-bg);
		color: #fff;
	}
	.vrow {
		position: absolute;
		left: 0;
		right: 0;
		display: flex;
		align-items: center;
		box-sizing: border-box;
	}
	.vrow:hover {
		background: var(--vsc-list-hover-bg);
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
	/* Highlight the whole row when its file is part of the selection. */
	.vrow:has(.row.selected) {
		background: var(--vsc-list-active-bg);
	}
	/* The keyboard cursor row gets a focus ring (inset, so it never shifts layout). */
	.vrow:has(.row.cursor) {
		box-shadow: inset 0 0 0 1px var(--vsc-focus-border);
	}
	.listwrap:focus {
		outline: none;
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
		color: var(--vsc-muted);
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
	.modal-backdrop {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.55);
		z-index: 30;
	}
	.modal {
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: 320px;
		max-width: 92vw;
		box-sizing: border-box;
		padding: 18px 20px;
		background: var(--vsc-widget-bg);
		border: 1px solid var(--vsc-widget-border);
		border-radius: 6px;
		color: var(--vsc-editor-fg);
		font: 13px var(--vsc-font);
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.44);
	}
	.modal h3 {
		margin: 0;
		font-size: 15px;
		font-weight: 600;
	}
	.modal .sub {
		margin: 0;
		font-size: 12px;
		color: var(--vsc-muted);
		word-break: break-all;
	}
	.modal .applyall {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		color: var(--vsc-muted);
		cursor: pointer;
	}
	.modal .applyall input {
		width: auto;
		margin: 0;
	}
	.modal input {
		padding: 7px 9px;
		border: 1px solid var(--vsc-input-border);
		border-radius: 4px;
		background: var(--vsc-input-bg);
		color: var(--vsc-input-fg);
		font: 13px var(--vsc-font);
		box-sizing: border-box;
	}
	.modal input:focus {
		outline: 1px solid var(--vsc-focus-border);
		outline-offset: -1px;
	}
	.modal .acts {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
	}
	.modal button {
		padding: 7px 14px;
		border: none;
		border-radius: 3px;
		background: var(--vsc-button-bg);
		color: #fff;
		font: 13px var(--vsc-font);
		cursor: pointer;
	}
	.modal button:hover {
		background: var(--vsc-button-hover);
	}
	.modal button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.modal .ghost {
		background: var(--vsc-button-secondary-bg);
		color: var(--vsc-button-secondary-fg);
	}
	.modal .ghost:hover {
		background: var(--vsc-button-secondary-hover);
	}
	.modal .danger {
		background: var(--vsc-danger, #c4314b);
	}
	.modal .danger:hover {
		background: var(--vsc-danger-hover, #a8293f);
	}
	.ctx-scrim {
		position: fixed;
		inset: 0;
		z-index: 20;
		border: none;
		background: transparent;
		cursor: default;
	}
	.ctx-scrim:hover {
		background: transparent;
	}
	.ctx {
		position: fixed;
		z-index: 21;
		min-width: 160px;
		padding: 4px;
		background: var(--vsc-widget-bg);
		border: 1px solid var(--vsc-widget-border);
		border-radius: var(--vsc-radius);
		box-shadow: 0 4px 14px var(--vsc-widget-shadow);
		display: flex;
		flex-direction: column;
	}
	.ctx-item {
		padding: 6px 10px;
		border: none;
		border-radius: 4px;
		background: transparent;
		color: var(--vsc-sidebar-fg);
		font: 13px var(--vsc-font);
		text-align: left;
		cursor: pointer;
	}
	.ctx-item:hover {
		background: var(--vsc-button-bg);
		color: #fff;
	}
	.ctx-item.danger {
		color: var(--vsc-red);
	}
	.ctx-item.danger:hover {
		background: var(--vsc-red);
		color: #fff;
	}
	.err {
		margin: 6px 8px;
		color: var(--vsc-red);
		font-size: 12px;
		word-break: break-word;
	}
	.empty {
		padding: 8px;
		opacity: 0.5;
	}
	/* Covers the file listing while the session is down so stale rows can't be
	   acted on; the transfer queue below stays visible. */
	.offline {
		position: absolute;
		inset: 0;
		z-index: 7;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 16px;
		text-align: center;
		background: var(--vsc-sidebar-bg);
		color: var(--vsc-sidebar-fg);
		opacity: 0.92;
		font-size: 12px;
	}
</style>
