<script lang="ts">
	import { onMount } from 'svelte';
	import { save } from '@tauri-apps/plugin-dialog';
	import { app } from '$lib/state.svelte';
	import { settings } from '$lib/settings.svelte';
	import SiteSidebar from '$lib/sites/SiteSidebar.svelte';
	import TerminalTabs from '$lib/session/TerminalTabs.svelte';
	import SftpPanel from '$lib/sftp/SftpPanel.svelte';
	import TunnelPanel from '$lib/tunnel/TunnelPanel.svelte';
	import HostKeyDialog from '$lib/HostKeyDialog.svelte';
	import { i18n } from '$lib/i18n.svelte';

	type RightPanel = 'none' | 'files' | 'tunnels';
	let rightPanel = $state<RightPanel>('none');

	function toggle(panel: RightPanel) {
		rightPanel = rightPanel === panel ? 'none' : panel;
	}

	// Session logging toggle for the active tab (TM-12).
	async function toggleLog() {
		const tab = app.activeTab;
		if (!tab?.sessionId) return;
		if (tab.logging) {
			await app.stopLog(tab.key);
			return;
		}
		const path = await save({
			defaultPath: `${tab.host}-session.log`,
			filters: [{ name: 'Log', extensions: ['log', 'txt'] }]
		});
		if (typeof path === 'string') await app.startLog(tab.key, path);
	}

	onMount(() => {
		app.init();
		settings.load();
	});

	const activeSession = $derived(app.activeTab?.sessionId);
	const logging = $derived(app.activeTab?.logging ?? false);
</script>

<div class="app">
	<div class="left"><SiteSidebar /></div>

	<div class="center">
		<TerminalTabs />
		<div class="panel-toggles">
			<button
				class="panel-toggle"
				class:active={rightPanel === 'tunnels'}
				disabled={!activeSession}
				onclick={() => toggle('tunnels')}
			>
				{i18n.t('tunnels.toggle')}
			</button>
			<button
				class="panel-toggle"
				class:active={rightPanel === 'files'}
				disabled={!activeSession}
				onclick={() => toggle('files')}
			>
				{i18n.t('common.files')}
			</button>
			<button
				class="panel-toggle log"
				class:active={logging}
				disabled={!activeSession}
				title={logging ? i18n.t('tabs.stopLog') : i18n.t('tabs.startLog')}
				onclick={toggleLog}
			>
				{logging ? '⏺' : '▤'}
			</button>
		</div>
	</div>

	{#if rightPanel !== 'none' && activeSession}
		<div class="right">
			{#key activeSession}
				{#if rightPanel === 'files'}
					<SftpPanel sessionId={activeSession} />
				{:else if rightPanel === 'tunnels'}
					<TunnelPanel sessionId={activeSession} />
				{/if}
			{/key}
		</div>
	{/if}

	<HostKeyDialog />
</div>

<style>
	:global(html, body) {
		margin: 0;
		height: 100%;
	}
	.app {
		position: fixed;
		inset: 0;
		display: flex;
		background: #1e1e1e;
	}
	.left {
		flex: 0 0 250px;
		min-width: 0;
	}
	.center {
		position: relative;
		flex: 1 1 auto;
		min-width: 0;
	}
	.right {
		flex: 0 0 320px;
		min-width: 0;
		border-left: 1px solid #333;
	}
	.panel-toggles {
		position: absolute;
		top: 6px;
		right: 10px;
		display: flex;
		gap: 4px;
		z-index: 5;
	}
	.panel-toggle {
		padding: 4px 10px;
		border: 1px solid #555;
		border-radius: 6px;
		background: #252526;
		color: #ddd;
		font: 12px system-ui, sans-serif;
		cursor: pointer;
	}
	.panel-toggle.active {
		background: #0e639c;
		border-color: #0e639c;
		color: #fff;
	}
	.panel-toggle.log.active {
		background: #7a1f1f;
		border-color: #a33;
	}
	.panel-toggle:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
