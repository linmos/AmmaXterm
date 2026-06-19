<script lang="ts">
	import { app } from '$lib/state.svelte';
	import Terminal from '$lib/terminal/Terminal.svelte';
</script>

<div class="tabs-wrap">
	<div class="tabbar">
		{#each app.tabs as tab (tab.key)}
			<div class="tab" class:active={tab.key === app.activeKey}>
				<button class="label" onclick={() => app.setActive(tab.key)} title={tab.host}>
					<span class="dot {tab.status}"></span>
					<span class="title">{tab.title}</span>
				</button>
				<button class="close" title="Close" onclick={() => app.closeTab(tab.key)}>×</button>
			</div>
		{/each}
	</div>

	<div class="panes">
		{#each app.tabs as tab (tab.key)}
			<div class="pane" class:active={tab.key === app.activeKey}>
				{#if tab.status === 'error'}
					<div class="msg error">Connection failed:<br />{tab.error}</div>
				{:else}
					<Terminal
						onReady={(api) => app.setTabApi(tab.key, api)}
						onData={(data) => app.sendInput(tab.key, data)}
						onResize={(size) => app.resizeTab(tab.key, size)}
					/>
					{#if tab.status === 'closed'}
						<div class="badge">session closed</div>
					{/if}
				{/if}
			</div>
		{/each}
		{#if !app.tabs.length}
			<div class="msg empty">
				No active sessions.<br />Connect to a site or use Quick Connect (⚡).
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
		padding: 2px 8px;
		border-radius: 10px;
		background: #3a3a3a;
		color: #ccc;
		font: 11px system-ui, sans-serif;
	}
</style>
