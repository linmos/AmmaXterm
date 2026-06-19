<script lang="ts">
	import { untrack } from 'svelte';
	import { i18n } from '$lib/i18n.svelte';
	import { settings, THEME_NAMES, type Settings } from '$lib/settings.svelte';

	interface Props {
		onclose: () => void;
	}
	let { onclose }: Props = $props();

	// Edit a working copy; commit on Save.
	const init = untrack(() => ({ ...settings.s }));
	let theme = $state(init.theme);
	let fontFamily = $state(init.fontFamily);
	let fontSize = $state(init.fontSize);
	let scrollback = $state(init.scrollback);
	let keepaliveSecs = $state(init.keepaliveSecs);
	let autoReconnect = $state(init.autoReconnect);
	let saving = $state(false);
	let errorMsg = $state<string | undefined>(undefined);

	async function save(event: Event) {
		event.preventDefault();
		saving = true;
		errorMsg = undefined;
		const next: Settings = {
			schemaVersion: init.schemaVersion,
			theme,
			fontFamily,
			fontSize: Number(fontSize),
			scrollback: Number(scrollback),
			keepaliveSecs: Number(keepaliveSecs),
			autoReconnect
		};
		try {
			await settings.save(next);
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
		<h2>{i18n.t('settings.title')}</h2>

		<h3>{i18n.t('settings.appearance')}</h3>
		<label>
			{i18n.t('settings.theme')}
			<select bind:value={theme}>
				{#each THEME_NAMES as name (name)}
					<option value={name}>{i18n.t(`theme.${name}`)}</option>
				{/each}
			</select>
		</label>
		<label>{i18n.t('settings.fontFamily')}<input bind:value={fontFamily} /></label>
		<div class="row">
			<label class="grow">{i18n.t('settings.fontSize')}<input type="number" min="8" max="40" bind:value={fontSize} /></label>
			<label class="grow">{i18n.t('settings.scrollback')}<input type="number" min="0" max="100000" step="500" bind:value={scrollback} /></label>
		</div>

		<h3>{i18n.t('settings.connection')}</h3>
		<label>
			{i18n.t('settings.keepalive')} <span class="hint">{i18n.t('settings.keepaliveHint')}</span>
			<input type="number" min="0" max="3600" bind:value={keepaliveSecs} />
		</label>
		<label class="check">
			<input type="checkbox" bind:checked={autoReconnect} />
			{i18n.t('settings.autoReconnect')}
		</label>

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
		width: 380px;
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
	h3 {
		margin: 6px 0 0;
		font-size: 12px;
		text-transform: uppercase;
		letter-spacing: 0.4px;
		opacity: 0.6;
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
	.check {
		flex-direction: row;
		align-items: center;
		gap: 8px;
	}
	.check input {
		width: auto;
	}
	.grow {
		flex: 1;
	}
	.hint {
		opacity: 0.6;
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
