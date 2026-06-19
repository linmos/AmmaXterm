<script lang="ts">
	import { untrack } from 'svelte';
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import type { AuthMethod, Site, SiteInput } from './types';
	import type { TunnelSpec } from '../tunnel/types';

	interface Props {
		site?: Site;
		onclose: () => void;
	}
	let { site, onclose }: Props = $props();

	const editing = untrack(() => site != null);

	// Snapshot the site's initial values once (the dialog is recreated per open).
	const init = untrack(() => ({
		name: site?.name ?? '',
		host: site?.host ?? '',
		port: site?.port ?? 22,
		username: site?.username ?? '',
		authType: (site?.auth.type ?? 'password') as AuthMethod['type'],
		keyPath: site && site.auth.type === 'publicKey' ? site.auth.keyPath : '',
		group: site?.group ?? '',
		tags: (site?.tags ?? []).join(', '),
		tunnels: (site?.tunnels ?? []).map((t) => ({ ...t }))
	}));

	let name = $state(init.name);
	let host = $state(init.host);
	let port = $state(init.port);
	let username = $state(init.username);
	let authType = $state<AuthMethod['type']>(init.authType);
	let keyPath = $state(init.keyPath);
	let group = $state(init.group);
	let tags = $state(init.tags);
	let tunnels = $state<TunnelSpec[]>(init.tunnels);
	let password = $state('');
	let saving = $state(false);
	let errorMsg = $state<string | undefined>(undefined);

	// Inline "add tunnel" row.
	let tKind = $state('local');
	let tListen = $state(8080);
	let tDestHost = $state('');
	let tDestPort = $state(80);

	function addTunnel() {
		if (!tListen) return;
		if (tKind === 'local' && (!tDestHost || !tDestPort)) return;
		tunnels = [
			...tunnels,
			{
				kind: tKind,
				listenPort: Number(tListen),
				destHost: tKind === 'local' ? tDestHost : '',
				destPort: tKind === 'local' ? Number(tDestPort) : 0
			}
		];
		tDestHost = '';
	}
	function removeTunnel(i: number) {
		tunnels = tunnels.filter((_, idx) => idx !== i);
	}
	function tunnelLabel(t: TunnelSpec): string {
		return t.kind === 'dynamic'
			? `D · SOCKS5 :${t.listenPort}`
			: `L · :${t.listenPort} → ${t.destHost}:${t.destPort}`;
	}

	// Existing group names for the datalist (autocomplete), de-duplicated.
	const groupOptions = $derived(
		[...new Set(app.sites.map((s) => s.group).filter((g): g is string => !!g))].sort()
	);

	function buildAuth(): AuthMethod {
		switch (authType) {
			case 'publicKey':
				return { type: 'publicKey', keyPath };
			case 'keyboardInteractive':
				return { type: 'keyboardInteractive' };
			case 'agent':
				return { type: 'agent' };
			default:
				return { type: 'password' };
		}
	}

	async function save(event: Event) {
		event.preventDefault();
		saving = true;
		errorMsg = undefined;
		const input: SiteInput = {
			name,
			host,
			port: Number(port),
			username,
			auth: buildAuth(),
			group: group.trim() || null,
			tags: tags
				.split(',')
				.map((t) => t.trim())
				.filter(Boolean),
			tunnels
		};
		try {
			if (editing && site) await app.updateSite(site.id, input, password || undefined);
			else await app.addSite(input, password || undefined);
			onclose();
		} catch (err) {
			errorMsg = (err as { message?: string })?.message ?? String(err);
		} finally {
			saving = false;
		}
	}
</script>

<div
	class="backdrop"
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose();
	}}
