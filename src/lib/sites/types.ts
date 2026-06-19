/** Auth method for a saved site (mirrors the Rust `AuthMethod`, tagged enum). */
export type AuthMethod =
	| { type: 'password' }
	| { type: 'publicKey'; keyPath: string }
	| { type: 'keyboardInteractive' }
	| { type: 'agent' };

import type { TunnelSpec } from '../tunnel/types';

/** Per-site overrides of the global defaults (mirrors Rust `SiteOverrides`).
 *  Every field is optional — omitted means "inherit the global setting" (SM-6). */
export interface SiteOverrides {
	theme?: string;
	fontFamily?: string;
	fontSize?: number;
	scrollback?: number;
	keepaliveSecs?: number;
}

/** A saved connection (mirrors the Rust `Site`). */
export interface Site {
	id: string;
	name: string;
	host: string;
	port: number;
	username: string;
	auth: AuthMethod;
	group: string | null;
	tags: string[];
	tunnels: TunnelSpec[];
	/** ProxyJump chain: ids of saved sites to hop through, in order (TM-9). */
	proxyJump: string[];
	/** Per-site overrides of the global defaults (SM-6); null = none. */
	overrides: SiteOverrides | null;
}

/** Create/update payload (mirrors the Rust `SiteInput`). */
export interface SiteInput {
	name: string;
	host: string;
	port: number;
	username: string;
	auth: AuthMethod;
	group?: string | null;
	tags?: string[];
	tunnels?: TunnelSpec[];
	proxyJump?: string[];
	overrides?: SiteOverrides | null;
}

/** A connection candidate from an import source (mirrors Rust `ImportedSite`). */
export interface ImportedSite {
	name: string;
	host: string;
	port: number;
	username: string;
	auth: AuthMethod;
	group: string | null;
	tags: string[];
}
