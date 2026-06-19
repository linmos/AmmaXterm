/** A remote directory entry returned by the `sftp_list` command (FT-1). */
export interface FileEntry {
	name: string;
	is_dir: boolean;
	size: number;
	permissions: number | null;
	modified: number | null;
}
