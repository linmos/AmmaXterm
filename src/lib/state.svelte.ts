import { invoke, Channel } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { settings } from './settings.svelte';
import type { ChatMessage } from './ai/types';
import type { ImportedSite, Site, SiteInput } from './sites/types';
import type { TerminalApi, TerminalSize } from './terminal/types';
import type { TransferInfo } from './sftp/types';
import type { TunnelInfo, TunnelSpec } from './tunnel/types';

export type TabStatus = 'connecting' | 'connected' | 'closed' | 'error';

/** An open terminal tab. `key` is a stable client id; `sessionId` is the
 *  backend session id assigned once the connection succeeds. */
export interface Tab {
	key: string;
	sessionId?: string;
	/** Saved-site id when the tab came from a site (enables reconnect). */
	siteId?: string;
	title: string;
	host: string;
	status: TabStatus;
	error?: string;
	api?: TerminalApi;
	channel: Channel<string>;
	buffer: Uint8Array[];
	size: TerminalSize;
	logging: boolean;
	/** Last cwd reported by the shell (OSC 7 / 9;9), for SFTP follow-cd. */
	cwd?: string;
}

/** A generated keypair returned by `keygen_generate` (AK-3). */
export interface GeneratedKey {
	publicKey: string;
	privateKey: string;
	fingerprint: string;
}

export interface HostKeyPrompt {
	requestId: string;
	host: string;
	port: number;
	fingerprint: string;
	changed: boolean;
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
	hostKeyPrompt = $state<HostKeyPrompt | null>(null);
	tunnels = $state<TunnelInfo[]>([]);
	transfers = $state<TransferInfo[]>([]);

	get activeTab(): Tab | undefined {
		return this.tabs.find((t) => t.key === this.activeKey);
	}

	async init() {
		await this.loadSites();
		await listen<string>('ssh://closed', (event) => {
			const tab = this.tabs.find((t) => t.sessionId === event.payload);
			// The backend tears down this session's tunnels when it closes; reflect
			// that so the activity-bar badge doesn't keep counting dead tunnels.
			void this.refreshTunnels();
			if (tab && tab.status === 'connected') {
				tab.status = 'closed';
				tab.logging = false;
				// Auto-reconnect saved-session tabs after an unexpected drop (TM-8).
				if (settings.s.autoReconnect && tab.siteId) {
					setTimeout(() => {
						if (tab.status === 'closed') void this.reconnect(tab.key);
					}, 1000);
				}
			}
		});
		await listen<HostKeyPrompt>('ssh://host-key-prompt', (event) => {
			this.hostKeyPrompt = event.payload;
		});
	}

