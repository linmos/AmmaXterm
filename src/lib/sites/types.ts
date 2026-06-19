/** Auth method for a saved site (mirrors the Rust `AuthMethod`, tagged enum). */
export type AuthMethod =
	| { type: 'password' }
	| { type: 'publicKey'; keyPath: string }
	| { type: 'keyboardInteractive' }
	| { type: 'agent' };

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
}
