<script lang="ts">
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';

	interface Props {
		onToggleLog: () => void;
		onShowTunnels: () => void;
	}
	let { onToggleLog, onShowTunnels }: Props = $props();

	const tab = $derived(app.activeTab);
	const status = $derived(tab?.status);
	const tunnelCount = $derived(app.tunnels.length);

	const statusLabel = $derived(
		status === 'connected'
			? i18n.t('status.connected')
			: status === 'connecting'
				? i18n.t('common.connecting')
				: status === 'error'
					? i18n.t('status.error')
					: status === 'closed'
						? i18n.t('tabs.closed')
						: i18n.t('status.noSession')
	);
</script>

<footer class="statusbar">
	<div class="side left">
		<span class="item static" class:live={status === 'connected'}>
			<span class="dot {status ?? 'none'}"></span>
			{#if tab}{tab.host}{:else}{statusLabel}{/if}
		</span>
		{#if tab}<span class="item static muted">{statusLabel}</span>{/if}
	</div>

	<div class="side right">
		{#if tunnelCount > 0}
			<button class="item" onclick={onShowTunnels} title={i18n.t('tunnels.title')}>
				⇆ {tunnelCount}
			</button>
		{/if}
		{#if tab?.sessionId}
			<button class="item" class:rec={tab.logging} onclick={onToggleLog} title={tab.logging ? i18n.t('tabs.stopLog') : i18n.t('tabs.startLog')}>
				{tab.logging ? '⏺ ' + i18n.t('tabs.logging') : '▤ ' + i18n.t('status.log')}
			</button>
		{/if}
		<span class="item static muted">UTF-8</span>
		<span class="item static muted">SSH</span>
	</div>
</footer>

<style>
	.statusbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		height: 22px;
		flex: none;
		padding: 0 4px;
		background: var(--vsc-statusbar-bg);
		color: var(--vsc-statusbar-fg);
		font: 12px var(--vsc-font);
		user-select: none;
	}
	.side {
		display: flex;
		align-items: center;
		height: 100%;
	}
	.item {
		display: flex;
		align-items: center;
		gap: 5px;
		height: 100%;
		padding: 0 8px;
		border: none;
		background: transparent;
		color: inherit;
		font: inherit;
		line-height: 22px;
		white-space: nowrap;
	}
	button.item {
		cursor: pointer;
	}
	button.item:hover {
		background: var(--vsc-statusbar-hover);
	}
	.item.muted {
		opacity: 0.85;
	}
	.item.rec {
		font-weight: 600;
	}
	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: rgba(255, 255, 255, 0.65);
	}
	.dot.connected {
		background: #4ade80;
	}
	.dot.connecting {
		background: #fde047;
	}
	.dot.error {
		background: #fca5a5;
	}
</style>
