<script lang="ts">
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import type { TunnelSpec } from './types';

	interface Props {
		sessionId: string;
		onclose: () => void;
	}
	let { sessionId, onclose }: Props = $props();

	// Local (-L), dynamic SOCKS5 (-D), and remote (-R) forwarding.
	let kind = $state('local');
	let listenPort = $state(8080);
	let destHost = $state('');
	let destPort = $state(80);
	let expose = $state(false);
	let saving = $state(false);
	let errorMsg = $state<string | undefined>(undefined);

	const needsDest = $derived(kind === 'local' || kind === 'remote');

	async function save(event: Event) {
		event.preventDefault();
		saving = true;
		errorMsg = undefined;
		const spec: TunnelSpec = {
			kind,
			listenPort: Number(listenPort),
			destHost: needsDest ? destHost : '',
			destPort: needsDest ? Number(destPort) : 0,
			expose: kind === 'remote' ? expose : false
		};
		try {
			await app.openTunnel(sessionId, spec);
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
		<h2>{i18n.t('tunnels.add')}</h2>

		<label>
			{i18n.t('tunnel.kind')}
			<select bind:value={kind}>
				<option value="local">{i18n.t('tunnel.local')}</option>
				<option value="dynamic">{i18n.t('tunnel.dynamic')}</option>
				<option value="remote">{i18n.t('tunnel.remote')}</option>
			</select>
		</label>

		<label>
			{kind === 'remote' ? i18n.t('tunnel.remoteBind') : i18n.t('tunnel.listenPort')}
			<input type="number" min="1" max="65535" bind:value={listenPort} required />
		</label>
		{#if needsDest}
			<div class="row">
				<label class="grow">{i18n.t('tunnel.destHost')}<input bind:value={destHost} placeholder="localhost" required /></label>
				<label class="port">{i18n.t('tunnel.destPort')}<input type="number" min="1" max="65535" bind:value={destPort} required /></label>
			</div>
		{/if}

		{#if kind === 'remote'}
			<p class="pf7">{i18n.t('tunnel.remoteHint')}</p>
			<label class="check">
				<input type="checkbox" bind:checked={expose} />
				{i18n.t('tunnel.expose')}
			</label>
			{#if expose}<p class="warn">{i18n.t('tunnel.exposeWarn')}</p>{/if}
		{:else}
			<p class="pf7">🔒 {i18n.t('tunnel.pf7')}</p>
		{/if}

		{#if errorMsg}<p class="error">{errorMsg}</p>{/if}

		<div class="actions">
			<button type="button" class="ghost" onclick={onclose}>{i18n.t('common.cancel')}</button>
			<button type="submit" disabled={saving}>{saving ? i18n.t('common.saving') : i18n.t('common.add')}</button>
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
		gap: 12px;
		width: 380px;
		max-width: 92vw;
		box-sizing: border-box;
		padding: 20px 22px;
		background: var(--vsc-widget-bg);
		border: 1px solid var(--vsc-widget-border);
		border-radius: 6px;
		color: var(--vsc-editor-fg);
		font: 13px var(--vsc-font);
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.44);
	}
	h2 {
		margin: 0;
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
		width: 110px;
	}
	.pf7 {
		margin: 0;
		font-size: 12px;
		color: var(--vsc-green);
	}
	.check {
		flex-direction: row;
		align-items: center;
		gap: 8px;
	}
	.check input {
		width: auto;
	}
	.warn {
		margin: 0;
		font-size: 12px;
		color: var(--vsc-yellow);
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
