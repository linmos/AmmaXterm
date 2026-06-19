# AmmaXterm 開發計畫（Development Plan）

| 項目 | 內容 |
|---|---|
| 對應 PRD | PRD-AmmaXterm-v0.5（2026-06-19 定稿） |
| 文件狀態 | 初版開發規劃 |
| 日期 | 2026-06-19 |
| 技術主軸 | Tauri v2 + Rust（russh）+ Svelte 5 + xterm.js |

> 本文件把 PRD 的決策（D1~D10）、里程碑（M0~M3）與需求編號（SM/TM/FT/PF/AK/ST）展開成可執行的工作項，並補上 PRD 未涵蓋的工程層決策。每個任務標註對應需求、規模（S/M/L）與相依，便於排程與追溯。

---

## 1. 目的與現況

### 1.1 目的
將 PRD 轉為可排程、可分工、可追溯的開發工作清單，明確：前置環境、補充技術決策、專案結構、里程碑任務分解、跨切面工作（安全 / 測試 / CI）、關鍵路徑與待確認事項。

### 1.2 專案現況（2026-06-19 實測）
- **儲存庫**：已是 git repo，但**尚無任何 commit**、無 remote；目前僅有 `docs/PRD-AmmaXterm-v0.5.md`。
- **工具鏈**：
  - Node `v24.17.0` ✓、npm `11.13.0` ✓
  - **Rust / Cargo：未安裝 ✗（阻塞項，最優先處理）**
  - Tauri CLI：未安裝 ✗
  - pnpm：未安裝（非必要）

> 結論：這是 greenfield 專案。第一件事是補齊 Rust 工具鏈與儲存庫治理，再進 M0 技術驗證。

---

## 2. 前置作業（Sprint 0：環境與儲存庫）

### 2.1 工具鏈安裝

| ID | 任務 | 規模 | 備註 |
|---|---|---|---|
| S0-1 | 安裝 Rust 工具鏈（rustup / stable） | S | **阻塞所有後端工作** |
| S0-2 | 安裝各平台 Tauri v2 前置需求 | S | Windows：WebView2 Runtime + MSVC Build Tools；Linux：`webkit2gtk`、`libssl`、`librsvg` 等；macOS：Xcode CLT |
| S0-3 | 安裝 Tauri CLI（`cargo install tauri-cli` 或 `@tauri-apps/cli`） | S | — |
| S0-4 | （選用）安裝 pnpm 作為前端套件管理 | S | npm 亦可，pnpm 較省碟/快 |
| S0-5 | `cargo tauri info` 驗證三項環境齊備 | S | 環境檢查通過即可開工 |

### 2.2 儲存庫治理（對應 PRD §9.2、§6.1）

| ID | 任務 | 對應 | 規模 |
|---|---|---|---|
| S0-6 | `.gitignore`（Rust `target/` + Node `node_modules/` + Tauri 產物） | — | S |
| S0-7 | `LICENSE`：MIT | §9.1 / D6 | S |
| S0-8 | `README`（專案簡介、從原始碼建置說明） | §9.2 | S |
| S0-9 | `CONTRIBUTING` / `CODE_OF_CONDUCT` | §9.2 | S |
| S0-10 | `SECURITY.md`（漏洞通報流程） | §6.1 | S |
| S0-11 | issue / PR 範本、`CHANGELOG`（semver） | §9.2 | S |
| S0-12 | 第三方授權檢查機制（`cargo-deny` 或 `cargo-about`） | §9.1 / 風險 | S |
| S0-13 | 首次 commit + 建立 GitHub 公開儲存庫與 remote | §9.2 | S |

---

## 3. 補充技術決策（PRD D1~D10 之外的工程決策）

> 以下為驅動實作所需、但 PRD 未明定的決策。標「建議」為預設方向，列於 §8 待你確認。

