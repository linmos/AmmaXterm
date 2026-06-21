// In-app auto-update (Tauri v2 updater). Checks GitHub Releases for a newer
// *signed* build, downloads + installs it in place, then relaunches — so users
// no longer have to uninstall and reinstall by hand.
//
// The updater only resolves an update for an installed build whose embedded
// public key matches the release signature; under `tauri dev` or offline,
// check() throws, and the silent startup check just swallows it.

import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export type UpdatePhase =
	| 'idle'
	| 'checking'
	| 'available'
	| 'downloading'
	| 'ready'
	| 'uptodate'
	| 'error';

class Updater {
	phase = $state<UpdatePhase>('idle');
	version = $state(''); // newer version, once found
	notes = $state(''); // release notes / changelog body
	progress = $state(0); // 0–100 during download
	error = $state('');

	#update: Update | null = null;
	#downloaded = 0;
	#contentLength = 0;

	/**
	 * Check for a newer release. `silent` (startup) swallows errors and the
	 * "already latest" case so nothing pops up unless an update exists; the
	 * manual "Check for updates" button passes false so the user always gets
	 * feedback.
	 */
	async checkForUpdates(silent = false) {
		if (this.phase === 'checking' || this.phase === 'downloading') return;
		this.error = '';
		this.phase = 'checking';
		try {
			const update = await check();
			if (update) {
				this.#update = update;
				this.version = update.version;
				this.notes = update.body ?? '';
				this.phase = 'available';
			} else {
				this.#update = null;
				this.phase = silent ? 'idle' : 'uptodate';
			}
		} catch (e) {
			this.#update = null;
			if (silent) {
				this.phase = 'idle';
			} else {
				this.error = String(e);
				this.phase = 'error';
			}
		}
	}

	/** Download + install the pending update, tracking byte progress. */
	async downloadAndInstall() {
		if (!this.#update) return;
		this.error = '';
		this.progress = 0;
		this.#downloaded = 0;
		this.#contentLength = 0;
		this.phase = 'downloading';
		try {
			await this.#update.downloadAndInstall((event) => {
				switch (event.event) {
					case 'Started':
						this.#contentLength = event.data.contentLength ?? 0;
						break;
					case 'Progress':
						this.#downloaded += event.data.chunkLength;
						this.progress = this.#contentLength
							? Math.min(100, Math.round((this.#downloaded / this.#contentLength) * 100))
							: 0;
						break;
					case 'Finished':
						this.progress = 100;
						break;
				}
			});
			this.phase = 'ready';
		} catch (e) {
			this.error = String(e);
			this.phase = 'error';
		}
	}

	/** Restart into the freshly installed version. */
	async relaunchApp() {
		await relaunch();
	}

	/** Dismiss the prompt, but never interrupt an in-flight download/install. */
	dismiss() {
		if (this.phase === 'downloading') return;
		this.phase = 'idle';
		this.error = '';
	}
}

export const updater = new Updater();
