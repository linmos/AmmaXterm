// Lightweight, dependency-free i18n (§6.5). Strings are externalized here;
// the active locale is a rune so `i18n.t(...)` in a template reacts to changes.
// Full coverage / more locales is ST-4 (M3).

type Messages = Record<string, string>;

const en: Messages = {
	'sidebar.quickConnect': 'Quick connect',
	'sidebar.newSite': 'New site',
	'sidebar.search': 'Search sites…',
	'sidebar.empty': 'No sites yet — click ＋ to add one.',
	'common.connect': 'Connect',
	'common.connecting': 'Connecting…',
	'common.cancel': 'Cancel',
	'common.save': 'Save',
	'common.saving': 'Saving…',
	'common.host': 'Host',
	'common.user': 'User',
	'common.port': 'Port',
	'common.password': 'Password',
	'common.edit': 'Edit',
	'common.delete': 'Delete',
	'common.sure': 'Sure?',
	'common.files': 'Files',
	'common.disconnect': 'Disconnect',
	'site.new': 'New site',
	'site.edit': 'Edit site',
	'site.name': 'Name',
	'site.auth': 'Auth',
	'auth.password': 'Password',
	'auth.publicKey': 'Public key',
	'auth.keyboardInteractive': 'Keyboard-interactive',
	'auth.agent': 'SSH agent',
	'site.keyPath': 'Private key path',
	'site.passphrase': 'Key passphrase',
	'site.blankKeep': '(blank = keep)',
	'tabs.empty': 'No active sessions.',
	'tabs.emptyHint': 'Connect to a site or use Quick Connect (⚡).',
	'tabs.failed': 'Connection failed:',
	'tabs.closed': 'session closed',
	'sftp.up': 'Up one level',
	'sftp.newFolder': 'New folder',
	'sftp.folderName': 'Folder name',
	'sftp.create': 'Create',
	'sftp.upload': 'Upload file',
	'sftp.refresh': 'Refresh',
	'sftp.download': 'Download',
	'sftp.rename': 'Rename',
	'sftp.empty': 'empty',
	'hostkey.unknownTitle': 'Unknown host key',
	'hostkey.changedTitle': '⚠ Host key changed',
	'hostkey.firstTime':
		'First time connecting to this host. Verify the fingerprint before trusting it.',
	'hostkey.changedWarn':
		"The server's host key differs from the one previously saved. This may indicate a man-in-the-middle attack. Only continue if you know why the key changed.",
	'hostkey.reject': 'Reject',
	'hostkey.trust': 'Trust',
	'hostkey.trustAnyway': 'Trust anyway'
};

const zhTW: Messages = {
	'sidebar.quickConnect': '快速連線',
	'sidebar.newSite': '新增站台',
	'sidebar.search': '搜尋站台…',
	'sidebar.empty': '尚無站台 — 點 ＋ 新增。',
	'common.connect': '連線',
	'common.connecting': '連線中…',
	'common.cancel': '取消',
	'common.save': '儲存',
	'common.saving': '儲存中…',
	'common.host': '主機',
	'common.user': '使用者',
	'common.port': '連接埠',
	'common.password': '密碼',
	'common.edit': '編輯',
	'common.delete': '刪除',
	'common.sure': '確定？',
	'common.files': '檔案',
	'common.disconnect': '中斷連線',
	'site.new': '新增站台',
	'site.edit': '編輯站台',
	'site.name': '名稱',
	'site.auth': '認證',
	'auth.password': '密碼',
	'auth.publicKey': '公開金鑰',
	'auth.keyboardInteractive': 'Keyboard-interactive',
	'auth.agent': 'SSH agent',
	'site.keyPath': '私鑰路徑',
	'site.passphrase': '金鑰密語',
	'site.blankKeep': '（留空則不變）',
	'tabs.empty': '沒有作用中的連線。',
	'tabs.emptyHint': '從站台連線，或使用快速連線（⚡）。',
	'tabs.failed': '連線失敗：',
	'tabs.closed': '連線已關閉',
	'sftp.up': '上一層',
	'sftp.newFolder': '新增資料夾',
	'sftp.folderName': '資料夾名稱',
	'sftp.create': '建立',
	'sftp.upload': '上傳檔案',
	'sftp.refresh': '重新整理',
	'sftp.download': '下載',
	'sftp.rename': '重新命名',
	'sftp.empty': '空',
	'hostkey.unknownTitle': '未知的主機金鑰',
	'hostkey.changedTitle': '⚠ 主機金鑰已變更',
	'hostkey.firstTime': '第一次連線到此主機。信任前請先核對指紋。',
	'hostkey.changedWarn':
		'伺服器的主機金鑰與先前儲存的不同，可能是中間人攻擊。除非你清楚金鑰變更的原因，否則請勿繼續。',
	'hostkey.reject': '拒絕',
	'hostkey.trust': '信任',
	'hostkey.trustAnyway': '仍然信任'
};

const dicts: Record<string, Messages> = { en, 'zh-TW': zhTW };

function detect(): 'en' | 'zh-TW' {
	const l = typeof navigator !== 'undefined' ? navigator.language : 'en';
	return l.toLowerCase().startsWith('zh') ? 'zh-TW' : 'en';
}

class I18n {
	locale = $state<'en' | 'zh-TW'>(detect());

	t = (key: string): string => dicts[this.locale][key] ?? en[key] ?? key;

	toggle() {
		this.locale = this.locale === 'en' ? 'zh-TW' : 'en';
	}
}

export const i18n = new I18n();
