# CPA Route Control

直接连接 CLIProxyAPI 官方 Management API 的 Tauri 2 + Vue 3 桌面客户端，不再运行独立的 Route Control Node 服务。

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

## 桌面安装包

GitHub Actions 支持 Windows 和 macOS 两个平台的 Tauri 桌面构建：

- Windows x64：便携 `.exe` 与 NSIS 安装包；
- macOS：`.app` 与 `.dmg`。

触发方式：在 `Actions → Build Desktop Releases → Run workflow` 手动编译，或推送 `v*` 标签。

本地构建：

```powershell
# Windows
npm run tauri build -- --bundles nsis
```

```bash
# macOS
npm run tauri build -- --bundles app,dmg
```
