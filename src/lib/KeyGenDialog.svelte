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
		background: rgba(0, 0, 0, 0.55);
		z-index: 10;
	}
	.dialog {
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: 440px;
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
		overflow: auto;
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
	select,
	textarea {
		padding: 7px 9px;
		border: 1px solid var(--vsc-input-border);
		border-radius: 4px;
		background: var(--vsc-input-bg);
		color: var(--vsc-input-fg);
		font: 13px var(--vsc-font);
	}
	input:focus,
	select:focus,
	textarea:focus {
		outline: 1px solid var(--vsc-focus-border);
		outline-offset: -1px;
		border-color: var(--vsc-focus-border);
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
		color: var(--vsc-muted);
	}
	.out {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 10px;
		border: 1px solid var(--vsc-panel-border);
		border-radius: 4px;
		background: rgba(0, 0, 0, 0.18);
	}
	.out .lbl {
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--vsc-muted);
	}
	.fp {
		font-size: 11px;
		color: var(--vsc-sidebar-fg);
		word-break: break-all;
	}
	.rowbtns {
		display: flex;
		gap: 8px;
	}
	.rowbtns button {
		flex: 1;
		padding: 7px;
		border: none;
		border-radius: 3px;
		background: var(--vsc-button-secondary-bg);
		color: var(--vsc-button-secondary-fg);
		font: 13px var(--vsc-font);
		cursor: pointer;
	}
	.rowbtns button:hover {
		background: var(--vsc-button-secondary-hover);
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
		margin-top: 6px;
	}
	.actions button {
		padding: 7px 14px;
		border: none;
		border-radius: 3px;
		background: var(--vsc-button-bg);
		color: var(--vsc-button-fg);
		font: 13px var(--vsc-font);
		cursor: pointer;
	}
	.actions button:hover {
		background: var(--vsc-button-hover);
	}
	.actions button:disabled {
		opacity: 0.5;
		cursor: default;
	}
	.actions .ghost {
		background: var(--vsc-button-secondary-bg);
		color: var(--vsc-button-secondary-fg);
	}
	.actions .ghost:hover {
		background: var(--vsc-button-secondary-hover);
	}
	.notice {
		margin: 0;
		color: var(--vsc-green);
		font-size: 13px;
	}
	.error {
		margin: 0;
		color: var(--vsc-red);
		font-size: 13px;
	}
</style>
