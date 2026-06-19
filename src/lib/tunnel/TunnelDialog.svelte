<script lang="ts">
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import type { TunnelSpec } from './types';

	interface Props {
		sessionId: string;
		onclose: () => void;
	}
	let { sessionId, onclose }: Props = $props();

	// Local (-L) and dynamic SOCKS5 (-D) forwarding; remote (-R) arrives later.
	let kind = $state('local');
	let listenPort = $state(8080);
	let destHost = $state('');
	let destPort = $state(80);
	let saving = $state(false);
	let errorMsg = $state<string | undefined>(undefined);

	async function save(event: Event) {
		event.preventDefault();
		saving = true;
		errorMsg = undefined;
		const spec: TunnelSpec = {
			kind,
			listenPort: Number(listenPort),
			destHost: kind === 'local' ? destHost : '',
			destPort: kind === 'local' ? Number(destPort) : 0
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
			</select>
		</label>

		<label>{i18n.t('tunnel.listenPort')}<input type="number" min="1" max="65535" bind:value={listenPort} required /></label>
		{#if kind === 'local'}
			<div class="row">
				<label class="grow">{i18n.t('tunnel.destHost')}<input bind:value={destHost} placeholder="localhost" required /></label>
				<label class="port">{i18n.t('tunnel.destPort')}<input type="number" min="1" max="65535" bind:value={destPort} required /></label>
			</div>
		{/if}

		<p class="pf7">🔒 {i18n.t('tunnel.pf7')}</p>

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
		margin: 0;
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
		width: 110px;
	}
	.pf7 {
		margin: 0;
		font-size: 12px;
		color: #9fe0b8;
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
