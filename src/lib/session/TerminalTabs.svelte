<script lang="ts">
	import { app } from '$lib/state.svelte';
	import type { Tab } from '$lib/state.svelte';
	import { i18n } from '$lib/i18n.svelte';
	import { settings, xtermTheme } from '$lib/settings.svelte';
	import Terminal from '$lib/terminal/Terminal.svelte';

	/** Resolve a tab's effective terminal appearance: per-site overrides (SM-6)
	 *  fall back to the global settings. */
	function appearance(tab: Tab) {
		const o = tab.siteId ? app.sites.find((s) => s.id === tab.siteId)?.overrides : null;
		return {
			fontSize: o?.fontSize ?? settings.s.fontSize,
			fontFamily: o?.fontFamily || settings.s.fontFamily,
			scrollback: o?.scrollback ?? settings.s.scrollback,
			theme: o?.theme ? xtermTheme(o.theme) : settings.theme
		};
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
					{@const a = appearance(tab)}
					<Terminal
						onReady={(api) => app.setTabApi(tab.key, api)}
						onData={(data) => app.sendInput(tab.key, data)}
						onResize={(size) => app.resizeTab(tab.key, size)}
						onCwd={(p) => app.setTabCwd(tab.key, p)}
						fontSize={a.fontSize}
						fontFamily={a.fontFamily}
						scrollback={a.scrollback}
						theme={a.theme}
						copyOnSelect={settings.s.copyOnSelect}
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
		background: var(--vsc-editor-bg);
	}
	.tabbar {
		display: flex;
		overflow-x: auto;
		background: var(--vsc-tabbar-bg);
		min-height: 35px;
	}
	.tabbar::-webkit-scrollbar {
		height: 0;
	}
	.tab {
		display: flex;
		align-items: center;
		height: 35px;
		background: var(--vsc-tab-inactive-bg);
		color: var(--vsc-tab-inactive-fg);
		border-right: 1px solid var(--vsc-tab-border);
		border-top: 1px solid transparent;
	}
	.tab.active {
		background: var(--vsc-tab-active-bg);
		color: var(--vsc-tab-active-fg);
		border-top-color: var(--vsc-tab-active-top);
	}
	.label {
		display: flex;
		align-items: center;
		gap: 7px;
		max-width: 180px;
		padding: 0 4px 0 12px;
		height: 100%;
		border: none;
		background: transparent;
		color: inherit;
		font: 13px var(--vsc-font);
		cursor: pointer;
	}
	.title {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.dot {
		width: 7px;
		height: 7px;
		border-radius: 50%;
		flex: none;
		background: var(--vsc-muted);
	}
	.dot.connecting {
		background: var(--vsc-yellow);
	}
	.dot.connected {
		background: var(--vsc-green);
	}
	.dot.closed {
		background: var(--vsc-muted);
	}
	.dot.error {
		background: var(--vsc-red);
	}
	.close {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 22px;
		height: 22px;
		margin-right: 6px;
		border: none;
		border-radius: 5px;
		background: transparent;
		color: inherit;
		font-size: 15px;
		line-height: 1;
		cursor: pointer;
		opacity: 0;
	}
	.tab.active .close,
	.tab:hover .close {
		opacity: 0.8;
	}
	.close:hover {
		opacity: 1;
		background: rgba(255, 255, 255, 0.12);
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
		padding: 4px 0 0 4px;
		box-sizing: border-box;
	}
	.pane.active {
		display: block;
	}
	.msg {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		text-align: center;
		color: var(--vsc-muted);
		font: 14px var(--vsc-font);
		line-height: 1.6;
	}
	.msg.error {
		color: var(--vsc-red);
	}
	.badge {
		position: absolute;
		top: 10px;
		right: 14px;
		/* Above xterm's canvas layers (which carry positive z-index and would
		   otherwise swallow the click even though the badge paints through). */
		z-index: 6;
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 3px 10px;
		border-radius: 11px;
		background: var(--vsc-widget-bg);
		border: 1px solid var(--vsc-widget-border);
		color: var(--vsc-sidebar-fg);
		font: 11px var(--vsc-font);
	}
	.reconnect {
		margin-top: 12px;
		padding: 6px 14px;
		border: none;
		border-radius: var(--vsc-radius);
		background: var(--vsc-button-bg);
		color: var(--vsc-button-fg);
		font: 13px var(--vsc-font);
		cursor: pointer;
	}
	.reconnect:hover {
		background: var(--vsc-button-hover);
	}
	.reconnect.sm {
		margin: 0;
		padding: 2px 10px;
		font-size: 11px;
	}
</style>
