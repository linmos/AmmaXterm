<script lang="ts">
	import { untrack } from 'svelte';
	import { app } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import type { ImportedSite } from './types';

	interface Props {
		entries: ImportedSite[];
		source: string;
		onclose: () => void;
	}
	let { entries, source, onclose }: Props = $props();

	const existingNames = untrack(() => new Set(app.sites.map((s) => s.name)));

	// Selection state, keyed by index; entries that already exist start unchecked.
	let selected = $state<boolean[]>(untrack(() => entries.map((e) => !existingNames.has(e.name))));
	let importing = $state(false);
	let errorMsg = $state<string | undefined>(undefined);

	const chosenCount = $derived(selected.filter(Boolean).length);
	const allOn = $derived(entries.length > 0 && selected.every(Boolean));

	function toggleAll() {
		const next = !allOn;
		selected = entries.map(() => next);
	}

	async function doImport() {
		importing = true;
		errorMsg = undefined;
		try {
			const picked = entries.filter((_, i) => selected[i]);
			await app.addImported(picked);
			onclose();
		} catch (err) {
			errorMsg = (err as { message?: string })?.message ?? String(err);
		} finally {
			importing = false;
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
	<div class="dialog">
		<h2>{i18n.t('import.title')}</h2>
		<p class="src">{i18n.t('import.from')}: <code>{source}</code></p>

		{#if !entries.length}
			<p class="empty">{i18n.t('import.empty')}</p>
		{:else}
			<label class="all">
				<input type="checkbox" checked={allOn} onchange={toggleAll} />
				{i18n.t('import.selectAll')}
			</label>
			<ul class="list">
				{#each entries as e, i (i)}
					<li>
						<label>
							<input type="checkbox" bind:checked={selected[i]} />
							<span class="name">{e.name}</span>
							<span class="addr">{e.username ? `${e.username}@` : ''}{e.host}:{e.port}</span>
							{#if existingNames.has(e.name)}<span class="badge">{i18n.t('import.exists')}</span>{/if}
						</label>
					</li>
				{/each}
			</ul>
		{/if}

		{#if errorMsg}<p class="error">{errorMsg}</p>{/if}

		<div class="actions">
			<button type="button" class="ghost" onclick={onclose}>{i18n.t('common.cancel')}</button>
			<button type="button" disabled={importing || !chosenCount} onclick={doImport}>
				{importing ? i18n.t('common.saving') : i18n.t('import.count').replace('{n}', String(chosenCount))}
			</button>
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
		background: rgba(0, 0, 0, 0.6);
		z-index: 10;
	}
	.dialog {
		display: flex;
		flex-direction: column;
		gap: 10px;
		width: 420px;
		max-height: 80vh;
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
	.src {
		margin: 0;
		font-size: 12px;
		opacity: 0.7;
	}
	.src code {
		font-size: 11px;
	}
	.all {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		opacity: 0.85;
		padding-bottom: 4px;
		border-bottom: 1px solid #333;
	}
	.list {
		flex: 1;
		min-height: 0;
		overflow: auto;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.list li label {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 5px 4px;
		cursor: pointer;
	}
	.list li:hover {
		background: #2a2a2a;
	}
	.list .name {
		font-weight: 600;
	}
	.list .addr {
		flex: 1;
		min-width: 0;
		font-size: 11px;
		opacity: 0.6;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.badge {
		padding: 0 6px;
		border-radius: 8px;
		background: #5a4a1a;
		color: #e8c97a;
		font-size: 10px;
		line-height: 16px;
	}
	.empty {
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
		opacity: 0.5;
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
