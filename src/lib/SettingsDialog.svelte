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
	let copyOnSelect = $state(init.copyOnSelect);
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
			autoReconnect,
			copyOnSelect
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
			{i18n.t('settings.language')}
			<select value={i18n.locale} onchange={(e) => i18n.setLocale(e.currentTarget.value)}>
				<option value="zh-TW">{i18n.t('lang.zhTW')}</option>
				<option value="en">{i18n.t('lang.en')}</option>
			</select>
		</label>
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
		<label class="check">
			<input type="checkbox" bind:checked={copyOnSelect} />
			{i18n.t('settings.copyOnSelect')}
		</label>

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
		background: rgba(0, 0, 0, 0.55);
		z-index: 10;
	}
	.dialog {
		display: flex;
		flex-direction: column;
		gap: 12px;
		width: 400px;
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
		overflow: hidden auto;
	}
	h2 {
		margin: 0;
		font-size: 17px;
		font-weight: 600;
	}
	h3 {
		margin: 8px 0 0;
		font-size: 11px;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		color: var(--vsc-muted);
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
		color: var(--vsc-muted);
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
