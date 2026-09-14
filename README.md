# CPA Route Control

基于 Tauri 2 + Vue 3 的 CLIProxyAPI 管理客户端，支持模型别名、优先级、权重和快捷策略管理。

## 本地调试

```powershell
npm install
npm run dev
```

打开 `http://localhost:1420`，输入：

```text
CPA Management URL: http://127.0.0.1:8317/v0/management
CPA 明文管理密钥: config.yaml 被 bcrypt 加密前的原始密钥
```

浏览器调试不持久化密钥；Tauri 勾选“记住密钥”后使用 Windows Credential Manager 安全保存。浏览器通过 Vite 代理访问本地 CPA，Tauri 使用 Rust `reqwest` 直接访问 CPA。

## 桌面端自动连接

桌面应用启动时可从主程序 EXE 同目录的 `.env`，或从启动进程继承的系统/用户环境变量读取 CPA 连接信息：

```dotenv
CPA_MANAGEMENT_URL=http://127.0.0.1:8317/v0/management
CPA_MANAGEMENT_KEY=your-plaintext-management-key
```

两项必须在同一来源中同时配置且不能为空。读取优先级为：

```text
EXE 同目录 .env > 系统/进程环境变量 > Windows Credential Manager > 手工输入
```

- 安装版将 `.env` 放在已安装的 `CPA Route Control` 主程序旁；便携版放在便携 EXE 旁。
- `.env` 不会被打进安装包，并已加入 `.gitignore`。它包含明文管理密钥，请限制文件访问权限，不要提交到 Git。
- 自动配置存在但格式错误、字段不完整或连接失败时，应用会停留在连接页，不再尝试低优先级来源。
- 自动连接只作用于 CPA Route Control。系统浏览器中打开的官方 `/management.html` 仍使用自己的登录状态。
- CPA 没有独立的管理账号字段；这里的“登录信息”是 Management URL 和 bcrypt 加密前的明文 Management Key。
- 普通浏览器页面无法读取本机环境变量或 EXE 目录文件，因此该能力仅在 Tauri 桌面端生效。

## 已接入的官方接口

- `GET /config`：读取当前运行时配置；
- `GET /<provider>-api-key`：读取 Provider Credential 数组；
- `PUT /<provider>-api-key`：更新 Weight/Priority 并由 CPA 写回 YAML、热重载；
- `GET/PUT /openai-compatibility`：读取和更新 OpenAI 兼容提供商。
- `GET /auth-files`：读取 OAuth/JSON 授权文件运行时状态；
- `PATCH /auth-files/fields`：按文件名或 auth ID 更新授权文件的 Weight/Priority。

当前支持识别 `claude-api-key`、`gemini-api-key`、`codex-api-key`、`xai-api-key`、`vertex-api-key`、`interactions-api-key` 和 `openai-compatibility`。

一键分配按钮根据 CPA 当前 Provider 动态生成，不绑定固定模型。用户可新增自定义权重策略；策略只保存 Credential 标识、显示名称和 Weight，不保存任何 API Key。若 CPA 删除了策略引用的 Credential，客户端会拒绝应用并提示具体缺失项。

授权文件（OAuth/JSON）与配置型 API Key 会统一显示在权重池中。授权文件使用 CPA 返回的 `auth_index` 作为稳定 ID，写入时按官方要求发送 `PATCH /auth-files/fields`，不会改写授权文件内容。

## 配置远程 CPA 网页注意事项

- 连接时填写管理接口地址，例如 `https://cpa.example.com/v0/management`；内嵌网页访问同一站点的 `/management.html`。
- 远程 CPA 需启用远程管理，并使用明文管理密钥登录。建议使用 HTTPS，不要将密钥放入 URL。
- EXE 需要在 `src-tauri/tauri.conf.json` 的 `app.security.csp` 中，将目标站点加入 `frame-src`。当前已配置 `https://cpa.10085.fun`；其他站点需追加对应协议、域名及非默认端口，保留其余安全策略。
- 修改上述配置后需重新打包并安装新版 EXE。
- 目标站点需允许网页被嵌入；如果站点限制嵌入，请在外部浏览器访问。
- 内嵌管理台需单独登录，可使用侧栏复制图标复制当前管理密钥。

## 桌面安装包

GitHub Actions 构建 Windows x64 安装包：

- Windows x64：便携 `.exe` 与 NSIS 安装包。

触发方式：推送到 `master` 或推送 `v*` 标签；也可在 Actions 中手动运行。

本地构建：

```powershell
npm run tauri build -- --bundles nsis
```
