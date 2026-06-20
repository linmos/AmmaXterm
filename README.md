<h1 align="center">AmmaXterm</h1>

<p align="center">
  <b>A lightweight, open-source SSH terminal for everyone.</b><br>
  SSH, SFTP file transfer, saved sessions, and port forwarding — fast, simple, and secure.
</p>

<p align="center">
  <b>輕量、開源的 SSH 終端工具,人人都能上手。</b><br>
  SSH 連線、SFTP 傳檔、工作階段管理與連接埠轉發 —— 快速、簡單、安全。
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-blue" alt="Platforms">
  <img src="https://img.shields.io/badge/status-active%20development-brightgreen" alt="Status">
</p>

<p align="center"><b>English</b> · <a href="#正體中文">正體中文</a></p>

---

## English

AmmaXterm is a friendly desktop app for connecting to your servers over SSH. It
bundles everything you need for day-to-day remote work — a terminal, a file
manager, saved connections, and network tunnels — into one small, fast, and
secure window. No subscriptions, no account, no clutter.

### ✨ Why you'll like it

- ⚡ **Light and fast** — starts in an instant and stays out of your way. The
  download is small because it uses your system's built-in browser engine.
- 🖥️ **Works everywhere** — one app for **Windows, macOS, and Linux**.
- 🆓 **Free and open source** — MIT-licensed, no paywalls, no sign-up required.
- 🔒 **Secure by default** — your passwords live in your operating system's secure
  storage, and every server's identity is verified before you connect.

### 🚀 Features

**🖥️ Multi-tab terminal**
Open as many sessions as you like in tabs. Log in with a password, an SSH key, or
interactive prompts. Enjoy true color and full UTF-8, search your scrollback,
switch between light and dark themes (it can follow your system automatically),
pick your favourite font, and let sessions reconnect on their own if the network
drops. You can even record a session to a log file.

**📁 Built-in file transfer (SFTP)**
Browse remote files next to your own computer in a dual-pane view. Drag and drop
to upload, line up multiple transfers in a queue with live progress and speed,
and pause, resume, or auto-retry as needed. Sort and filter folders, change file
permissions, and let the file view follow along as you change directories in the
terminal.

**🗂️ Saved sessions**
Save your servers and organize them into groups you can drag to reorder. Find any
one of them instantly with search, or connect on the fly by typing
`user@host:port`. Already have an OpenSSH `config`? Import it in seconds. Back up
and restore your whole list anytime.

**🔀 Port forwarding & tunnels**
Set up local, remote, and dynamic (SOCKS5) tunnels, and watch them in a live
panel that shows active connections and traffic. Sensible defaults keep your
forwards private to your own machine unless you choose otherwise.

**🔐 Keys & secrets**
Generate Ed25519 or RSA key pairs right inside the app. Credentials are kept in
your OS keychain instead of plain text, and on systems without one, a built-in
encrypted vault (AES-256-GCM + Argon2id) keeps your secrets locked down.

### 📦 Get started

