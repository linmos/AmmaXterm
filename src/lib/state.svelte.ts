import { invoke, Channel } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Site, SiteInput } from './sites/types';
import type { TerminalApi, TerminalSize } from './terminal/types';

export type TabStatus = 'connecting' | 'connected' | 'closed' | 'error';

/** An open terminal tab. `key` is a stable client id; `sessionId` is the
 *  backend session id assigned once the connection succeeds. */
export interface Tab {
	key: string;
	sessionId?: string;
	title: string;
	host: string;
	status: TabStatus;
	error?: string;
	api?: TerminalApi;
	channel: Channel<string>;
	buffer: Uint8Array[];
	size: TerminalSize;
}

function b64ToBytes(b64: string): Uint8Array {
	const bin = atob(b64);
	const bytes = new Uint8Array(bin.length);
	for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
	return bytes;
}

function errMessage(err: unknown): string {
	const e = err as { message?: string } | undefined;
	return e?.message ?? String(err);
}

class AppState {
	sites = $state<Site[]>([]);
	tabs = $state<Tab[]>([]);
	activeKey = $state<string | null>(null);

	get activeTab(): Tab | undefined {
		return this.tabs.find((t) => t.key === this.activeKey);
	}

	async init() {
		await this.loadSites();
		await listen<string>('ssh://closed', (event) => {
			const tab = this.tabs.find((t) => t.sessionId === event.payload);
			if (tab && tab.status === 'connected') tab.status = 'closed';
		});
	}

	// --- sites ---

	async loadSites() {
		this.sites = await invoke<Site[]>('site_list');
	}

	async addSite(input: SiteInput, password?: string): Promise<Site> {
		const site = await invoke<Site>('site_add', { input });
		if (password) await invoke('site_set_password', { siteId: site.id, password });
		await this.loadSites();
		return site;
	}

	async updateSite(id: string, input: SiteInput, password?: string): Promise<void> {
		await invoke('site_update', { id, input });
		if (password) await invoke('site_set_password', { siteId: id, password });
		await this.loadSites();
	}

	async deleteSite(id: string): Promise<void> {
		await invoke('site_delete', { id });
		await this.loadSites();
	}

	// --- tabs / sessions ---

	setActive(key: string) {
		this.activeKey = key;
		requestAnimationFrame(() => {
			const tab = this.activeTab;
			tab?.api?.fit();
			tab?.api?.focus();
		});
	}

	/** Push a new tab and return the reactive (proxied) element. */
	private newTab(title: string, host: string): Tab {
		const base: Tab = {
			key: crypto.randomUUID(),
			title,
			host,
			status: 'connecting',
			channel: new Channel<string>(),
			buffer: [],
			size: { cols: 80, rows: 24 }
		};
		this.tabs.push(base);
		const tab = this.tabs[this.tabs.length - 1];
		tab.channel.onmessage = (b64) => {
			const bytes = b64ToBytes(b64);
			if (tab.api) tab.api.write(bytes);
			else tab.buffer.push(bytes);
		};
		this.activeKey = tab.key;
		return tab;
	}

	async quickConnect(opts: { host: string; port: number; username: string; password: string }) {
		const tab = this.newTab(`${opts.username}@${opts.host}`, opts.host);
		try {
			tab.sessionId = await invoke<string>('ssh_connect', {
				options: { ...opts, cols: tab.size.cols, rows: tab.size.rows },
				onOutput: tab.channel
			});
			tab.status = 'connected';
		} catch (err) {
			tab.status = 'error';
			tab.error = errMessage(err);
		}
	}

	async connectSite(site: Site) {
		const tab = this.newTab(site.name, site.host);
		try {
			tab.sessionId = await invoke<string>('site_connect', {
				siteId: site.id,
				cols: tab.size.cols,
				rows: tab.size.rows,
				onOutput: tab.channel
			});
			tab.status = 'connected';
		} catch (err) {
			tab.status = 'error';
			tab.error = errMessage(err);
		}
	}

	/** Called when a tab's terminal mounts; flushes any buffered output. */
	setTabApi(key: string, api: TerminalApi) {
		const tab = this.tabs.find((t) => t.key === key);
		if (!tab) return;
		tab.api = api;
		for (const chunk of tab.buffer) api.write(chunk);
		tab.buffer = [];
		if (this.activeKey === key) api.focus();
	}

	sendInput(key: string, data: string) {
		const tab = this.tabs.find((t) => t.key === key);
		if (tab?.sessionId) invoke('ssh_send_input', { id: tab.sessionId, data }).catch(() => {});
	}

	resizeTab(key: string, size: TerminalSize) {
		const tab = this.tabs.find((t) => t.key === key);
		if (!tab) return;
		tab.size = size;
		if (tab.sessionId) {
			invoke('ssh_resize', { id: tab.sessionId, cols: size.cols, rows: size.rows }).catch(() => {});
		}
	}

	async closeTab(key: string) {
		const idx = this.tabs.findIndex((t) => t.key === key);
		if (idx === -1) return;
		const tab = this.tabs[idx];
		if (tab.sessionId) await invoke('ssh_disconnect', { id: tab.sessionId }).catch(() => {});
		tab.api?.dispose();
		this.tabs.splice(idx, 1);
		if (this.activeKey === key) {
			this.activeKey = this.tabs[Math.min(idx, this.tabs.length - 1)]?.key ?? null;
		}
	}
}

export const app = new AppState();