| 決策點 | 建議方案 | 理由 |
|---|---|---|
| Svelte 形態 | **SvelteKit 2 + Svelte 5（runes）+ adapter-static（SPA）+ TypeScript** ✅ 已採用 | Tauri 官方維護模板；static adapter 編譯為純前端 SPA（無 SSR server、執行期輕量），檔案式路由利於多視圖組織 |
| 前端套件管理 | pnpm（npm 亦可） | 速度/碟用較佳 |
| 終端套件 | `@xterm/xterm` + `addon-webgl` + `addon-fit` + `addon-search` + `addon-unicode11` | WebGL 應付高吞吐（§6.2）；search 對應 TM-10；unicode11 修正 UTF-8 寬度（TM-3） |
| 終端串流 IPC | **Tauri v2 `Channel`（位元組串流）**，非 event | Channel 為串流設計，吞吐/延遲優於 event，契合 §6.2 < 50ms |
| Session 模型 | **actor-per-session**：每 session 一個 tokio 任務 + mpsc 指令通道，輸出走 Channel | 單一 session/通道失敗不影響其他（§6.4） |
| 設定檔格式 | JSON（`sites.json` / `settings.json`），含 `schema_version` 欄位 | 易演進、利於未來同步（A4） |
| 憑證儲存 | `keyring` crate；站台檔只存「金鑰庫參考鍵」 | **絕不落地明文/密文**（§6.1、AK-1） |
| 記憶體衛生 | `zeroize` 清除密碼/私鑰 | §6.1「使用後盡速清除」 |
| known_hosts | 維護 app 私有 known_hosts（OpenSSH 格式），可選讀系統檔 | 與系統 SSH 互通（§8.3） |
| 錯誤模型 | 後端 `thiserror` 分類錯誤（DNS/逾時/認證/金鑰不符/通道） | 對應 §6.4 清楚錯誤訊息 |
| i18n | **第一天就外部化字串**（即使完整多語系為 P2/M3） | 避免後期大規模回頭重工（§6.5） |
| Tauri 安全 | 啟用 capabilities/permissions，最小化開放的 command 介面 | 降低 IPC 攻擊面 |
| 金鑰演算法庫 | `russh-keys`（產生/解析 Ed25519、RSA） | 對應 AK-2、AK-3 |
| 本機庫加密（無 OS 金鑰庫時） | `aes-gcm`（AES-256-GCM）+ Argon2 KDF | 對應 AK-4、§6.1 |

---

## 4. 建議專案結構

```
AmmaXterm/
├─ docs/                       # PRD、開發計畫、設計文件
├─ src/                        # Svelte 前端
│  ├─ lib/
│  │  ├─ components/           # 版面、站台清單、SFTP 面板、通道面板…
│  │  ├─ terminal/            # xterm.js 封裝、addon 設定
│  │  ├─ stores/              # 前端狀態（session、sites、settings）
│  │  ├─ ipc/                 # 呼叫 Rust command 的型別化封裝
│  │  └─ i18n/                # 語系字串（zh-TW / en）
│  ├─ App.svelte
│  └─ main.ts
├─ src-tauri/                  # Rust 後端
│  ├─ src/
│  │  ├─ ssh/                 # russh 連線、PTY、認證、known_hosts
│  │  ├─ sftp/                # russh-sftp 操作
│  │  ├─ tunnel/             # 連接埠轉發（L/R/D），PF-7 安全預設
│  │  ├─ session/            # actor-per-session 管理、registry
│  │  ├─ store/              # 站台/設定資料模型與持久化
│  │  ├─ secrets/            # keyring 整合、本機庫加密
│  │  ├─ commands.rs         # Tauri command 入口
│  │  └─ error.rs            # 分類錯誤
│  ├─ capabilities/          # Tauri 權限設定
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ .github/workflows/         # CI、tauri-action 發布
├─ LICENSE  README  CONTRIBUTING  SECURITY.md  CHANGELOG
└─ package.json
```

---

## 5. 里程碑與任務分解

### M0 — 技術驗證（Spike：打通核心鏈路）

> 目標：以最少程式碼證明 russh + Tauri + xterm 整條鏈路可行，**降低技術風險**。可硬編碼參數、不追求 UI。

| ID | 任務 | 對應 | 規模 | 相依 |
|---|---|---|---|---|
| M0-1 | Tauri v2 + Vite + Svelte 5 + TS 專案骨架 | §8.1 | S | S0-* |
| M0-2 | 整合 xterm.js（webgl/fit）渲染靜態終端 | TM-3 | S | M0-1 |
| M0-3 | russh 建立單一 SSH 連線（密碼）並開 PTY shell | TM-1, TM-2 | M | S0-1 |
| M0-4 | 串流打通：SSH channel ↔ Tauri Channel ↔ xterm（輸入/輸出雙向） | TM-1 | M | M0-2, M0-3 |
| M0-5 | 視窗 resize → window-change 同步遠端尺寸 | TM-5 | S | M0-4 |
| M0-6 | 主機金鑰驗證 handler + 寫入 known_hosts | TM-6 | M | M0-3 |
| M0-7 | russh-sftp：列目錄 + 上傳 + 下載單檔 | FT-1, FT-2 | M | M0-3 |
| M0-8 | Spike 結論與決策紀錄（russh 是否滿足；備案 `ssh2`） | §8.2 | S | M0-4, M0-7 |