	respondHostKey(trust: boolean) {
		const prompt = this.hostKeyPrompt;
		this.hostKeyPrompt = null;
		if (prompt) {
			invoke('host_key_decision', { requestId: prompt.requestId, trust }).catch(() => {});
		}
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

	/** Clone a saved site as a new record, copying its stored password/passphrase
	 *  to the clone. Returns the newly created site. */
	async duplicateSite(site: Site, name: string): Promise<Site> {
		const input: SiteInput = {
			name,
			host: site.host,
			port: site.port,
			username: site.username,
			auth: site.auth,
			group: site.group,
			tags: [...site.tags],
			tunnels: site.tunnels.map((t) => ({ ...t })),
			proxyJump: [...site.proxyJump],
			overrides: site.overrides ? { ...site.overrides } : null
		};
		const clone = await invoke<Site>('site_add', { input });
		await invoke('site_copy_secrets', { fromId: site.id, toId: clone.id }).catch(() => {});
		await this.loadSites();
		return clone;
	}

	// --- tunnels / port forwarding (PF-1..PF-7) ---

	async refreshTunnels() {
		this.tunnels = await invoke<TunnelInfo[]>('tunnel_list');
	}

	async openTunnel(sessionId: string, spec: TunnelSpec) {
		await invoke<string>('tunnel_open', { sessionId, spec });
		await this.refreshTunnels();
	}

	async closeTunnel(id: string) {
		await invoke('tunnel_close', { id }).catch(() => {});
		await this.refreshTunnels();
	}

	// --- SFTP transfer queue (FT-4) ---

	async refreshTransfers() {
		this.transfers = await invoke<TransferInfo[]>('transfer_list');
	}

	async uploadFile(sessionId: string, localPath: string, remotePath: string) {
		await invoke<string>('transfer_upload', { sessionId, localPath, remotePath });
		await this.refreshTransfers();
	}

	async downloadFile(sessionId: string, remotePath: string, localPath: string) {
		await invoke<string>('transfer_download', { sessionId, remotePath, localPath });
		await this.refreshTransfers();
	}

	async cancelTransfer(id: string) {
		await invoke('transfer_cancel', { id }).catch(() => {});
		await this.refreshTransfers();
	}

	async pauseTransfer(id: string) {
		await invoke('transfer_pause', { id }).catch(() => {});
		await this.refreshTransfers();
	}

	async resumeTransfer(id: string) {
		await invoke('transfer_resume', { id }).catch(() => {});
		await this.refreshTransfers();
	}

	async retryTransfer(id: string) {
		await invoke('transfer_retry', { id }).catch(() => {});
		await this.refreshTransfers();
	}

	async clearTransfer(id: string) {
		await invoke('transfer_clear', { id }).catch(() => {});
		await this.refreshTransfers();
	}

	// --- import / export (SM-7, SM-8) ---

	/** Parse an OpenSSH config into review candidates (default ~/.ssh/config). */
	async importSshConfig(path?: string): Promise<ImportedSite[]> {
		return invoke<ImportedSite[]>('import_ssh_config', { path: path ?? null });
	}

	/** Read an AmmaXterm backup file into review candidates. */
	async importBackup(path: string): Promise<ImportedSite[]> {
		return invoke<ImportedSite[]>('import_sites_backup', { path });
	}

	/** Read saved PuTTY sessions from the Windows registry (SM-7). */
	async importPuttyRegistry(): Promise<ImportedSite[]> {
		return invoke<ImportedSite[]>('import_putty_registry');
	}

	/** Parse a PuTTY `.reg` export into review candidates (SM-7). */
	async importPuttyReg(path: string): Promise<ImportedSite[]> {
		return invoke<ImportedSite[]>('import_putty_reg', { path });
	}

	/** Write all saved sites to a backup file (no secrets). */
	async exportSites(path: string): Promise<void> {
		await invoke('export_sites', { path });
	}

	/** Persist a batch of imported candidates as new sites, then reload. */
	async addImported(entries: ImportedSite[]): Promise<void> {
		for (const e of entries) {
			const input: SiteInput = {
				name: e.name,
				host: e.host,
				port: e.port,
				username: e.username,
				auth: e.auth,
				group: e.group,
				tags: e.tags
			};
			await invoke<Site>('site_add', { input });
		}
		await this.loadSites();
	}

	// --- key generation (AK-3) ---

	async generateKey(algorithm: string, comment: string): Promise<GeneratedKey> {
		return invoke<GeneratedKey>('keygen_generate', { algorithm, comment });
	}

	async saveKey(privatePath: string, privateKey: string, publicKey: string): Promise<void> {
		await invoke('keygen_save', { privatePath, privateKey, publicKey });
	}

	// --- encrypted vault (AK-4) ---

	async vaultStatus(): Promise<{ exists: boolean; unlocked: boolean }> {
		return invoke('vault_status');
	}
	async vaultUnlock(masterPassword: string): Promise<void> {
		await invoke('vault_unlock', { masterPassword });
	}
	async vaultLock(): Promise<void> {
		await invoke('vault_lock');
	}
	async vaultSetSecret(key: string, value: string): Promise<void> {
		await invoke('vault_set_secret', { key, value });
	}
	async vaultDeleteSecret(key: string): Promise<void> {
		await invoke('vault_delete_secret', { key });
	}
	async vaultKeys(): Promise<string[]> {
		return invoke('vault_keys');
	}

	// --- AI assistant (multi-provider, BYO key) ---

	/** Stream a chat completion; text deltas arrive on `onChunk`. Resolves when
	 *  the stream completes, rejects on provider/transport error. */
	async aiStream(
		requestId: string,
		provider: string,
		model: string,
		messages: ChatMessage[],
		system: string | undefined,
		onChunk: Channel<string>
	): Promise<void> {
		await invoke('ai_stream', {
			requestId,
			provider,
			model,
			messages,
			system: system ?? null,
			onChunk
		});
	}

	/** Cancel an in-flight AI stream (fire-and-forget). */
	aiCancel(requestId: string) {
		invoke('ai_cancel', { requestId }).catch(() => {});
	}

	/** Store/replace a provider's API key (keychain / vault). */
	async aiSetApiKey(provider: string, key: string): Promise<void> {
		await invoke('ai_set_api_key', { provider, key });
	}

	/** Whether a provider already has a stored API key. */
	async aiHasApiKey(provider: string): Promise<boolean> {
		return invoke<boolean>('ai_has_api_key', { provider });
	}

	/** Move a site into a group (or out of all groups when `group` is null).
	 *  Rewrites the site record only — the stored secret is untouched. */
	async moveSiteToGroup(site: Site, group: string | null): Promise<void> {
		if ((site.group ?? null) === group) return;
		const input: SiteInput = {
			name: site.name,
			host: site.host,
			port: site.port,
			username: site.username,
			auth: site.auth,
			group,
			tags: site.tags,
			tunnels: site.tunnels,
			proxyJump: site.proxyJump,
			overrides: site.overrides
		};
		await this.updateSite(site.id, input);
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

	/** Wire a tab's output channel to its terminal (buffering pre-mount). */
	private bindChannel(tab: Tab) {
		tab.channel.onmessage = (b64) => {
			const bytes = b64ToBytes(b64);
			if (tab.api) tab.api.write(bytes);
			else tab.buffer.push(bytes);
		};
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
			size: { cols: 80, rows: 24 },
			logging: false
		};
		this.tabs.push(base);
		const tab = this.tabs[this.tabs.length - 1];
		this.bindChannel(tab);
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
		tab.siteId = site.id;
		try {
			tab.sessionId = await invoke<string>('site_connect', {
				siteId: site.id,
				cols: tab.size.cols,
				rows: tab.size.rows,
				onOutput: tab.channel
			});
			tab.status = 'connected';
			// The site may auto-establish tunnels on connect (PF-4) — reflect them.
			if (site.tunnels.length) void this.refreshTunnels();
		} catch (err) {
			tab.status = 'error';
			tab.error = errMessage(err);
		}
	}

	/** Reconnect a closed/errored saved-session tab in place, reusing its terminal. */
	async reconnect(key: string) {
		const tab = this.tabs.find((t) => t.key === key);
		if (!tab || !tab.siteId || tab.status === 'connecting' || tab.status === 'connected') return;
		const site = this.sites.find((s) => s.id === tab.siteId);
		if (!site) return;
		tab.status = 'connecting';
		tab.error = undefined;
		tab.buffer = [];
		try {
			tab.sessionId = await invoke<string>('site_connect', {
				siteId: site.id,
				cols: tab.size.cols,
				rows: tab.size.rows,
				onOutput: tab.channel
			});
			tab.status = 'connected';
			tab.api?.focus();
		} catch (err) {
			tab.status = 'error';
			tab.error = errMessage(err);
		}
	}

	/** Toggle session logging for a tab (TM-12); `path` is required to start. */
	async startLog(key: string, path: string) {
		const tab = this.tabs.find((t) => t.key === key);
		if (!tab?.sessionId) return;
		await invoke('session_start_log', { id: tab.sessionId, path });
		tab.logging = true;
	}

	async stopLog(key: string) {
		const tab = this.tabs.find((t) => t.key === key);
		if (!tab?.sessionId) return;
		await invoke('session_stop_log', { id: tab.sessionId }).catch(() => {});
		tab.logging = false;
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

	/** Record the shell's reported cwd for a tab (drives SFTP follow-cd). */
	setTabCwd(key: string, path: string) {
		const tab = this.tabs.find((t) => t.key === key);
		if (tab) tab.cwd = path;
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
		if (tab.sessionId) {
			await invoke('ssh_disconnect', { id: tab.sessionId }).catch(() => {});
			// `ssh_disconnect` closed this session's tunnels backend-side — sync the
			// list so the activity-bar badge drops them.
			void this.refreshTunnels();
		}
		tab.api?.dispose();
		this.tabs.splice(idx, 1);
		if (this.activeKey === key) {
			this.activeKey = this.tabs[Math.min(idx, this.tabs.length - 1)]?.key ?? null;
		}
	}
}

export const app = new AppState();
