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
		background: rgba(0, 0, 0, 0.65);
		z-index: 20;
	}
	.dialog {
		width: 380px;
		padding: 22px;
		background: #252526;
		border: 1px solid #333;
		border-radius: 10px;
		color: #eee;
		font: 14px system-ui, sans-serif;
	}
	.dialog.changed {
		border-color: #f48771;
	}
	h2 {
		margin: 0 0 8px;
		font-size: 17px;
	}
	.who {
		margin: 0 0 10px;
		opacity: 0.85;
	}
	.warn {
		color: #f48771;
	}
	.fp {
		margin: 12px 0;
		padding: 8px 10px;
		border-radius: 6px;
		background: #1e1e1e;
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
		padding: 8px 14px;
		border: none;
		border-radius: 6px;
		background: #0e639c;
		color: #fff;
		font: 14px system-ui, sans-serif;
		cursor: pointer;
	}
	.ghost {
		background: transparent;
		border: 1px solid #555;
		color: #ddd;
	}
	.danger {
		background: #a1260d;
	}
</style>
