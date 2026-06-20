<script lang="ts">
	import { getVersion, getTauriVersion } from '@tauri-apps/api/app';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { i18n } from '$lib/i18n.svelte';

	interface Props {
		onclose: () => void;
	}
	let { onclose }: Props = $props();

	const REPO = 'https://github.com/linmos/AmmaXterm';

	let version = $state('');
	let tauriVersion = $state('');

	// App/Tauri versions are async; fetch once on mount.
	$effect(() => {
		getVersion().then((v) => (version = v));
		getTauriVersion().then((v) => (tauriVersion = v));
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
		<div class="brand">
			<img class="logo" src="/favicon.png" alt="" />
			<div>
				<h2>AmmaXterm</h2>
				<p class="tagline">{i18n.t('about.tagline')}</p>
			</div>
		</div>

		<dl class="facts">
			<dt>{i18n.t('about.version')}</dt>
			<dd>{version || '…'}</dd>
			<dt>Tauri</dt>
			<dd>{tauriVersion || '…'}</dd>
			<dt>{i18n.t('about.license')}</dt>
			<dd>MIT</dd>
			<dt>{i18n.t('about.repo')}</dt>
			<dd><button type="button" class="link" onclick={() => openUrl(REPO)}>{REPO}</button></dd>
		</dl>

		<div class="actions">
			<button type="button" onclick={onclose}>{i18n.t('common.close')}</button>
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
		gap: 16px;
		width: 380px;
		max-width: 92vw;
		box-sizing: border-box;
		padding: 22px;
		background: var(--vsc-widget-bg);
		border: 1px solid var(--vsc-widget-border);
		border-radius: 6px;
		color: var(--vsc-editor-fg);
		font: 13px var(--vsc-font);
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.44);
	}
	.brand {
		display: flex;
		align-items: center;
		gap: 14px;
	}
	.logo {
		width: 44px;
		height: 44px;
		flex: none;
	}
	h2 {
		margin: 0;
		font-size: 18px;
		font-weight: 600;
	}
	.tagline {
		margin: 3px 0 0;
		font-size: 12px;
		color: var(--vsc-muted);
	}
	.facts {
		display: grid;
		grid-template-columns: auto 1fr;
		gap: 6px 14px;
		margin: 0;
		padding-top: 4px;
		border-top: 1px solid var(--vsc-widget-border);
	}
	dt {
		color: var(--vsc-muted);
		font-size: 12px;
	}
	dd {
		margin: 0;
		font-size: 12px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.link {
		padding: 0;
		border: none;
		background: transparent;
		color: var(--vsc-focus-border);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.link:hover {
		text-decoration: underline;
		background: transparent;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
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
</style>
