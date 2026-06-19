<script lang="ts">
	import { untrack } from 'svelte';
	import { open } from '@tauri-apps/plugin-dialog';
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import type { AuthMethod, Site, SiteInput, SiteOverrides } from './types';
	import type { TunnelSpec } from '../tunnel/types';
	import { THEME_NAMES } from '$lib/settings.svelte';

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
		tunnels: (site?.tunnels ?? []).map((t) => ({ ...t })),
		proxyJump: [...(site?.proxyJump ?? [])],
		ov: site?.overrides ?? null
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

	// ProxyJump chain (TM-9): ordered ids of saved sites to hop through.
	let proxyJump = $state<string[]>(init.proxyJump);
	let jumpPick = $state('');

	// Per-site overrides (SM-6); blank string = inherit the global default.
	let ovTheme = $state(init.ov?.theme ?? '');
	let ovFontFamily = $state(init.ov?.fontFamily ?? '');
	let ovFontSize = $state(init.ov?.fontSize != null ? String(init.ov.fontSize) : '');
	let ovScrollback = $state(init.ov?.scrollback != null ? String(init.ov.scrollback) : '');
	let ovKeepalive = $state(init.ov?.keepaliveSecs != null ? String(init.ov.keepaliveSecs) : '');

	// Saved sites eligible as jump hosts: anything but the site being edited and
	// hosts already in the chain.
	const jumpCandidates = $derived(
		app.sites.filter((s) => s.id !== site?.id && !proxyJump.includes(s.id))
	);
	function siteName(id: string): string {
		return app.sites.find((s) => s.id === id)?.name ?? id;
	}
	function addJump() {
		if (jumpPick && !proxyJump.includes(jumpPick)) proxyJump = [...proxyJump, jumpPick];
		jumpPick = '';
	}
	function removeJump(i: number) {
		proxyJump = proxyJump.filter((_, idx) => idx !== i);
	}

	function buildOverrides(): SiteOverrides | null {
		const o: SiteOverrides = {};
		if (ovTheme) o.theme = ovTheme;
		if (ovFontFamily.trim()) o.fontFamily = ovFontFamily.trim();
		if (ovFontSize.trim()) o.fontSize = Number(ovFontSize);
		if (ovScrollback.trim()) o.scrollback = Number(ovScrollback);
		if (ovKeepalive.trim()) o.keepaliveSecs = Number(ovKeepalive);
		return Object.keys(o).length ? o : null;
	}

	async function browseKey() {
		const picked = await open({ multiple: false, directory: false, title: i18n.t('site.keyPath') });
		if (typeof picked === 'string') keyPath = picked;
	}

	// Inline "add tunnel" row.
	let tKind = $state('local');
	let tListen = $state(8080);
	let tDestHost = $state('');
	let tDestPort = $state(80);

	const tNeedsDest = $derived(tKind === 'local' || tKind === 'remote');

	function addTunnel() {
		if (!tListen) return;
		if (tNeedsDest && (!tDestHost || !tDestPort)) return;
		tunnels = [
			...tunnels,
			{
				kind: tKind,
				listenPort: Number(tListen),
				destHost: tNeedsDest ? tDestHost : '',
				destPort: tNeedsDest ? Number(tDestPort) : 0
			}
		];
		tDestHost = '';
	}
	function removeTunnel(i: number) {
		tunnels = tunnels.filter((_, idx) => idx !== i);
	}
	function tunnelLabel(t: TunnelSpec): string {
		if (t.kind === 'dynamic') return `D · SOCKS5 :${t.listenPort}`;
		if (t.kind === 'remote') return `R · :${t.listenPort} → ${t.destHost}:${t.destPort}`;
		return `L · :${t.listenPort} → ${t.destHost}:${t.destPort}`;
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
			tunnels,
			proxyJump,
			overrides: buildOverrides()
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
			<label>
				{i18n.t('site.keyPath')}
				<span class="keyrow">
					<input bind:value={keyPath} placeholder="~/.ssh/id_ed25519" />
					<button type="button" class="browse" onclick={browseKey}>{i18n.t('common.browse')}</button>
				</span>
			</label>
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
					<option value="remote">{i18n.t('tunnel.remote')}</option>
				</select>
				<input class="tport" type="number" min="1" max="65535" bind:value={tListen} title={i18n.t('tunnel.listenPort')} />
				{#if tNeedsDest}
					<input class="thost" bind:value={tDestHost} placeholder={i18n.t('tunnel.destHost')} />
					<input class="tport" type="number" min="1" max="65535" bind:value={tDestPort} title={i18n.t('tunnel.destPort')} />
				{/if}
				<button type="button" class="tadd-btn" onclick={addTunnel}>＋</button>
			</div>
		</div>

		<div class="tunnels">
			<div class="tlabel">{i18n.t('site.proxyJump')} <span class="hint">{i18n.t('site.proxyJumpHint')}</span></div>
			{#each proxyJump as id, i (id)}
				<div class="trow">
					<span class="tinfo">{i + 1}. {siteName(id)}</span>
					<button type="button" class="tdel" onclick={() => removeJump(i)}>×</button>
				</div>
			{/each}
			{#if jumpCandidates.length}
				<div class="tadd">
					<select class="grow" bind:value={jumpPick}>
						<option value="">{i18n.t('site.proxyJumpPick')}</option>
						{#each jumpCandidates as s (s.id)}<option value={s.id}>{s.name}</option>{/each}
					</select>
					<button type="button" class="tadd-btn" onclick={addJump}>＋</button>
				</div>
			{:else if !proxyJump.length}
				<div class="hint">{i18n.t('site.proxyJumpNone')}</div>
			{/if}
		</div>

		<details class="overrides">
			<summary>{i18n.t('site.overrides')} <span class="hint">{i18n.t('site.overridesHint')}</span></summary>
			<label>
				{i18n.t('settings.theme')}
				<select bind:value={ovTheme}>
					<option value="">{i18n.t('site.inherit')}</option>
					{#each THEME_NAMES as name (name)}<option value={name}>{i18n.t(`theme.${name}`)}</option>{/each}
				</select>
			</label>
			<label>
				{i18n.t('settings.fontFamily')}
				<input bind:value={ovFontFamily} placeholder={i18n.t('site.inherit')} autocomplete="off" />
			</label>
			<div class="row">
				<label class="grow">
					{i18n.t('settings.fontSize')}
					<input type="number" min="6" max="40" bind:value={ovFontSize} placeholder={i18n.t('site.inherit')} />
				</label>
				<label class="grow">
					{i18n.t('settings.scrollback')}
					<input type="number" min="0" bind:value={ovScrollback} placeholder={i18n.t('site.inherit')} />
				</label>
			</div>
			<label>
				{i18n.t('settings.keepalive')} <span class="hint">{i18n.t('settings.keepaliveHint')}</span>
				<input type="number" min="0" bind:value={ovKeepalive} placeholder={i18n.t('site.inherit')} />
			</label>
		</details>

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
		background: rgba(0, 0, 0, 0.55);
		z-index: 10;
	}
	.dialog {
		display: flex;
		flex-direction: column;
		gap: 11px;
		width: 420px;
		max-width: 92vw;
		max-height: 88vh;
		box-sizing: border-box;
		padding: 20px 22px;
		background: var(--vsc-widget-bg);
		border: 1px solid var(--vsc-widget-border);
		border-radius: 6px;
		color: var(--vsc-editor-fg);
		font: 13px var(--vsc-font);
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.44);
		overflow: hidden auto;
	}
	h2 {
		margin: 0 0 2px;
		font-size: 17px;
		font-weight: 600;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 5px;
		font-size: 12px;
		color: var(--vsc-sidebar-fg);
	}
	input,
	select {
		padding: 7px 9px;
		border: 1px solid var(--vsc-input-border);
		border-radius: 4px;
		background: var(--vsc-input-bg);
		color: var(--vsc-input-fg);
		font: 13px var(--vsc-font);
		box-sizing: border-box;
		min-width: 0;
	}
	input:focus,
	select:focus {
		outline: 1px solid var(--vsc-focus-border);
		outline-offset: -1px;
		border-color: var(--vsc-focus-border);
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
		color: var(--vsc-muted);
	}
	.keyrow {
		display: flex;
		gap: 6px;
	}
	.keyrow input {
		flex: 1;
		min-width: 0;
	}
	.browse {
		flex: none;
		padding: 7px 12px;
		border: none;
		border-radius: 3px;
		background: var(--vsc-button-secondary-bg);
		color: var(--vsc-button-secondary-fg);
		font: 13px var(--vsc-font);
		cursor: pointer;
	}
	.browse:hover {
		background: var(--vsc-button-secondary-hover);
	}
	.tunnels {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 8px;
		border: 1px solid var(--vsc-panel-border);
		border-radius: 4px;
	}
	.tlabel {
		font-size: 12px;
		color: var(--vsc-sidebar-fg);
	}
	.trow {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 4px 6px;
		border-radius: 4px;
		background: var(--vsc-input-bg);
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
		color: var(--vsc-red);
		font-size: 15px;
		cursor: pointer;
	}
	.tdel:hover {
		color: #ff6b6b;
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
	.tadd select.grow {
		flex: 1;
		min-width: 0;
	}
	.overrides {
		border: 1px solid var(--vsc-panel-border);
		border-radius: 4px;
		padding: 8px;
	}
	.overrides summary {
		font-size: 12px;
		color: var(--vsc-sidebar-fg);
		cursor: pointer;
	}
	.overrides label {
		margin-top: 8px;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 6px;
	}
	button {
		padding: 7px 14px;
		border: none;
		border-radius: 3px;
		background: var(--vsc-button-bg);
		color: var(--vsc-button-fg);
		font: 13px var(--vsc-font);
		cursor: pointer;
	}
	button:hover {
		background: var(--vsc-button-hover);
	}
	button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.ghost {
		background: var(--vsc-button-secondary-bg);
		color: var(--vsc-button-secondary-fg);
	}
	.ghost:hover {
		background: var(--vsc-button-secondary-hover);
	}
	.error {
		margin: 0;
		color: var(--vsc-red);
		font-size: 13px;
	}
</style>