>
	<form class="dialog" onsubmit={save}>
		<h2>{editing ? i18n.t('site.edit') : i18n.t('site.new')}</h2>

		<label>{i18n.t('site.name')}<input bind:value={name} required /></label>
		<label>{i18n.t('common.host')}<input bind:value={host} required /></label>
		<div class="row">
			<label class="grow">{i18n.t('common.user')}<input bind:value={username} required /></label>
			<label class="port">{i18n.t('common.port')}<input type="number" min="1" max="65535" bind:value={port} /></label>
		</div>

		<label>
			{i18n.t('site.auth')}
			<select bind:value={authType}>
				<option value="password">{i18n.t('auth.password')}</option>
				<option value="publicKey">{i18n.t('auth.publicKey')}</option>
				<option value="keyboardInteractive">{i18n.t('auth.keyboardInteractive')}</option>
				<option value="agent">{i18n.t('auth.agent')}</option>
			</select>
		</label>

		{#if authType === 'publicKey'}
			<label>{i18n.t('site.keyPath')}<input bind:value={keyPath} placeholder="~/.ssh/id_ed25519" /></label>
			<label>
				{i18n.t('site.passphrase')} {#if editing}<span class="hint">{i18n.t('site.blankKeep')}</span>{/if}
				<input type="password" bind:value={password} />
			</label>
		{:else if authType === 'password' || authType === 'keyboardInteractive'}
			<label>
				{i18n.t('common.password')} {#if editing}<span class="hint">{i18n.t('site.blankKeep')}</span>{/if}
				<input type="password" bind:value={password} />
			</label>
		{/if}

		<div class="row">
			<label class="grow">
				{i18n.t('site.group')} <span class="hint">{i18n.t('site.groupHint')}</span>
				<input bind:value={group} list="site-groups" autocomplete="off" />
				<datalist id="site-groups">
					{#each groupOptions as g (g)}<option value={g}></option>{/each}
				</datalist>
			</label>
		</div>
		<label>
			{i18n.t('site.tags')} <span class="hint">{i18n.t('site.tagsHint')}</span>
			<input bind:value={tags} placeholder="prod, db" autocomplete="off" />
		</label>

		<div class="tunnels">
			<div class="tlabel">{i18n.t('site.tunnels')} <span class="hint">{i18n.t('site.tunnelsHint')}</span></div>
			{#each tunnels as t, i (i)}
				<div class="trow">
					<span class="tinfo">{tunnelLabel(t)}</span>
					<button type="button" class="tdel" onclick={() => removeTunnel(i)}>×</button>
				</div>
			{/each}
			<div class="tadd">
				<select bind:value={tKind}>
					<option value="local">{i18n.t('tunnel.local')}</option>
					<option value="dynamic">{i18n.t('tunnel.dynamic')}</option>
				</select>
				<input class="tport" type="number" min="1" max="65535" bind:value={tListen} title={i18n.t('tunnel.listenPort')} />
				{#if tKind === 'local'}
					<input class="thost" bind:value={tDestHost} placeholder={i18n.t('tunnel.destHost')} />
					<input class="tport" type="number" min="1" max="65535" bind:value={tDestPort} title={i18n.t('tunnel.destPort')} />
				{/if}
				<button type="button" class="tadd-btn" onclick={addTunnel}>＋</button>
			</div>
		</div>

		{#if errorMsg}<p class="error">{errorMsg}</p>{/if}

		<div class="actions">
			<button type="button" class="ghost" onclick={onclose}>{i18n.t('common.cancel')}</button>
			<button type="submit" disabled={saving}>{saving ? i18n.t('common.saving') : i18n.t('common.save')}</button>
		</div>
	</form>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.6);
		z-index: 10;
	}
	.dialog {
		display: flex;
		flex-direction: column;
		gap: 10px;
		width: 360px;
		padding: 22px;
		background: #252526;
		border: 1px solid #333;
		border-radius: 10px;
		color: #eee;
		font: 14px system-ui, sans-serif;
	}
	h2 {
		margin: 0 0 4px;
		font-size: 18px;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 12px;
		opacity: 0.9;
	}
	input,
	select {
		padding: 8px 10px;
		border: 1px solid #3c3c3c;
		border-radius: 6px;
		background: #1e1e1e;
		color: #eee;
		font: 14px system-ui, sans-serif;
	}
	.row {
		display: flex;
		gap: 10px;
	}
	.grow {
		flex: 1;
	}
	.port {
		width: 90px;
	}
	.hint {
		opacity: 0.6;
	}
	.tunnels {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 8px;
		border: 1px solid #333;
		border-radius: 6px;
	}
	.tlabel {
		font-size: 12px;
		opacity: 0.9;
	}
	.trow {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 6px;
		border-radius: 5px;
		background: #1e1e1e;
	}
	.tinfo {
		flex: 1;
		min-width: 0;
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.tdel {
		padding: 0 7px;
		background: transparent;
		border: none;
		color: #f48771;
		font-size: 15px;
		cursor: pointer;
	}
	.tadd {
		display: flex;
		gap: 4px;
		align-items: center;
	}
	.tadd select {
		flex: 0 0 auto;
		padding: 6px;
		font-size: 12px;
	}
	.tadd .thost {
		flex: 1;
		min-width: 0;
	}
	.tadd .tport {
		width: 64px;
	}
	.tadd-btn {
		flex: 0 0 auto;
		padding: 6px 10px;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 6px;
	}
	button {
		padding: 8px 14px;
		border: none;
		border-radius: 6px;
		background: #0e639c;
		color: #fff;
		font: 14px system-ui, sans-serif;
		cursor: pointer;
	}
	button:disabled {
		opacity: 0.6;
	}
	.ghost {
		background: transparent;
		border: 1px solid #555;
		color: #ddd;
	}
	.error {
		margin: 0;
		color: #f48771;
		font-size: 13px;
	}
</style>
