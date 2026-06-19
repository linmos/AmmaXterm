<script lang="ts">
	import { app } from '$lib/state.svelte';
	import SiteDialog from './SiteDialog.svelte';
	import type { Site } from './types';

	let showDialog = $state(false);
	let dialogSite = $state<Site | undefined>(undefined);
	let confirmingDelete = $state<string | null>(null);

	// Quick Connect form.
	let showQuick = $state(false);
	let qHost = $state('');
	let qPort = $state(22);
	let qUser = $state('');
	let qPass = $state('');
	let filter = $state('');

	const visibleSites = $derived(
		app.sites.filter((s) => {
			const q = filter.trim().toLowerCase();
			if (!q) return true;
			return (
				s.name.toLowerCase().includes(q) ||
				s.host.toLowerCase().includes(q) ||
				s.username.toLowerCase().includes(q)
			);
		})
	);

	function newSite() {
		dialogSite = undefined;
		showDialog = true;
	}
	function editSite(site: Site) {
		dialogSite = site;
		showDialog = true;
	}
	async function confirmDelete(site: Site) {
		if (confirmingDelete === site.id) {
			await app.deleteSite(site.id);
			confirmingDelete = null;
		} else {
			confirmingDelete = site.id;
		}
	}
	async function quickConnect(event: Event) {
		event.preventDefault();
		await app.quickConnect({ host: qHost, port: Number(qPort), username: qUser, password: qPass });
		qPass = '';
		showQuick = false;
	}
</script>

<aside class="sidebar">
	<div class="head">
		<strong>AmmaXterm</strong>
		<div class="head-actions">
			<button class="ghost" title="Quick connect" onclick={() => (showQuick = !showQuick)}>⚡</button>
			<button class="ghost" title="New site" onclick={newSite}>＋</button>
		</div>
	</div>

	{#if showQuick}
		<form class="quick" onsubmit={quickConnect}>
			<input placeholder="Host" bind:value={qHost} required />
			<div class="row">
				<input class="grow" placeholder="User" bind:value={qUser} required />
				<input class="port" type="number" placeholder="22" bind:value={qPort} />
			</div>
			<input type="password" placeholder="Password" bind:value={qPass} />
			<button type="submit">Connect</button>
		</form>
	{/if}

	<input class="filter" placeholder="Search sites…" bind:value={filter} />

	<ul class="sites">
		{#each visibleSites as site (site.id)}
			<li>
				<button class="site" ondblclick={() => app.connectSite(site)} title="Double-click to connect">
					<span class="name">{site.name}</span>
					<span class="addr">{site.username}@{site.host}:{site.port}</span>
				</button>
				<div class="site-actions">
					<button class="ghost sm" title="Edit" onclick={() => editSite(site)}>✎</button>
					<button
						class="ghost sm"
						class:danger={confirmingDelete === site.id}
						title="Delete"
						onclick={() => confirmDelete(site)}
					>
						{confirmingDelete === site.id ? 'Sure?' : '🗑'}
					</button>
				</div>
			</li>
		{/each}
		{#if !visibleSites.length}
			<li class="empty">No sites yet — click ＋ to add one.</li>
		{/if}
	</ul>
</aside>

{#if showDialog}
	<SiteDialog site={dialogSite} onclose={() => (showDialog = false)} />
{/if}

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: #202020;
		color: #ddd;
		font: 13px system-ui, sans-serif;
		border-right: 1px solid #333;
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 10px;
		border-bottom: 1px solid #333;
	}
	.head-actions {
		display: flex;
		gap: 4px;
	}
	.quick {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 8px 10px;
		border-bottom: 1px solid #333;
		background: #1b1b1b;
	}
	.quick .row {
		display: flex;
		gap: 6px;
	}
	.quick .grow {
		flex: 1;
		min-width: 0;
	}
	.quick .port {
		width: 64px;
	}
	.filter {
		margin: 8px 10px;
	}
	input {
		padding: 6px 8px;
		border: 1px solid #3c3c3c;
		border-radius: 5px;
		background: #1e1e1e;
		color: #eee;
		font: 13px system-ui, sans-serif;
	}
	.sites {
		flex: 1;
		min-height: 0;
		overflow: auto;
		margin: 0;
		padding: 0 6px;
		list-style: none;
	}
	.sites li {
		display: flex;
		align-items: center;
		gap: 2px;
		border-radius: 6px;
	}
	.sites li:hover {
		background: #2a2a2a;
	}
	.site {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding: 6px 8px;
		border: none;
		background: transparent;
		color: inherit;
		text-align: left;
		cursor: pointer;
	}
	.site .name {
		font-weight: 600;
	}
	.site .addr {
		font-size: 11px;
		opacity: 0.6;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.site-actions {
		display: flex;
		gap: 2px;
		padding-right: 4px;
	}
	.empty {
		padding: 12px 8px;
		opacity: 0.5;
	}
	button {
		padding: 6px 10px;
		border: none;
		border-radius: 6px;
		background: #0e639c;
		color: #fff;
		font: 13px system-ui, sans-serif;
		cursor: pointer;
	}
	.ghost {
		background: transparent;
		border: 1px solid #444;
		color: #ddd;
	}
	.ghost.sm {
		padding: 4px 6px;
		border: none;
	}
	.ghost.danger {
		color: #f48771;
		border: 1px solid #f48771;
	}
</style>
