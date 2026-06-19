<script lang="ts">
	import { save } from '@tauri-apps/plugin-dialog';
	import { app, type GeneratedKey } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';

	interface Props {
		onclose: () => void;
	}
	let { onclose }: Props = $props();

	let algorithm = $state('ed25519');
	let comment = $state('');
	let generating = $state(false);
	let result = $state<GeneratedKey | null>(null);
	let errorMsg = $state<string | undefined>(undefined);
	let notice = $state<string | undefined>(undefined);

	async function generate(event: Event) {
		event.preventDefault();
		generating = true;
		errorMsg = undefined;
		notice = undefined;
		try {
			result = await app.generateKey(algorithm, comment);
		} catch (err) {
			errorMsg = (err as { message?: string })?.message ?? String(err);
		} finally {
			generating = false;
		}
	}

	async function copyPublic() {
		if (!result) return;
		await navigator.clipboard.writeText(result.publicKey);
		notice = i18n.t('keygen.copied');
	}

	async function saveKey() {
		if (!result) return;
		const path = await save({ defaultPath: algorithm === 'rsa' ? 'id_rsa' : 'id_ed25519' });
		if (typeof path !== 'string') return;
		try {
			await app.saveKey(path, result.privateKey, result.publicKey);
			notice = i18n.t('keygen.saved');
		} catch (err) {
			errorMsg = (err as { message?: string })?.message ?? String(err);
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
	<form class="dialog" onsubmit={generate}>
		<h2>{i18n.t('keygen.title')}</h2>

		<label>
			{i18n.t('keygen.algorithm')}
			<select bind:value={algorithm}>
				<option value="ed25519">{i18n.t('keygen.ed25519')}</option>
				<option value="rsa">{i18n.t('keygen.rsa')}</option>
			</select>
		</label>
		<label>{i18n.t('keygen.comment')}<input bind:value={comment} placeholder="user@host" /></label>
		{#if algorithm === 'rsa'}<p class="hint">{i18n.t('keygen.rsaSlow')}</p>{/if}

		{#if result}
			<div class="out">
				<div class="lbl">{i18n.t('keygen.publicKey')}</div>
				<textarea readonly rows="3">{result.publicKey}</textarea>
				<div class="fp">{i18n.t('keygen.fingerprint')}: <code>{result.fingerprint}</code></div>
				<div class="rowbtns">
					<button type="button" onclick={copyPublic}>{i18n.t('keygen.copy')}</button>
					<button type="button" onclick={saveKey}>{i18n.t('keygen.save')}</button>
				</div>
			</div>
		{/if}

		{#if notice}<p class="notice">{notice}</p>{/if}
		{#if errorMsg}<p class="error">{errorMsg}</p>{/if}

		<div class="actions">
			<button type="button" class="ghost" onclick={onclose}>{i18n.t('common.cancel')}</button>
			<button type="submit" disabled={generating}>
				{generating ? i18n.t('keygen.generating') : i18n.t('keygen.generate')}
			</button>
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
		width: 440px;
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
	select,
	textarea {
		padding: 8px 10px;
		border: 1px solid #3c3c3c;
		border-radius: 6px;
		background: #1e1e1e;
		color: #eee;
		font: 14px system-ui, sans-serif;
	}
	textarea {
		resize: vertical;
		font-family: Consolas, monospace;
		font-size: 12px;
		word-break: break-all;
	}
	.hint {
		margin: 0;
		font-size: 12px;
		opacity: 0.6;
	}
	.out {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px;
		border: 1px solid #333;
		border-radius: 6px;
		background: #1b1b1b;
	}
	.out .lbl {
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.4px;
		opacity: 0.6;
	}
	.fp {
		font-size: 11px;
		opacity: 0.8;
		word-break: break-all;
	}
	.rowbtns {
		display: flex;
		gap: 8px;
	}
	.rowbtns button {
		flex: 1;
		padding: 7px;
		border: 1px solid #555;
		border-radius: 6px;
		background: #2a2a2a;
		color: #eee;
		cursor: pointer;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 6px;
	}
	.actions button {
		padding: 8px 14px;
		border: none;
		border-radius: 6px;
		background: #0e639c;
		color: #fff;
		font: 14px system-ui, sans-serif;
		cursor: pointer;
	}
	.actions button:disabled {
		opacity: 0.6;
	}
	.actions .ghost {
		background: transparent;
		border: 1px solid #555;
		color: #ddd;
	}
	.notice {
		margin: 0;
		color: #9fe0b8;
		font-size: 13px;
	}
	.error {
		margin: 0;
		color: #f48771;
		font-size: 13px;
	}
</style>
