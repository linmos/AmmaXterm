<script lang="ts">
	import { updater } from '$lib/updater.svelte';
	import { i18n } from '$lib/i18n.svelte';

	// Only the "actionable" phases warrant a modal. `checking` / `uptodate` are
	// shown inline by AboutDialog so the silent startup check never pops up.
	const open = $derived(
		updater.phase === 'available' ||
			updater.phase === 'downloading' ||
			updater.phase === 'ready' ||
			updater.phase === 'error'
	);

	function backdropClose() {
		// dismiss() itself refuses to interrupt an in-flight download.
		updater.dismiss();
	}
</script>

{#if open}
	<div
		class="backdrop"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) backdropClose();
		}}
	>
		<div class="dialog">
			{#if updater.phase === 'available'}
				<h2>{i18n.t('update.available').replace('{v}', updater.version)}</h2>
				{#if updater.notes}
					<div class="notes-label">{i18n.t('update.notes')}</div>
					<pre class="notes">{updater.notes}</pre>
				{/if}
				<div class="actions">
					<button type="button" class="ghost" onclick={() => updater.dismiss()}>
						{i18n.t('update.later')}
					</button>
					<button type="button" onclick={() => updater.downloadAndInstall()}>
						{i18n.t('update.install')}
					</button>
				</div>
			{:else if updater.phase === 'downloading'}
				<h2>{i18n.t('update.downloading')}</h2>
				<div class="bar"><div class="fill" style="width: {updater.progress}%"></div></div>
				<div class="pct">{updater.progress}%</div>
			{:else if updater.phase === 'ready'}
				<h2>{i18n.t('update.ready')}</h2>
				<div class="actions">
					<button type="button" class="ghost" onclick={() => updater.dismiss()}>
						{i18n.t('update.later')}
					</button>
					<button type="button" onclick={() => updater.relaunchApp()}>
						{i18n.t('update.relaunch')}
					</button>
				</div>
			{:else if updater.phase === 'error'}
				<h2>{i18n.t('update.error')}</h2>
				<pre class="notes err">{updater.error}</pre>
				<div class="actions">
					<button type="button" class="ghost" onclick={() => updater.dismiss()}>
						{i18n.t('common.close')}
					</button>
					<button type="button" onclick={() => updater.checkForUpdates(false)}>
						{i18n.t('update.retry')}
					</button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		background: rgba(0, 0, 0, 0.55);
		z-index: 20;
	}
	.dialog {
		display: flex;
		flex-direction: column;
		gap: 14px;
		width: 420px;
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
	h2 {
		margin: 0;
		font-size: 16px;
		font-weight: 600;
	}
	.notes-label {
		color: var(--vsc-muted);
		font-size: 12px;
	}
	.notes {
		margin: 0;
		max-height: 220px;
		overflow: auto;
		padding: 10px 12px;
		background: var(--vsc-editor-bg, rgba(0, 0, 0, 0.18));
		border: 1px solid var(--vsc-widget-border);
		border-radius: 4px;
		font: 12px var(--vsc-font);
		white-space: pre-wrap;
		word-break: break-word;
	}
	.err {
		color: var(--vsc-error-fg, #f48771);
	}
	.bar {
		height: 8px;
		border-radius: 4px;
		background: var(--vsc-widget-border);
		overflow: hidden;
	}
	.fill {
		height: 100%;
		background: var(--vsc-focus-border, var(--vsc-button-bg));
		transition: width 0.15s ease;
	}
	.pct {
		font-size: 12px;
		color: var(--vsc-muted);
		text-align: right;
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 8px;
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
	button.ghost {
		background: transparent;
		color: var(--vsc-editor-fg);
		border: 1px solid var(--vsc-widget-border);
	}
	button.ghost:hover {
		background: var(--vsc-list-hover, rgba(255, 255, 255, 0.06));
	}
</style>
