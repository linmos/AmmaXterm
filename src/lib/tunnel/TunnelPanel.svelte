<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import TunnelDialog from './TunnelDialog.svelte';

	interface Props {
		sessionId: string;
	}
	let { sessionId }: Props = $props();

	let showDialog = $state(false);
	let timer: ReturnType<typeof setInterval> | undefined;

	const mine = $derived(app.tunnels.filter((t) => t.sessionId === sessionId));

	function fmtBytes(n: number): string {
		if (n < 1024) return `${n} B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
		return `${(n / 1024 / 1024).toFixed(1)} MB`;
	}

	onMount(() => {
		app.refreshTunnels();
		timer = setInterval(() => app.refreshTunnels(), 2000);
	});
	onDestroy(() => clearInterval(timer));
</script>

<div class="panel">
	<div class="head">
		<strong>{i18n.t('tunnels.title')}</strong>
		<button class="add" onclick={() => (showDialog = true)}>＋ {i18n.t('tunnels.add')}</button>
	</div>

	<ul class="list">
		{#each mine as t (t.id)}
			<li>
				<div class="info">
					<div class="route">
						<span class="badge">{t.kind === 'dynamic' ? 'D' : t.kind === 'remote' ? 'R' : 'L'}</span>
						{#if t.kind === 'dynamic'}
							SOCKS5 {t.listenHost}:{t.listenPort}
						{:else}
							{t.listenHost}:{t.listenPort} → {t.destHost}:{t.destPort}
						{/if}
					</div>
					<div class="stats">
						{t.conns}
						{i18n.t('tunnel.conns')} · ↑{fmtBytes(t.bytesUp)} ↓{fmtBytes(t.bytesDown)}
					</div>
				</div>
				<button class="close" title={i18n.t('common.close')} onclick={() => app.closeTunnel(t.id)}>×</button>
			</li>
		{/each}
		{#if !mine.length}
			<li class="empty">{i18n.t('tunnels.none')}</li>
		{/if}
	</ul>
</div>

{#if showDialog}
	<TunnelDialog {sessionId} onclose={() => (showDialog = false)} />
{/if}

<style>
	.panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		background: #1e1e1e;
		color: #ddd;
		font: 13px system-ui, sans-serif;
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 10px;
		border-bottom: 1px solid #333;
	}
	.add {
		padding: 5px 10px;
		border: none;
		border-radius: 6px;
		background: #0e639c;
		color: #fff;
		font: 12px system-ui, sans-serif;
		cursor: pointer;
	}
	.list {
		flex: 1;
		min-height: 0;
		overflow: auto;
		margin: 0;
		padding: 6px;
		list-style: none;
	}
	.list li {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px;
		border-radius: 6px;
	}
	.list li:hover {
		background: #2a2a2a;
	}
	.info {
		flex: 1;
		min-width: 0;
	}
	.route {
		display: flex;
		align-items: center;
		gap: 6px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.badge {
		padding: 0 6px;
		border-radius: 4px;
		background: #0e639c;
		color: #fff;
		font-size: 10px;
		font-weight: 700;
		line-height: 16px;
	}
	.stats {
		margin-top: 3px;
		font-size: 11px;
		opacity: 0.6;
	}
	.close {
		padding: 4px 8px;
		border: none;
		border-radius: 6px;
		background: transparent;
		color: #ddd;
		font-size: 15px;
		cursor: pointer;
		opacity: 0.6;
	}
	.close:hover {
		opacity: 1;
		background: #3a3a3a;
	}
	.empty {
		opacity: 0.5;
		justify-content: center;
	}
</style>
