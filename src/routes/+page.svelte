<script lang="ts">
	import { onMount } from 'svelte';
	import { save } from '@tauri-apps/plugin-dialog';
	import '$lib/styles/vscode.css';
	import { app } from '$lib/state.svelte';
	import { settings } from '$lib/settings.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import ActivityBar, { type SidebarView } from '$lib/shell/ActivityBar.svelte';
	import StatusBar from '$lib/shell/StatusBar.svelte';
	import SiteSidebar from '$lib/sites/SiteSidebar.svelte';
	import TerminalTabs from '$lib/session/TerminalTabs.svelte';
	import SftpPanel from '$lib/sftp/SftpPanel.svelte';
	import TunnelPanel from '$lib/tunnel/TunnelPanel.svelte';
	import AiPanel from '$lib/ai/AiPanel.svelte';
	import SettingsDialog from '$lib/SettingsDialog.svelte';
	import AboutDialog from '$lib/AboutDialog.svelte';
	import UpdateDialog from '$lib/UpdateDialog.svelte';
	import HostKeyDialog from '$lib/HostKeyDialog.svelte';
	import { updater } from '$lib/updater.svelte';

	let view = $state<SidebarView>('sessions');
	let collapsed = $state(false);
	let sidebarWidth = $state(300);
	let showSettings = $state(false);
	let showAbout = $state(false);

	const activeSession = $derived(app.activeTab?.sessionId);
	const tunnelCount = $derived(app.tunnels.length);

	// Fall back to the sessions view if the AI panel is showing when the feature
	// gets disabled in settings (its activity-bar button disappears too).
	$effect(() => {
		if (view === 'ai' && !settings.s.aiEnabled) view = 'sessions';
	});

	// Activity-bar click: re-clicking the open view collapses the sidebar;
	// otherwise switch to that view and make sure the sidebar is showing.
	function selectView(next: SidebarView) {
		if (next === view && !collapsed) collapsed = true;
		else {
			view = next;
			collapsed = false;
		}
	}

	const viewTitle = $derived(
		view === 'sessions'
			? i18n.t('view.sessions')
			: view === 'files'
				? i18n.t('view.files')
				: i18n.t('view.tunnels')
	);

	// Drag the sidebar's right edge to resize (clamped), VS Code-style.
	function startResize(e: MouseEvent) {
		e.preventDefault();
		const startX = e.clientX;
		const startW = sidebarWidth;
		const move = (ev: MouseEvent) => {
			sidebarWidth = Math.min(640, Math.max(220, startW + (ev.clientX - startX)));
		};
		const up = () => {
			window.removeEventListener('mousemove', move);
			window.removeEventListener('mouseup', up);
			document.body.style.cursor = '';
		};
		document.body.style.cursor = 'col-resize';
		window.addEventListener('mousemove', move);
		window.addEventListener('mouseup', up);
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
		settings.init();
		settings.load();

		// The app defines no right-click menu of its own, so suppress the WebView's
		// default context menu everywhere. The terminal attaches its own handler
		// (copy/paste) on its container, which runs first and still works.
		const noContextMenu = (e: MouseEvent) => e.preventDefault();
		window.addEventListener('contextmenu', noContextMenu);

		// Silent background update check shortly after launch — surfaces a prompt
		// only when a newer release exists; no-ops under `tauri dev` / offline.
		const updateTimer = setTimeout(() => updater.checkForUpdates(true), 3000);

		return () => {
			window.removeEventListener('contextmenu', noContextMenu);
			clearTimeout(updateTimer);
		};
	});
</script>

<div class="workbench">
	<div class="main">
		<ActivityBar
			active={view}
			{collapsed}
			{tunnelCount}
			aiEnabled={settings.s.aiEnabled}
			onselect={selectView}
			onsettings={() => (showSettings = true)}
			onabout={() => (showAbout = true)}
		/>

		{#if !collapsed}
			<div class="sidebar" style="width: {sidebarWidth}px">
				{#if view === 'sessions'}
					<SiteSidebar />
				{:else if view === 'ai'}
					<div class="view-head">{i18n.t('ai.title')}</div>
					<AiPanel />
				{:else}
					<div class="view-head">{viewTitle}</div>
					{#if activeSession}
						{#key activeSession}
							{#if view === 'files'}
								<SftpPanel sessionId={activeSession} />
							{:else}
								<TunnelPanel sessionId={activeSession} />
							{/if}
						{/key}
					{:else}
						<div class="empty">
							{view === 'files' ? i18n.t('view.filesEmpty') : i18n.t('view.tunnelsEmpty')}
						</div>
					{/if}
				{/if}
				<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
				<div
					class="resizer"
					role="separator"
					aria-label="Resize sidebar"
					tabindex="-1"
					onmousedown={startResize}
				></div>
			</div>
		{/if}

		<div class="editor"><TerminalTabs /></div>
	</div>

	<StatusBar onToggleLog={toggleLog} onShowTunnels={() => selectView('tunnels')} />
</div>

{#if showSettings}
	<SettingsDialog onclose={() => (showSettings = false)} />
{/if}
{#if showAbout}
	<AboutDialog onclose={() => (showAbout = false)} />
{/if}
<UpdateDialog />
<HostKeyDialog />

<style>
	.workbench {
		position: fixed;
		inset: 0;
		display: flex;
		flex-direction: column;
		background: var(--vsc-editor-bg);
		color: var(--vsc-editor-fg);
	}
	.main {
		display: flex;
		flex: 1 1 auto;
		min-height: 0;
	}
	.sidebar {
		position: relative;
		flex: none;
		min-width: 0;
		display: flex;
		flex-direction: column;
		background: var(--vsc-sidebar-bg);
		color: var(--vsc-sidebar-fg);
		overflow: hidden;
	}
	.view-head {
		flex: none;
		padding: 10px 18px 6px;
		font: 11px var(--vsc-font);
		letter-spacing: 0.6px;
		text-transform: uppercase;
		color: var(--vsc-sidebar-title-fg);
	}
	.empty {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 24px;
		text-align: center;
		color: var(--vsc-muted);
		font: 13px var(--vsc-font);
		line-height: 1.5;
	}
	.editor {
		position: relative;
		flex: 1 1 auto;
		min-width: 0;
		background: var(--vsc-editor-bg);
	}
	/* Invisible 4px grab strip on the sidebar's right edge. */
	.resizer {
		position: absolute;
		top: 0;
		right: 0;
		width: 4px;
		height: 100%;
		cursor: col-resize;
		z-index: 6;
	}
	.resizer:hover {
		background: var(--vsc-focus-border);
	}
</style>
