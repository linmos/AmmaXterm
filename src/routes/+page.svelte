<script lang="ts">
	import { onMount } from 'svelte';
	import { app } from '$lib/state.svelte';
	import SiteSidebar from '$lib/sites/SiteSidebar.svelte';
	import TerminalTabs from '$lib/session/TerminalTabs.svelte';
	import SftpPanel from '$lib/sftp/SftpPanel.svelte';
	import HostKeyDialog from '$lib/HostKeyDialog.svelte';
	import { i18n } from '$lib/i18n.svelte';

	let showFiles = $state(false);

	onMount(() => {
		app.init();
	});

	const activeSession = $derived(app.activeTab?.sessionId);
</script>

<div class="app">
	<div class="left"><SiteSidebar /></div>

	<div class="center">
		<TerminalTabs />
		<button
			class="files-toggle"
			class:active={showFiles}
			disabled={!activeSession}
			onclick={() => (showFiles = !showFiles)}
		>
			{i18n.t('common.files')}
		</button>
	</div>

	{#if showFiles && activeSession}
		<div class="right">
			{#key activeSession}
				<SftpPanel sessionId={activeSession} />
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
	.files-toggle {
		position: absolute;
		top: 6px;
		right: 10px;
		padding: 4px 10px;
		border: 1px solid #555;
		border-radius: 6px;
		background: #252526;
		color: #ddd;
		font: 12px system-ui, sans-serif;
		cursor: pointer;
		z-index: 5;
	}
	.files-toggle.active {
		background: #0e639c;
		border-color: #0e639c;
		color: #fff;
	}
	.files-toggle:disabled {
		opacity: 0.4;
		cursor: default;
	}
</style>
