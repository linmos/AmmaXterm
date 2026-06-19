<script lang="ts">
	import { save } from '@tauri-apps/plugin-dialog';
	import { app, type Tab } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import { settings } from '$lib/settings.svelte';
	import Terminal from '$lib/terminal/Terminal.svelte';

	async function toggleLog(tab: Tab) {
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
</script>

<div class="tabs-wrap">
	<div class="tabbar">
		{#each app.tabs as tab (tab.key)}
			<div class="tab" class:active={tab.key === app.activeKey}>
				<button class="label" onclick={() => app.setActive(tab.key)} title={tab.host}>
					<span class="dot {tab.status}"></span>
					<span class="title">{tab.title}</span>
				</button>
				<button class="close" title={i18n.t('common.delete')} onclick={() => app.closeTab(tab.key)}>×</button>
			</div>
		{/each}
	</div>

	<div class="panes">
		{#each app.tabs as tab (tab.key)}
			<div class="pane" class:active={tab.key === app.activeKey}>
				{#if tab.status === 'error'}
					<div class="msg error">
						{i18n.t('tabs.failed')}<br />{tab.error}
						{#if tab.siteId}
							<div><button class="reconnect" onclick={() => app.reconnect(tab.key)}>{i18n.t('tabs.reconnect')}</button></div>
						{/if}
					</div>
				{:else}
					{#if tab.status === 'connected'}
						<button
							class="pane-ctl"
							class:on={tab.logging}
							title={tab.logging ? i18n.t('tabs.stopLog') : i18n.t('tabs.startLog')}
							onclick={() => toggleLog(tab)}
						>
							{tab.logging ? '⏺' : '▤'}
						</button>
					{/if}
					<Terminal
						onReady={(api) => app.setTabApi(tab.key, api)}
						onData={(data) => app.sendInput(tab.key, data)}
						onResize={(size) => app.resizeTab(tab.key, size)}
						fontSize={settings.s.fontSize}
						fontFamily={settings.s.fontFamily}
						scrollback={settings.s.scrollback}
						theme={settings.theme}
					/>
					{#if tab.status === 'closed'}
						<div class="badge">
							{i18n.t('tabs.closed')}
							{#if tab.siteId}
								<button class="reconnect sm" onclick={() => app.reconnect(tab.key)}>{i18n.t('tabs.reconnect')}</button>
							{/if}
						</div>
					{/if}
				{/if}
			</div>
		{/each}
		{#if !app.tabs.length}
			<div class="msg empty">
				{i18n.t('tabs.empty')}<br />{i18n.t('tabs.emptyHint')}
			</div>
		{/if}
	</div>
</div>

<style>
	.tabs-wrap {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-width: 0;
		background: #1e1e1e;
	}
	.tabbar {
		display: flex;
		gap: 2px;
		padding: 4px 6px 0;
		overflow-x: auto;
		background: #252526;
	}
	.tab {
		display: flex;
		align-items: center;
		border-radius: 6px 6px 0 0;
		background: #2d2d2d;
		color: #bbb;
	}
	.tab.active {
		background: #1e1e1e;
		color: #fff;
	}
	.label {
		display: flex;
		align-items: center;
		gap: 6px;
		max-width: 180px;
		padding: 6px 4px 6px 10px;
		border: none;
		background: transparent;
		color: inherit;
		font: 12px system-ui, sans-serif;
		cursor: pointer;
	}
	.title {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex: none;
		background: #888;
	}
	.dot.connecting {
		background: #d7ba7d;
	}
	.dot.connected {
		background: #3fb950;
	}
	.dot.closed {
		background: #888;
	}
	.dot.error {
		background: #f48771;
	}
	.close {
		padding: 4px 8px;
		border: none;
		background: transparent;
		color: inherit;
		font-size: 14px;
		cursor: pointer;
		opacity: 0.6;
	}
	.close:hover {
		opacity: 1;
	}
	.panes {
		position: relative;
		flex: 1;
		min-height: 0;
	}
	.pane {
		position: absolute;
		inset: 0;
		display: none;
		padding: 6px;
		box-sizing: border-box;
	}
	.pane.active {
		display: block;
	}
	.msg {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		text-align: center;
		color: #999;
		font: 14px system-ui, sans-serif;
	}
	.msg.error {
		color: #f48771;
	}
	.badge {
		position: absolute;
		top: 10px;
		right: 14px;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 2px 8px;
		border-radius: 10px;
		background: #3a3a3a;
		color: #ccc;
		font: 11px system-ui, sans-serif;
	}
	.pane-ctl {
		position: absolute;
		top: 6px;
		left: 10px;
		z-index: 6;
		padding: 3px 8px;
		border: 1px solid #555;
		border-radius: 6px;
		background: #252526;
		color: #ddd;
		font: 12px system-ui, sans-serif;
		cursor: pointer;
	}
	.pane-ctl.on {
		background: #7a1f1f;
		border-color: #a33;
		color: #fff;
	}
	.reconnect {
		margin-top: 8px;
		padding: 6px 12px;
		border: none;
		border-radius: 6px;
		background: #0e639c;
		color: #fff;
		font: 13px system-ui, sans-serif;
		cursor: pointer;
	}
	.reconnect.sm {
		margin: 0;
		padding: 2px 8px;
		font-size: 11px;
	}
</style>