**M0 退出條件**：能 SSH 連真實主機、取得互動 shell（vim / top 操作順暢、UTF-8/色彩正確）、成功傳輸一個檔案。

---

### M1 — MVP（所有 P0；目標：可日常使用 + 首個 Release）

**後端（Rust）**

| ID | 任務 | 對應 | 規模 | 相依 |
|---|---|---|---|---|
| M1-B1 | 站台資料模型 + 本機儲存（`sites.json`、schema 版本、CRUD） | SM-1 | M | M0 |
| M1-B2 | keyring 整合：密碼/passphrase 安全儲存（不落地明文） | AK-1 | M | — |
| M1-B3 | 私鑰管理：匯入金鑰、每站台指定金鑰、passphrase 解鎖 | AK-2 | M | M1-B2 |
| M1-B4 | 認證三法落地：密碼 / 公鑰 / keyboard-interactive | TM-2 | M | M1-B3 |
| M1-B5 | 連線生命週期：actor-per-session、狀態事件、故障隔離 | SM-2, §6.4 | L | M0-4 |
| M1-B6 | 多 session registry（對應多分頁） | TM-7 | M | M1-B5 |
| M1-B7 | SFTP 操作：mkdir / rename / delete / move（遞迴） | FT-3 | M | M0-7 |
| M1-B8 | 錯誤分類與訊息（DNS/逾時/認證/金鑰不符） | §6.4 | M | M1-B5 |
| M1-B9 | 主機金鑰 UX 後端：首次指紋確認、變更警示流程 | TM-6 | M | M0-6 |

**前端（Svelte）**

| ID | 任務 | 對應 | 規模 | 相依 |
|---|---|---|---|---|
| M1-F1 | 三區式版面骨架（側欄 / 工作區 / 面板） | §7.1 | M | M0-1 |
| M1-F2 | 站台清單 + 新增/編輯/刪除對話框（基本頁） | SM-1, SM-2 | M | M1-B1 |
| M1-F3 | 連線狀態指示（已連 / 未連 / 失敗） | SM-2 | S | M1-B5 |
| M1-F4 | 終端機強化：256/True Color、UTF-8、控制序列 | TM-3 | M | M0-2 |
| M1-F5 | 複製 / 貼上 / 選取（右鍵 + 快捷鍵） | TM-4 | S | M0-2 |
| M1-F6 | 多分頁 UI（重新命名、拖曳排序） | TM-7 | M | M1-B6 |
| M1-F7 | SFTP 面板（路徑列 + 檔案表格 + 上傳/下載/操作） | FT-1~3 | L | M1-B7 |
| M1-F8 | 主機金鑰指紋對話框 + 變更警示 | TM-6 | S | M1-B9 |
| M1-F9 | 破壞性操作二次確認（刪站台 / 刪遠端檔） | §6.4 | S | — |
| M1-F10 | i18n 框架接入 + 繁中/英文基礎字串 | §6.5 | S | M0-1 |

**基礎建設 / CI**

| ID | 任務 | 對應 | 規模 |
|---|---|---|---|
| M1-I1 | GitHub Actions PR 檢查（cargo fmt/clippy/test、前端 lint/test） | §9.3 | M |
| M1-I2 | `tauri-action` 三平台建置 + 首個 GitHub Release | §9.3 / D8 | M |
| M1-I3 | `cargo-deny` 授權與安全稽核納入 CI | §9.1 / 風險 | S |

**M1 退出條件**：P0 全數完成、可日常使用（站台 CRUD、三法認證、多分頁終端、SFTP 基本操作、主機金鑰驗證、安全儲存），三平台 CI 綠燈並產出首個 Release。

---

### M2 — v1.0（主要 P1；目標：比 MobaXterm 輕但夠用）

**站台管理**

