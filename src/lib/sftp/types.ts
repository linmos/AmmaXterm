/** A remote directory entry returned by the `sftp_list` command (FT-1). */
export interface FileEntry {
	name: string;
	is_dir: boolean;
	is_symlink: boolean;
	size: number;
	permissions: number | null;
	modified: number | null;
	uid: number | null;
	gid: number | null;
}

/** A transfer-queue entry (mirrors the Rust `TransferInfo`, FT-4). */
export interface TransferInfo {
	id: string;
	sessionId: string;
	name: string;
	direction: string;
	total: number;
	done: number;
	status: string;
	error: string | null;
}
