<script lang="ts">
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';

	const prompt = $derived(app.hostKeyPrompt);
</script>

{#if prompt}
	<div
		class="backdrop"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) app.respondHostKey(false);
		}}
	>
		<div class="dialog" class:changed={prompt.changed}>
			<h2>{prompt.changed ? i18n.t('hostkey.changedTitle') : i18n.t('hostkey.unknownTitle')}</h2>
			<p class="who">{prompt.host}:{prompt.port}</p>

			{#if prompt.changed}
				<p class="warn">{i18n.t('hostkey.changedWarn')}</p>
			{:else}
				<p>{i18n.t('hostkey.firstTime')}</p>
			{/if}

			<p class="fp">{prompt.fingerprint}</p>

			<div class="actions">
				<button class="ghost" onclick={() => app.respondHostKey(false)}>{i18n.t('hostkey.reject')}</button>
				<button class:danger={prompt.changed} onclick={() => app.respondHostKey(true)}>
					{prompt.changed ? i18n.t('hostkey.trustAnyway') : i18n.t('hostkey.trust')}
				</button>
			</div>
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
		background: rgba(0, 0, 0, 0.6);
		z-index: 20;
	}
	.dialog {
		width: 400px;
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
	.dialog.changed {
		border-color: var(--vsc-red);
	}
	h2 {
		margin: 0 0 8px;
		font-size: 16px;
		font-weight: 600;
	}
	.who {
		margin: 0 0 10px;
		color: var(--vsc-sidebar-fg);
	}
	.warn {
		color: var(--vsc-red);
	}
	.fp {
		margin: 12px 0;
		padding: 8px 10px;
		border-radius: 4px;
		background: var(--vsc-input-bg);
		font-family: ui-monospace, monospace;
		font-size: 13px;
		word-break: break-all;
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
	.ghost {
		background: var(--vsc-button-secondary-bg);
		color: var(--vsc-button-secondary-fg);
	}
	.ghost:hover {
		background: var(--vsc-button-secondary-hover);
	}
	.danger {
		background: #c4341c;
	}
	.danger:hover {
		background: #d83c22;
	}
</style>