| ID | 任務 | 對應 |
|---|---|---|
| M2-1 | 分組 / 資料夾 + 拖曳搬移 | SM-3 |
| M2-2 | 即時搜尋 / 過濾（名稱/host/標籤） | SM-4 |
| M2-3 | 快速連線（`user@host:port`） | SM-5 |
| M2-4 | 站台層級覆寫設定（金鑰/跳板/通道/編碼/字型/keepalive） | SM-6 |
| M2-5 | 匯入 OpenSSH `~/.ssh/config` / PuTTY session | SM-7 |
| M2-6 | 匯出 / 備份（憑證加密或排除） | SM-8 |

**終端機**

| ID | 任務 | 對應 |
|---|---|---|
| M2-7 | 斷線自動重連 + keepalive | TM-8 |
| M2-8 | 跳板機 ProxyJump（多層） | TM-9 |
| M2-9 | 終端輸出內搜尋 | TM-10 |
| M2-10 | 字型/字級/主題（深淺+自訂）/捲動緩衝 + 全域預設 | TM-11, ST-1, ST-2 |
| M2-11 | Session logging（輸出存檔） | TM-12 |

**SFTP**

| ID | 任務 | 對應 |
|---|---|---|
| M2-12 | 傳輸佇列（進度/速度/剩餘，暫停/取消/重試） | FT-4 |
| M2-13 | 拖放傳輸（本機 ↔ 面板、面板內移動） | FT-5 |
| M2-14 | SFTP 面板跟隨終端 cd（可開關） | FT-6 |
| M2-15 | 斷點續傳 / 失敗自動重試 | FT-7 |
| M2-16 | chmod + 檢視擁有者/群組 | FT-8 |
| M2-17 | 大目錄虛擬化捲動 + 排序/過濾 | FT-9 |
| M2-18 | 雙窗模式（本機 / 遠端並排） | FT-10 |

**連接埠轉發 / 通道（D9：本期 v1.0；PF-7 安全預設第一版即須正確）**

| ID | 任務 | 對應 |
|---|---|---|
| M2-19 | 本機轉發 Local `-L` | PF-1 |
| M2-20 | 動態轉發 SOCKS5 `-D` | PF-2 |
| M2-21 | 遠端轉發 `-R`（**預設關閉 + 內網暴露警示**） | PF-3 |
| M2-22 | 通道存於站台、連線時自動建立、可逐條啟停 | PF-4 |
| M2-23 | 通道管理面板（類型/位址/狀態/連線數/流量） | PF-5, §7.2 |
| M2-24 | 通道錯誤清楚提示（占用/不可達/權限/拒絕） | PF-6 |
| M2-25 | **★ 安全預設：本機監聽綁 `127.0.0.1`、不綁 `0.0.0.0`** | PF-7（P0，貫穿 PF-1~3） |

**金鑰**

| ID | 任務 | 對應 |
|---|---|---|
| M2-26 | 金鑰產生器 Ed25519/RSA + 一鍵複製公鑰 | AK-3 |
| M2-27 | 主鑰加密本機憑證庫（未用 OS 金鑰庫時） | AK-4 |

**M2 退出條件**：上述 P1 完成；通道功能在 PF-7 安全預設正確下上線；達「比 MobaXterm 輕但夠用」。

---

### M3 — v1.x（P2；體驗強化）

| 項目 | 對應 |
|---|---|
| 分割窗（多終端並排） | TM-13 |
| SSH agent forwarding | TM-14 |
| 廣播輸入（多 session 同步指令） | TM-15 |
| edit-in-place（本機編輯器開遠端檔回寫） | FT-11 |
| 衝突處理策略（覆寫/略過/兩者保留/比對） | FT-12 |
| 多執行緒並行傳輸 | FT-13 |
| 站台標籤 + 顏色標記 | SM-9 |
| 複製站台、最近使用清單 | SM-10 |
| 公鑰部署到遠端（ssh-copy-id 等效） | AK-5 |
| 密碼貼上後剪貼簿自動清除 | AK-6 |
| 快捷鍵檢視與自訂 | ST-3 |
| 完整多國語系 | ST-4 |

---

## 6. 跨切面工作

