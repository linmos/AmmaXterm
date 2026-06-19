<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';

	interface Props {
		sessionId: string;
	}
	let { sessionId }: Props = $props();

	let timer: ReturnType<typeof setInterval> | undefined;
	// Speed tracking: id → { done, t } from the previous poll.
	let prev = new Map<string, { done: number; t: number }>();
	let speeds = $state<Record<string, number>>({});

	const mine = $derived(app.transfers.filter((t) => t.sessionId === sessionId));
	const hasFinished = $derived(mine.some((t) => t.status !== 'active'));

	function tick() {
		app.refreshTransfers().then(() => {
			const now = performance.now();
			const next: Record<string, number> = {};
			for (const t of app.transfers) {
				const p = prev.get(t.id);
				if (p && now > p.t && t.status === 'active') {
					next[t.id] = ((t.done - p.done) * 1000) / (now - p.t);
				}
				prev.set(t.id, { done: t.done, t: now });
			}
			speeds = next;
		});
	}

	function fmtBytes(n: number): string {
		if (n < 1024) return `${n} B`;
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
		return `${(n / 1024 / 1024).toFixed(1)} MB`;
	}
	function pct(t: { done: number; total: number }): number {
		return t.total > 0 ? Math.min(100, Math.round((t.done / t.total) * 100)) : 0;
	}

	function clearDone() {
		for (const t of mine) if (t.status !== 'active') app.clearTransfer(t.id);
	}

	onMount(() => {
		tick();
		timer = setInterval(tick, 800);
	});
	onDestroy(() => clearInterval(timer));
</script>

{#if mine.length}
	<div class="queue">
		<div class="qhead">
			<strong>{i18n.t('xfer.title')} ({mine.length})</strong>
			{#if hasFinished}
				<button class="clearall" onclick={clearDone}>{i18n.t('xfer.clearDone')}</button>
			{/if}
		</div>
		<ul>
			{#each mine as t (t.id)}
				<li>
					<div class="line">
						<span class="dir">{t.direction === 'upload' ? '⬆' : '⬇'}</span>
						<span class="nm" title={t.name}>{t.name}</span>
						{#if t.status === 'active'}
							<span class="sp">{speeds[t.id] ? `${fmtBytes(speeds[t.id])}/s` : ''}</span>
							<button class="x" title={i18n.t('xfer.cancel')} onclick={() => app.cancelTransfer(t.id)}>×</button>
						{:else if t.status === 'done'}
							<span class="tag ok">{i18n.t('xfer.done')}</span>
							<button class="x" title={i18n.t('xfer.clear')} onclick={() => app.clearTransfer(t.id)}>×</button>
						{:else}
							<span class="tag bad" title={t.error ?? ''}>{t.status === 'canceled' ? i18n.t('xfer.canceled') : i18n.t('xfer.error')}</span>
							<button class="rt" title={i18n.t('xfer.retry')} onclick={() => app.retryTransfer(t.id)}>↻</button>
							<button class="x" title={i18n.t('xfer.clear')} onclick={() => app.clearTransfer(t.id)}>×</button>
						{/if}
					</div>
					<div class="bar"><div class="fill" class:err={t.status === 'error' || t.status === 'canceled'} style="width:{pct(t)}%"></div></div>
				</li>
			{/each}
		</ul>
	</div>
{/if}

<style>
	.queue {
		border-top: 1px solid #333;
		background: #161616;
		max-height: 38%;
		overflow: auto;
		flex: none;
	}
	.qhead {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 5px 8px;
		font-size: 12px;
	}
	.clearall {
		padding: 2px 8px;
		border: 1px solid #444;
		border-radius: 4px;
		background: #2a2a2a;
		color: #ccc;
		font-size: 11px;
		cursor: pointer;
	}
	ul {
		margin: 0;
		padding: 0 6px 6px;
		list-style: none;
	}
	li {
		padding: 4px 2px;
	}
	.line {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
	}
	.dir {
		flex: none;
		opacity: 0.7;
	}
	.nm {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sp {
		flex: none;
		font-size: 11px;
		opacity: 0.6;
		font-variant-numeric: tabular-nums;
	}
	.tag {
		flex: none;
		font-size: 10px;
		padding: 0 6px;
		border-radius: 8px;
	}
	.tag.ok {
		background: #1b3a2a;
		color: #9fe0b8;
	}
	.tag.bad {
		background: #3a1b1b;
		color: #f0a59a;
	}
	.x,
	.rt {
		flex: none;
		padding: 0 6px;
		border: none;
		background: transparent;
		color: #ccc;
		font-size: 14px;
		cursor: pointer;
	}
	.x:hover,
	.rt:hover {
		color: #fff;
	}
	.bar {
		margin-top: 3px;
		height: 4px;
		border-radius: 2px;
		background: #2a2a2a;
		overflow: hidden;
	}
	.fill {
		height: 100%;
		background: #0e639c;
		transition: width 0.3s;
	}
	.fill.err {
		background: #a33;
	}
</style>
