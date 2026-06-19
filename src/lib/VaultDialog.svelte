<script lang="ts">
	import { onMount } from 'svelte';
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';

	interface Props {
		onclose: () => void;
	}
	let { onclose }: Props = $props();

	let exists = $state(false);
	let unlocked = $state(false);
	let master = $state('');
	let keys = $state<string[]>([]);
	let nKey = $state('');
	let nValue = $state('');
	let busy = $state(false);
	let errorMsg = $state<string | undefined>(undefined);

	async function refresh() {
		const s = await app.vaultStatus();
		exists = s.exists;
		unlocked = s.unlocked;
		keys = unlocked ? await app.vaultKeys() : [];
	}

	async function run(fn: () => Promise<void>) {
		busy = true;
		errorMsg = undefined;
		try {
			await fn();
			await refresh();
		} catch (err) {
			errorMsg = (err as { message?: string })?.message ?? String(err);
		} finally {
			busy = false;
		}
	}

	async function unlock(event: Event) {
		event.preventDefault();
		if (!master) return;
		await run(async () => {
			await app.vaultUnlock(master);
			master = '';
		});
	}
	function lock() {
		run(() => app.vaultLock());
	}
	function addSecret() {
		if (!nKey) return;
		run(async () => {
			await app.vaultSetSecret(nKey, nValue);
			nKey = '';
			nValue = '';
		});
	}

	onMount(() => {
		refresh().catch(() => {});
	});
</script>

<div
	class="backdrop"
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose();
	}}
>
	<div class="dialog">
		<h2>{i18n.t('vault.title')}</h2>
		<p class="note">{i18n.t('vault.note')}</p>

		{#if !unlocked}
			<form class="unlock" onsubmit={unlock}>
				<input type="password" placeholder={i18n.t('vault.master')} bind:value={master} />
				<button type="submit" disabled={busy || !master}>
					{exists ? i18n.t('vault.unlock') : i18n.t('vault.create')}
				</button>
			</form>
		{:else}
			<div class="status">
				<span class="tag ok">{i18n.t('vault.unlocked')}</span>
				<button class="ghost" onclick={lock} disabled={busy}>{i18n.t('vault.lock')}</button>
			</div>
			<ul class="list">
				{#each keys as k (k)}
					<li>
						<span class="k">{k}</span>
						<button class="x" onclick={() => run(() => app.vaultDeleteSecret(k))} disabled={busy}>×</button>
					</li>
				{/each}
				{#if !keys.length}<li class="empty">{i18n.t('vault.empty')}</li>{/if}
			</ul>
			<div class="add">
				<input class="grow" placeholder={i18n.t('vault.key')} bind:value={nKey} />
				<input class="grow" type="password" placeholder={i18n.t('vault.value')} bind:value={nValue} />
				<button onclick={addSecret} disabled={busy || !nKey}>{i18n.t('vault.add')}</button>
			</div>
		{/if}

		{#if errorMsg}<p class="error">{errorMsg}</p>{/if}

		<div class="actions">
			<button type="button" class="ghost" onclick={onclose}>{i18n.t('common.cancel')}</button>
		</div>
	</div>
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
		width: 420px;
		max-width: 92vw;
		max-height: 86vh;
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
	.note {
		margin: 0;
		font-size: 12px;
		color: var(--vsc-muted);
	}
	.unlock,
	.add {
		display: flex;
		gap: 6px;
	}
	.add .grow {
		flex: 1;
		min-width: 0;
	}
	input {
		flex: 1;
		padding: 7px 9px;
		border: 1px solid var(--vsc-input-border);
		border-radius: 4px;
		background: var(--vsc-input-bg);
		color: var(--vsc-input-fg);
		font: 13px var(--vsc-font);
	}
	input:focus {
		outline: 1px solid var(--vsc-focus-border);
		outline-offset: -1px;
		border-color: var(--vsc-focus-border);
	}
	.status {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.tag.ok {
		padding: 2px 9px;
		border-radius: 10px;
		background: rgba(63, 185, 80, 0.18);
		color: var(--vsc-green);
		font-size: 12px;
	}
	.list {
		flex: 1;
		min-height: 0;
		overflow: auto;
		margin: 0;
		padding: 0;
		list-style: none;
		border: 1px solid var(--vsc-panel-border);
		border-radius: 4px;
	}
	.list li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 5px 8px;
	}
	.list li:hover {
		background: var(--vsc-list-hover-bg);
	}
	.k {
		font-family: Consolas, monospace;
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.x {
		border: none;
		background: transparent;
		color: var(--vsc-red);
		font-size: 15px;
		cursor: pointer;
	}
	.x:hover {
		color: #ff6b6b;
		background: transparent;
	}
	.empty {
		opacity: 0.5;
		justify-content: center;
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
	.actions {
		display: flex;
		justify-content: flex-end;
	}
	.error {
		margin: 0;
		color: var(--vsc-red);
		font-size: 13px;
	}
</style>
