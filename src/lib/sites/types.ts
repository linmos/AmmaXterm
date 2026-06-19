/** Auth method for a saved site (mirrors the Rust `AuthMethod`, tagged enum). */
export type AuthMethod =
	| { type: 'password' }
	| { type: 'publicKey'; keyPath: string }
	| { type: 'keyboardInteractive' }
	| { type: 'agent' };

import type { TunnelSpec } from '../tunnel/types';

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