### 6.1 安全（最高優先，§6.1）
- 憑證一律走 OS 金鑰庫；本機庫（若有）AES-256-GCM + Argon2 加密。
- 強制主機金鑰驗證；指紋變更明顯警示並要求確認。
- 偏好現代演算法（Ed25519、curve25519-sha256、AES-GCM、ChaCha20-Poly1305），相容舊裝置為可選開關。
- **PF-7 自第一版通道功能即正確**：本機綁 `127.0.0.1`、Remote 預設關閉並警示。
- 預設零遙測；密碼/私鑰用 `zeroize` 清除。
- Tauri capabilities 最小化開放 command。

### 6.2 測試策略
- **後端**：`cargo test` 單元測試；以 Docker 化 OpenSSH 容器跑整合測試（連線/認證/SFTP/通道）。
- **前端**：Vitest 元件測試。
- **相容性清單**：vim / tmux / htop / fish / zsh 控制序列驗證（對應風險）。
- **E2E**：後續導入 `tauri-driver`（WebDriver）。
- **效能基準**：終端延遲 < 50ms、大檔（>1GB）傳輸成功率 > 99%（§11）。

### 6.3 CI/CD（§9.3）
- PR：fmt + clippy + test（Rust）、lint + test（前端）、`cargo-deny`（授權/CVE）。
- Release：`tauri-action` 三平台產出 `.msi/.exe`、`.dmg/.app`、`.deb/.AppImage`。
- 自三平台第一天就跑 CI（**及早暴露 Linux WebKitGTK 差異**，對應風險）。

### 6.4 文件與在地化
- 「從原始碼建置」說明（含 Rust toolchain、Tauri 前置）降低貢獻門檻。
- i18n 基礎建設於 M1 接入；完整語系於 M3。
- 維護第三方授權聲明（NOTICE / licenses 頁）。

---

## 7. 關鍵路徑與相依

```
S0-1(安裝 Rust) ─┬─► M0-3(SSH+PTY) ─► M0-4(串流核心) ─► M1-B5(session 生命週期) ─► M1-B6(多分頁)
                 └─► M0-1(骨架) ─► M0-2(xterm) ─┘                                   │
                                                                                    ▼
M1-B2(keyring) ─► M1-B3(私鑰) ─► M1-B4(三法認證)                          M1-F6(多分頁 UI)
M1-B1(資料模型) ─► M1-F2(站台 UI)
PF 功能（M2-19~24）必須與 M2-25(PF-7 安全預設) 一起落地
```

- **最關鍵阻塞**：`S0-1` 安裝 Rust——未完成則所有後端工作無法開始。
- **核心技術風險**：`M0-4` 串流核心（russh ↔ Tauri Channel ↔ xterm）——M0 即須驗證。
- **必須同生**：任何 PF 程式碼（M2-19~24）不得早於 / 脫離 PF-7 安全預設（M2-25）。
- **及早奠基（難回頭重工）**：資料模型（M1-B1，預留同步）、安全架構（§6.1）、i18n 字串外部化（M1-F10）。

---

## 8. 待確認的技術決策

| # | 決策 | 建議（預設） | 影響 |
|---|---|---|---|
| ~~Q1~~ ✅ | Svelte 形態 | **已定案：SvelteKit 2 + Svelte 5 + adapter-static** | 採官方 Tauri 模板，骨架已建立 |
| Q2 | 前端套件管理 | pnpm（npm 亦可） | CI 與本機開發 |
| Q3 | 設定檔格式 | JSON + `schema_version` | 持久化與未來同步 |
| Q4 | known_hosts 策略 | app 私有檔（可選讀系統檔） | 與系統 SSH 互通程度 |
| Q5 | GitHub 儲存庫 | 立即建立公開 repo + remote | 發布與協作流程 |

> 若無異議，將以「建議」欄為預設推進。

---

## 9. 近期建議行動（接下來 1~2 步）

1. **解除阻塞**：安裝 Rust 工具鏈 + Tauri 三平台前置需求（S0-1~S0-5）。
2. **儲存庫治理**：建立 `.gitignore`、`LICENSE(MIT)`、`README`、`SECURITY.md` 等並首次 commit（S0-6~S0-13）。
3. **M0 骨架**：scaffold Tauri v2 + Vite + Svelte 5 + xterm（M0-1、M0-2）。
4. **M0 串流核心**：打通 russh ↔ Tauri Channel ↔ xterm（M0-3、M0-4）——本專案最關鍵的技術驗證。
