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
		background: rgba(0, 0, 0, 0.6);
		z-index: 10;
	}
	.dialog {
		display: flex;
		flex-direction: column;
		gap: 10px;
		width: 420px;
		max-height: 80vh;
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
	.note {
		margin: 0;
		font-size: 12px;
		opacity: 0.6;
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
		padding: 8px 10px;
		border: 1px solid #3c3c3c;
		border-radius: 6px;
		background: #1e1e1e;
		color: #eee;
		font: 14px system-ui, sans-serif;
	}
	.status {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.tag.ok {
		padding: 1px 8px;
		border-radius: 8px;
		background: #1b3a2a;
		color: #9fe0b8;
		font-size: 12px;
	}
	.list {
		flex: 1;
		min-height: 0;
		overflow: auto;
		margin: 0;
		padding: 0;
		list-style: none;
		border: 1px solid #333;
		border-radius: 6px;
	}
	.list li {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 5px 8px;
	}
	.list li:hover {
		background: #2a2a2a;
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
		color: #f48771;
		font-size: 15px;
		cursor: pointer;
	}
	.empty {
		opacity: 0.5;
		justify-content: center;
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
		opacity: 0.5;
	}
	.ghost {
		background: transparent;
		border: 1px solid #555;
		color: #ddd;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
	}
	.error {
		margin: 0;
		color: #f48771;
		font-size: 13px;
	}
</style>
