<script lang="ts">
	import { i18n } from '$lib/i18n.svelte';

	export type SidebarView = 'sessions' | 'files' | 'tunnels';

	interface Props {
		active: SidebarView;
		collapsed: boolean;
		tunnelCount: number;
		onselect: (view: SidebarView) => void;
		onsettings: () => void;
		onabout: () => void;
	}
	let { active, collapsed, tunnelCount, onselect, onsettings, onabout }: Props = $props();

	// A view button reads "active" only when its view is showing *and* the
	// sidebar is open — clicking the open view collapses it (VS Code behaviour).
	function isOn(view: SidebarView): boolean {
		return active === view && !collapsed;
	}
</script>

<nav class="activitybar" aria-label="Views">
	<div class="group top">
		<button
			class="item"
			class:on={isOn('sessions')}
			title={i18n.t('view.sessions')}
			aria-label={i18n.t('view.sessions')}
			aria-pressed={isOn('sessions')}
			onclick={() => onselect('sessions')}
		>
			<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2">
				<rect x="2" y="2.5" width="12" height="4.4" rx="1" />
				<rect x="2" y="9.1" width="12" height="4.4" rx="1" />
				<circle cx="4.5" cy="4.7" r="0.65" fill="currentColor" stroke="none" />
				<circle cx="4.5" cy="11.3" r="0.65" fill="currentColor" stroke="none" />
			</svg>
		</button>

		<button
			class="item"
			class:on={isOn('files')}
			title={i18n.t('view.files')}
			aria-label={i18n.t('view.files')}
			aria-pressed={isOn('files')}
			onclick={() => onselect('files')}
		>
			<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2">
				<path
					d="M1.6 4.2c0-.6.4-1 1-1h3l1.3 1.6h6.5c.6 0 1 .4 1 1v6.4c0 .6-.4 1-1 1h-11c-.6 0-1-.4-1-1z"
					stroke-linejoin="round"
				/>
			</svg>
		</button>

		<button
			class="item"
			class:on={isOn('tunnels')}
			title={i18n.t('view.tunnels')}
			aria-label={i18n.t('view.tunnels')}
			aria-pressed={isOn('tunnels')}
			onclick={() => onselect('tunnels')}
		>
			<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round">
				<path d="M2 5.5h8.5M8.5 3l2.5 2.5L8.5 8" />
				<path d="M14 10.5H5.5M7.5 8 5 10.5 7.5 13" />
			</svg>
			{#if tunnelCount > 0}<span class="badge">{tunnelCount}</span>{/if}
		</button>
	</div>

	<div class="group bottom">
		<button class="item" title={i18n.t('about.open')} aria-label={i18n.t('about.open')} onclick={onabout}>
			<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2">
				<circle cx="8" cy="8" r="6.4" />
				<path d="M8 7.1v4" stroke-linecap="round" />
				<circle cx="8" cy="4.8" r="0.5" fill="currentColor" stroke="none" />
			</svg>
		</button>
		<button class="item" title={i18n.t('settings.open')} aria-label={i18n.t('settings.open')} onclick={onsettings}>
			<svg viewBox="0 0 24 24" fill="currentColor">
				<path
					d="M19.14 12.94c.03-.31.05-.62.05-.94s-.02-.63-.06-.94l2.03-1.58a.49.49 0 0 0 .12-.61l-1.92-3.32a.49.49 0 0 0-.59-.22l-2.39.96a7.02 7.02 0 0 0-1.62-.94l-.36-2.54A.49.49 0 0 0 13.5 2h-3a.49.49 0 0 0-.49.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96a.49.49 0 0 0-.59.22L2.74 8.47a.5.5 0 0 0 .12.61l2.03 1.58c-.04.31-.06.63-.06.94s.02.63.06.94l-2.03 1.58a.49.49 0 0 0-.12.61l1.92 3.32c.13.24.42.32.59.22l2.39-.96c.49.38 1.03.7 1.62.94l.36 2.54c.04.24.25.41.49.41h3c.24 0 .45-.17.49-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.18.1.46.02.59-.22l1.92-3.32a.49.49 0 0 0-.12-.61zM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7z"
				/>
			</svg>
		</button>
	</div>
</nav>

<style>
	.activitybar {
		display: flex;
		flex-direction: column;
		justify-content: space-between;
		width: 48px;
		flex: none;
		background: var(--vsc-activitybar-bg);
		user-select: none;
	}
	.group {
		display: flex;
		flex-direction: column;
	}
	.item {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		height: 48px;
		width: 48px;
		padding: 0;
		border: none;
		background: transparent;
		color: var(--vsc-activitybar-inactive);
		cursor: pointer;
	}
	.item svg {
		width: 24px;
		height: 24px;
	}
	.item:hover {
		color: var(--vsc-activitybar-fg);
	}
	.item.on {
		color: var(--vsc-activitybar-fg);
	}
	/* The active view marker: a 2px bar on the inner edge, like VS Code. */
	.item.on::before {
		content: '';
		position: absolute;
		left: 0;
		top: 0;
		bottom: 0;
		width: 2px;
		background: var(--vsc-activitybar-active-border);
	}
	.item:focus-visible {
		outline: 1px solid var(--vsc-focus-border);
		outline-offset: -1px;
	}
	.badge {
		position: absolute;
		right: 6px;
		bottom: 6px;
		min-width: 15px;
		height: 15px;
		padding: 0 3px;
		box-sizing: border-box;
		border-radius: 8px;
		background: var(--vsc-activitybar-badge-bg);
		color: #fff;
		font: 600 9px var(--vsc-font);
		line-height: 15px;
		text-align: center;
	}
</style>