1. Head to the **[Releases page](https://github.com/linmos/AmmaXterm/releases)**.
2. Download the installer for your platform (Windows, macOS, or Linux).
3. Install, launch, and add your first server with the **＋** button — or click
   **⚡** to connect right away.

The version and other details are always available from the **ⓘ About** button in
the sidebar.

### 🛠️ Build from source

Prefer to build it yourself? You'll need **Rust** (stable), **Node.js 20+**,
**pnpm**, and your platform's
[Tauri prerequisites](https://tauri.app/start/prerequisites/).

```bash
corepack enable        # makes pnpm available (bundled with Node)
pnpm install
pnpm tauri dev         # run in development
pnpm tauri build       # produce an installer for your platform
```

### 🤝 Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) and our
[Code of Conduct](CODE_OF_CONDUCT.md). Please report security issues privately as
described in [SECURITY.md](SECURITY.md).

### 📄 License

[MIT](LICENSE) © AmmaXterm contributors. Third-party dependencies keep their own
licenses (Apache-2.0 / BSD / MIT); see their respective notices.

---

## 正體中文

AmmaXterm 是一款親切好用的桌面應用程式,讓你透過 SSH 連到自己的伺服器。它把日常
遠端工作需要的一切 —— 終端機、檔案管理、已存連線、網路通道 —— 全部整合在一個輕巧、
快速又安全的視窗裡。不必訂閱、不用註冊帳號、沒有多餘負擔。

### ✨ 你會喜歡的理由

- ⚡ **輕巧又快速** —— 開啟瞬間完成,不擋路。因為使用系統內建的瀏覽器引擎,安裝
  檔也特別小。
- 🖥️ **到處都能用** —— 一套程式同時支援 **Windows、macOS 與 Linux**。
- 🆓 **免費且開源** —— 採用 MIT 授權,沒有付費牆,也不需要註冊。
- 🔒 **預設就安全** —— 你的密碼存放在作業系統的安全儲存區,每次連線前都會先驗證
  伺服器身分。

### 🚀 功能特色

**🖥️ 多分頁終端機**
想開幾個工作階段都行,通通收在分頁裡。可用密碼、SSH 金鑰或互動式提示登入。支援
真彩色與完整 UTF-8、可搜尋捲動紀錄、在亮色與暗色佈景間切換(還能自動跟隨系統)、
挑選喜歡的字型,網路中斷時也能自動重新連線。甚至可以把工作階段記錄成檔案。

**📁 內建檔案傳輸(SFTP)**
以雙窗格檢視,把遠端檔案與本機並排瀏覽。拖放即可上傳,多個傳輸排入佇列並即時顯示
進度與速度,還能暫停、繼續或自動重試。可排序與篩選資料夾、變更檔案權限,檔案檢視
也能跟著你在終端機切換的目錄走。

**🗂️ 工作階段管理**
儲存你的伺服器,並整理成可拖曳排序的群組。用搜尋立刻找到任何一台,或直接輸入
`user@host:port` 快速連線。已經有 OpenSSH `config`?幾秒就能匯入。整份清單隨時都能
備份與還原。

**🔀 連接埠轉發與通道**
建立本機、遠端與動態(SOCKS5)通道,並在即時面板中查看作用中的連線與流量。預設值
會把轉發限制在自己的電腦上,除非你自行調整,兼顧便利與安全。

**🔐 金鑰與機密**
直接在程式內產生 Ed25519 或 RSA 金鑰對。帳密會存放在作業系統的金鑰圈,而非明文;
在沒有金鑰圈的系統上,內建的加密保險庫(AES-256-GCM + Argon2id)也能牢牢守住你的
機密。

### 📦 開始使用

1. 前往 **[Releases 頁面](https://github.com/linmos/AmmaXterm/releases)**。
2. 下載對應你平台的安裝檔(Windows、macOS 或 Linux)。
3. 安裝、開啟,按 **＋** 新增第一台伺服器 —— 或按 **⚡** 立即連線。

版本與其他資訊都能從側邊欄的 **ⓘ 關於** 按鈕查看。

### 🛠️ 從原始碼建置

想自己建置?你需要 **Rust**(stable)、**Node.js 20+**、**pnpm**,以及你平台的
[Tauri 環境需求](https://tauri.app/start/prerequisites/)。

```bash
corepack enable        # 啟用 pnpm(隨 Node 內附)
pnpm install
pnpm tauri dev         # 開發模式執行
pnpm tauri build       # 產生你平台的安裝檔
```

### 🤝 參與貢獻

歡迎貢獻!請參閱 [CONTRIBUTING.md](CONTRIBUTING.md) 與
[行為準則](CODE_OF_CONDUCT.md)。資安問題請依
[SECURITY.md](SECURITY.md) 的說明私下回報。

### 📄 授權

[MIT](LICENSE) © AmmaXterm 貢獻者。第三方相依套件各自保留其授權
(Apache-2.0 / BSD / MIT),詳見各自的授權聲明。
