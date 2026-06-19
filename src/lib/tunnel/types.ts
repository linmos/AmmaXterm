/** A tunnel definition (mirrors the Rust `TunnelSpec`). */
export interface TunnelSpec {
	/** "local" (-L) | "dynamic" (-D) | "remote" (-R). */
	kind: string;
	listenPort: number;
	destHost: string;
	destPort: number;
	/** Remote (-R) only: bind the server listener to 0.0.0.0 (LAN-exposed). */
	expose?: boolean;
}

/** A live tunnel snapshot (mirrors the Rust `TunnelInfo`). */
export interface TunnelInfo {
	id: string;
	sessionId: string;
	kind: string;
	listenHost: string;
	listenPort: number;
	destHost: string;
	destPort: number;
	conns: number;
	bytesUp: number;
	bytesDown: number;
}
