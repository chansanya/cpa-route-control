# CPA 动态路由快捷控制系统开发说明书

**项目代号：CPA Route Control**
**版本：V1.0**
**目标平台：Windows 优先，兼容 macOS / Linux**
**客户端技术：Tauri 2 + Vue 3 + Vite + JavaScript**
**服务端增强：CLIProxyAPI 独立 Web 增强插件（Go）**

---

## 1. 项目背景

当前 CPA（CLIProxyAPI）已经具备模型路由、凭据优先级、权重等基础能力，但日常运维中存在以下问题：

1. 经常需要临时调整某个模型别名的真实目标模型。
2. 经常需要在多个 Key / 账号之间动态修改权重。
3. 需要按照时间段自动切换权重，例如：
   - 上午 Key S 权重大；
   - 下午 Key Y 权重大；
   - 晚上恢复均衡。
4. 需要保留一个稳定的客户端模型名，例如始终使用：

```text
model = code
```

而内部实际可以动态变化：

```text
code -> GLM
code -> Kimi
code -> GPT
code -> Claude
```

5. 希望在 Windows 桌面直接快速切换，而不是每次打开 CPA 管理网页。
6. 同时希望 CPA 本身具备一个独立的 Web 增强管理插件，即使 Tauri 客户端关闭，也能继续执行定时策略。

因此本项目设计为：

```text
┌──────────────────────────────┐
│       Tauri Desktop UI       │
│  快捷切换 / 托盘 / 状态查看 │
└──────────────┬───────────────┘
               │ HTTPS / Management API
               ▼
┌──────────────────────────────┐
│     cpa-route-control        │
│     CPA 独立增强插件          │
│                              │
│ Alias / Profile / Schedule   │
│ Weight / Priority / Audit    │
│ Snapshot / Rollback          │
└──────────────┬───────────────┘
               │
       ┌───────┴────────┐
       ▼                ▼
  CLIProxyAPI       cpa-key-policy
  Credential        Alias / Group
  Weight/Priority   Mapping
       │                │
       └───────┬────────┘
               ▼
        实际上游模型 / Key
```

核心原则：

> **Tauri 只负责快捷控制，真正的策略、定时任务和状态都放在 CPA 插件端。**

因此即使关闭 Tauri：

```text
08:00 -> 自动切换上午策略
12:00 -> 自动切换下午策略
18:00 -> 自动切换晚上策略
```

仍然正常执行。

---

# 2. 项目目标

## 2.1 第一阶段目标

V1 必须实现：

- CPA 多实例连接管理；
- 实例在线状态检测；
- 获取模型别名；
- 获取 CPA Credential / Key；
- 查看 Priority；
- 查看 Weight；
- 动态修改 Weight；
- 动态修改 Priority；
- 模型别名 `code` 快捷切换；
- 一键应用预设策略；
- 定时策略；
- 手动覆盖定时策略；
- 自动恢复；
- 操作日志；
- 配置快照；
- 一键回滚；
- Tauri 系统托盘；
- CPA 独立 Web 管理页面。

---

# 3. 非目标

V1 暂不实现：

- CPA 本身完整管理后台替代品；
- OAuth 登录管理；
- API 用量计费系统；
- 复杂监控平台；
- 用户权限 RBAC；
- 多租户；
- 自动购买 Key；
- 自动注册第三方账号。

这些功能后续可作为 V2/V3 扩展。

---

# 4. 核心使用场景

## 4.1 模型快捷切换

客户端始终调用：

```json
{
  "model": "code"
}
```

上午：

```text
code
  -> GLM
     Key A
```

下午：

```text
code
  -> Kimi
     Key B
```

客户端不需要修改任何配置。

---

## 4.2 多 Key 权重

例如：

```text
Key S
Key Y
```

上午：

```text
Key S = 80
Key Y = 20
```

下午：

```text
Key S = 20
Key Y = 80
```

晚上：

```text
Key S = 50
Key Y = 50
```

---

## 4.3 强制切换

如果希望所有新请求全部走 Key A：

```text
Key A weight = 100
Key B weight = 0
```

切换到 Key B：

```text
Key A weight = 0
Key B weight = 100
```

V1 UI 中应提供：

```text
[全部 GLM]
[全部 Kimi]
[50 / 50]
[上午策略]
[下午策略]
```

而不是要求用户手工输入数字。

---

# 5. 路由原则

CPA 当前路由逻辑应按以下思路理解：

```text
第一层：
Priority

第二层：
Weight
```

即：

```text
最高可用 Priority
        │
        ▼
同 Priority Credential Pool
        │
        ▼
根据 Weight 分配
```

示例：

```text
A:
priority = 100
weight = 80

B:
priority = 100
weight = 20
```

则 A、B 在同一个池内按权重参与新请求分配。

如果：

```text
A:
priority = 100
weight = 1

B:
priority = 90
weight = 100
```

只要 A 可用，则 B 不应该因为 Weight 更高而越级抢占 A。

因此快捷策略页面必须把 Priority 和 Weight 分开显示。

---

# 6. 总体架构

## 6.1 组件

系统拆分为三个独立组件：

### A. CLIProxyAPI

原始 CPA 服务。

职责：

- 提供实际 API；
- 管理 Credential；
- 执行模型调用；
- 根据 Priority / Weight 选择 Credential。

---

### B. cpa-route-control

独立 CPA 插件。

职责：

- 提供增强 Web UI；
- 提供 Tauri 调用 API；
- 管理 Profile；
- 管理 Schedule；
- 执行自动切换；
- Alias 快捷切换；
- Credential 权重控制；
- Priority 控制；
- 快照；
- 回滚；
- 操作日志；
- 与 `cpa-key-policy` 对接。

---

### C. cpa-route-control-desktop

Tauri 桌面程序。

职责：

- 快捷控制；
- 系统托盘；
- 当前状态；
- 手动策略切换；
- 管理多个 CPA；
- 查看最近操作；
- 不负责长期调度。

---

# 7. cpa-route-control 插件设计

## 7.1 插件名称

```text
cpa-route-control
```

建议目录：

```text
plugins/
└── cpa-route-control_linux_amd64.so
```

配置：

```yaml
plugins:
  enabled: true
  dir: "plugins"

  configs:
    cpa-route-control:
      enabled: true
      priority: 20
      state_file: "cpa-route-control-state.json"
```

---

# 8. 插件 Web UI

插件必须提供独立 Web 页面。

建议地址：

```text
/v0/resource/plugins/cpa-route-control/index.html
```

管理 API：

```text
/v0/management/plugins/cpa-route-control/*
```

Web UI 不依赖 Tauri。

也就是说：

```text
浏览器
   │
   ▼
CPA Web Plugin
```

即可完成所有操作。

---

# 9. Web UI 页面设计

## 9.1 Dashboard

显示：

```text
CPA Route Control
────────────────────────

CPA           ● Online
Scheduler     ● Running

当前 Profile
下午 Kimi 优先

当前 Alias
code -> kimi-k2.5

下一次切换
18:00 -> 晚间均衡

Credential

GLM-A    ████████░░ 20
Kimi-B   █████████████████ 80
```

快捷按钮：

```text
[上午]
[下午]
[晚上]
[GLM 100%]
[Kimi 100%]
[恢复自动]
```

---

# 10. Profile 策略

Profile 是整个系统最重要的业务对象。

例如：

```json
{
  "id": "morning",
  "name": "上午策略",
  "aliases": [
    {
      "alias": "code",
      "provider": "glm",
      "model": "glm-5"
    }
  ],
  "credentials": [
    {
      "credential_id": "glm-a",
      "priority": 100,
      "weight": 80
    },
    {
      "credential_id": "kimi-b",
      "priority": 100,
      "weight": 20
    }
  ]
}
```

下午：

```json
{
  "id": "afternoon",
  "name": "下午策略",
  "aliases": [
    {
      "alias": "code",
      "provider": "kimi",
      "model": "kimi-k2.5"
    }
  ],
  "credentials": [
    {
      "credential_id": "glm-a",
      "priority": 100,
      "weight": 20
    },
    {
      "credential_id": "kimi-b",
      "priority": 100,
      "weight": 80
    }
  ]
}
```

---

# 11. Profile 类型

支持三类。

## 11.1 完整 Profile

同时控制：

- Alias；
- Priority；
- Weight。

---

## 11.2 Weight Profile

只修改：

```text
Weight
```

不修改 Alias。

适用于：

```text
code 同时允许 GLM + Kimi
只调整两个 Key 的流量比例
```

---

## 11.3 Alias Profile

只修改：

```text
code -> 实际模型
```

不修改 Credential 权重。

---

# 12. Schedule 定时任务

示例：

```json
{
  "id": "weekday-morning",
  "name": "工作日上午",
  "enabled": true,
  "cron": "0 8 * * 1-5",
  "profile_id": "morning",
  "timezone": "Asia/Shanghai"
}
```

下午：

```json
{
  "id": "weekday-afternoon",
  "name": "工作日下午",
  "enabled": true,
  "cron": "0 12 * * 1-5",
  "profile_id": "afternoon",
  "timezone": "Asia/Shanghai"
}
```

晚间：

```json
{
  "id": "evening",
  "name": "晚间",
  "enabled": true,
  "cron": "0 18 * * *",
  "profile_id": "evening"
}
```

---

# 13. Manual Override 手动覆盖

必须提供手动覆盖机制。

例如系统当前为：

```text
自动策略：下午 Kimi 80
```

用户临时点击：

```text
GLM 100%
```

弹出：

```text
覆盖时间

○ 直到下一次定时策略
○ 30 分钟
○ 1 小时
○ 2 小时
○ 今天结束
○ 一直保持
```

推荐默认：

```text
直到下一次定时策略
```

数据结构：

```json
{
  "active": true,
  "profile_id": "force-glm",
  "mode": "until_next_schedule",
  "expires_at": null
}
```

---

# 14. Alias 管理

插件需要提供 Alias 页面。

例如：

```text
Alias: code

当前：
Kimi / kimi-k2.5

可选目标：

○ GLM     glm-5
● Kimi    kimi-k2.5
○ OpenAI  gpt-5.6-sol
○ Claude  claude-opus
```

快捷切换：

```text
[应用]
```

---

# 15. 与 cpa-key-policy 的关系

推荐 V1 不重新实现已有的 Key Policy Alias 内核。

采用：

```text
cpa-route-control
        │
        ▼
cpa-key-policy Management API
        │
        ▼
Alias / Group
```

route-control 负责：

- UI；
- Profile；
- Schedule；
- 快捷操作；
- 审计；
- 回滚。

key-policy 继续负责：

- Alias 真正路由；
- 下游 Key Policy；
- Group；
- RPM；
- Budget。

这样职责清晰。

---

# 16. 兼容模式

插件提供：

```yaml
alias_backend: auto
```

支持：

```text
auto
key-policy
native
disabled
```

含义：

### auto

检测是否安装：

```text
cpa-key-policy
```

存在则自动使用。

---

### key-policy

强制使用：

```text
cpa-key-policy
```

---

### native

只调用 CPA 原生能力。

---

### disabled

完全不管理 Alias，只控制 Weight / Priority。

---

# 17. Credential 标识

禁止使用明文 Key 作为系统 ID。

错误：

```text
credential_id = sk-xxxx
```

正确：

```text
credential_id = codex-account-01
```

或：

```text
credential_id = auth_46d72...
```

UI 只显示：

```text
GLM-A
sk-***7F2A
```

绝不返回完整 Key。

---

# 18. 插件 Management API

建议统一前缀：

```text
/v0/management/plugins/cpa-route-control
```

---

## 18.1 状态

```http
GET /status
```

返回：

```json
{
  "version": "1.0.0",
  "scheduler": true,
  "active_profile": "afternoon",
  "manual_override": false,
  "next_schedule": "2026-09-09T18:00:00+08:00"
}
```

---

## 18.2 获取 Credential

```http
GET /credentials
```

返回：

```json
[
  {
    "id": "glm-a",
    "name": "GLM A",
    "provider": "glm",
    "priority": 100,
    "weight": 20,
    "enabled": true,
    "masked_key": "sk-***12AF"
  }
]
```

---

## 18.3 修改权重

```http
POST /credentials/weights
```

请求：

```json
{
  "items": [
    {
      "credential_id": "glm-a",
      "weight": 80
    },
    {
      "credential_id": "kimi-b",
      "weight": 20
    }
  ]
}
```

---

## 18.4 修改 Priority

```http
POST /credentials/priorities
```

请求：

```json
{
  "items": [
    {
      "credential_id": "glm-a",
      "priority": 100
    }
  ]
}
```

---

## 18.5 获取 Alias

```http
GET /aliases
```

---

## 18.6 Alias 快捷切换

```http
POST /aliases/switch
```

请求：

```json
{
  "alias": "code",
  "provider": "kimi",
  "target_model": "kimi-k2.5"
}
```

---

## 18.7 Profile 列表

```http
GET /profiles
```

---

## 18.8 保存 Profile

```http
POST /profiles
```

---

## 18.9 应用 Profile

```http
POST /profiles/apply
```

请求：

```json
{
  "profile_id": "afternoon",
  "override": {
    "mode": "until_next_schedule"
  }
}
```

---

## 18.10 Dry Run

所有真正修改前都应支持：

```http
POST /profiles/dry-run
```

返回：

```json
{
  "changes": [
    {
      "type": "weight",
      "credential": "glm-a",
      "from": 80,
      "to": 20
    },
    {
      "type": "weight",
      "credential": "kimi-b",
      "from": 20,
      "to": 80
    },
    {
      "type": "alias",
      "alias": "code",
      "from": "glm-5",
      "to": "kimi-k2.5"
    }
  ]
}
```

---

# 19. 原子应用

Profile 不能逐条随意修改。

错误：

```text
先改 GLM = 20
          ↓
程序异常
          ↓
Kimi 还没改
```

会产生中间错误状态。

正确流程：

```text
1. 读取当前状态
2. 创建 Snapshot
3. 校验目标 Profile
4. 计算 Diff
5. 获取全局写锁
6. Alias 更新
7. Credential 更新
8. 二次读取确认
9. 写 Audit
10. 释放写锁
```

如果任意步骤失败：

```text
自动 Rollback Snapshot
```

---

# 20. Snapshot

每次修改前保存：

```json
{
  "snapshot_id": "snap_20260909_120000",
  "created_at": "2026-09-09T12:00:00+08:00",
  "reason": "apply_profile",
  "aliases": {},
  "credentials": {}
}
```

默认保留：

```text
最近 100 个
```

可配置：

```yaml
snapshot_retention: 100
```

---

# 21. Rollback

API：

```http
POST /rollback
```

请求：

```json
{
  "snapshot_id": "snap_20260909_120000"
}
```

Tauri 页面：

```text
最近操作

12:00 下午策略
Before: GLM80 / Kimi20
After : GLM20 / Kimi80

[回滚]
```

---

# 22. Audit 操作日志

记录：

- 时间；
- 操作者；
- 来源；
- 修改内容；
- Before；
- After；
- 是否成功；
- 错误信息。

来源：

```text
tauri
web
scheduler
api
```

示例：

```json
{
  "time": "2026-09-09T12:00:00+08:00",
  "source": "scheduler",
  "action": "apply_profile",
  "profile": "afternoon",
  "success": true
}
```

---

# 23. 状态文件

V1 推荐：

```text
cpa-route-control-state.json
```

负责保存：

- Profile；
- Schedule；
- Alias 配置引用；
- Manual Override；
- Snapshot 索引；
- 插件配置。

写文件时必须：

```text
write tmp
   ↓
fsync
   ↓
rename
```

禁止直接覆盖原文件导致异常退出后 JSON 损坏。

---

# 24. Scheduler 设计

Scheduler 必须运行在 CPA 插件内。

不要运行在：

```text
Tauri 前端
```

原因：

```text
Tauri 被关闭
Windows 重启
用户退出桌面
```

都会导致任务失效。

正确：

```text
CPA 服务启动
    │
    ▼
Route Control Plugin
    │
    ▼
Scheduler
```

---

# 25. Scheduler 启动恢复

CPA 重启后：

```text
1. 加载 state
2. 加载所有 Schedule
3. 检查 Manual Override
4. 计算当前应该生效的 Profile
5. 对比实际 CPA 状态
6. 必要时自动恢复正确状态
```

配置：

```yaml
reconcile_on_startup: true
```

---

# 26. Tauri Desktop 技术选型

推荐：

```text
Tauri 2
Vue 3
Vite
JavaScript
Pinia
Element Plus
Rust
```

结构：

```text
Vue
 │
 │ invoke
 ▼
Tauri Rust
 │
 │ reqwest
 ▼
CPA Route Control API
```

禁止：

```text
Vue 直接保存 CPA Management Secret
```

Secret 必须由 Rust 层管理。

---

# 27. Tauri 为什么需要 Rust 中间层

前端：

```text
Vue WebView
```

只负责：

- 页面；
- 状态展示；
- 用户输入。

真正请求：

```text
Rust
```

负责：

- Management Secret；
- HTTP；
- TLS；
- 超时；
- 重试；
- 凭据安全；
- 多 CPA 实例。

前端调用：

```javascript
invoke("apply_profile", {
  profileId: "afternoon"
})
```

Rust：

```text
apply_profile()
       │
       ▼
POST CPA API
```

---

# 28. Tauri Commands

V1 Commands：

```text
get_servers
add_server
update_server
remove_server

test_connection
get_status

get_profiles
apply_profile

get_credentials
set_weight
set_priority

get_aliases
switch_alias

get_schedules
save_schedule

get_audit_logs
rollback

pause_scheduler
resume_scheduler
```

---

# 29. Tauri 本地配置

保存：

```text
servers.json
preferences.json
```

示例：

```json
{
  "servers": [
    {
      "id": "cpa-home",
      "name": "家里 CPA",
      "url": "https://cpa.example.com",
      "secret_ref": "cpa-home-secret"
    }
  ]
}
```

Management Secret 不写入普通 JSON。

Windows 应使用系统凭据安全存储。

---

# 30. Tauri 首页

设计为快捷操作，不做复杂后台。

```text
┌────────────────────────────────────┐
│ CPA Route Control                  │
│ 家里 CPA                    ● 在线 │
├────────────────────────────────────┤
│ 当前模型                           │
│                                    │
│ code                               │
│ Kimi / kimi-k2.5                   │
│                                    │
│ [切 GLM]  [切 Kimi]  [自动]        │
├────────────────────────────────────┤
│ 权重                               │
│                                    │
│ GLM A     20  ████                 │
│ Kimi B    80  ████████████████     │
│                                    │
│ [50/50] [GLM100] [Kimi100]         │
├────────────────────────────────────┤
│ 当前 Profile：下午                 │
│ 下一次：18:00 晚间策略             │
└────────────────────────────────────┘
```

---

# 31. Tauri 页面

V1 一共建议 6 个一级页面：

```text
1 Dashboard
2 Quick Control
3 Profiles
4 Schedule
5 Logs
6 Settings
```

不要把 CPA 全部配置搬到桌面端。

---

# 32. Quick Control

最重要页面。

显示：

```text
模型
code
```

目标：

```text
[GLM]
[Kimi]
[GPT]
[Claude]
```

权重：

```text
GLM    [--------80]
Kimi   [--20]
```

预设：

```text
[GLM 100]
[Kimi 100]
[80 / 20]
[50 / 50]
[20 / 80]
```

应用：

```text
[立即应用]
```

---

# 33. 系统托盘

Tauri 必须支持系统托盘。

右键：

```text
CPA Route Control
────────────────

● CPA Online

当前：
code -> Kimi

快速切换
  GLM 100%
  Kimi 100%
  50 / 50

Profile
  上午
  下午
  晚上

───────────────
打开控制面板
退出
```

这样绝大多数时候不需要打开窗口。

---

# 34. Tray 二次确认

对于：

```text
100% 切换
```

可以配置：

```text
confirm_destructive_switch = true
```

开启时：

```text
是否将 code 全部切换到 GLM？

GLM 100
Kimi 0

[取消] [确认]
```

---

# 35. 快捷键

支持全局快捷键，例如：

```text
Ctrl + Alt + 1 -> 上午策略
Ctrl + Alt + 2 -> 下午策略
Ctrl + Alt + 3 -> 晚上策略

Ctrl + Alt + G -> GLM 100%
Ctrl + Alt + K -> Kimi 100%
```

必须可关闭和修改。

---

# 36. 多 CPA 实例

桌面客户端支持：

```text
CPA-A
CPA-B
CPA-测试
```

顶部选择：

```text
当前实例：生产 CPA ▼
```

每个实例独立保存：

- URL；
- Secret；
- TLS 选项；
- 昵称。

---

# 37. Web 与 Tauri 一致性

Web 和 Tauri 不各自实现业务。

统一：

```text
                   Route Control API
                  /                 \
                 /                   \
               Web                 Tauri
```

Web 与 Tauri 都只是 API Client。

所有业务必须在：

```text
Go Plugin Service Layer
```

里实现。

---

# 38. Go 插件代码结构

建议：

```text
cpa-route-control/
├── cmd/
│   └── cpa-route-control/
│       └── main.go
│
├── internal/
│   ├── api/
│   │   ├── status.go
│   │   ├── profile.go
│   │   ├── schedule.go
│   │   ├── alias.go
│   │   ├── credential.go
│   │   └── audit.go
│   │
│   ├── service/
│   │   ├── route_service.go
│   │   ├── profile_service.go
│   │   ├── schedule_service.go
│   │   └── rollback_service.go
│   │
│   ├── adapter/
│   │   ├── cpa/
│   │   └── keypolicy/
│   │
│   ├── scheduler/
│   │   └── scheduler.go
│   │
│   ├── state/
│   │   ├── store.go
│   │   └── snapshot.go
│   │
│   └── model/
│       ├── profile.go
│       ├── credential.go
│       └── schedule.go
│
├── web/
│   ├── src/
│   └── dist/
│
├── config.example.yaml
├── go.mod
└── Makefile
```

---

# 39. Tauri 项目结构

```text
cpa-route-control-desktop/
├── src/
│   ├── api/
│   ├── components/
│   ├── pages/
│   │   ├── Dashboard.vue
│   │   ├── QuickControl.vue
│   │   ├── Profiles.vue
│   │   ├── Schedule.vue
│   │   ├── Logs.vue
│   │   └── Settings.vue
│   │
│   ├── stores/
│   └── App.vue
│
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   ├── cpa_client/
│   │   ├── secure_store/
│   │   ├── tray/
│   │   └── lib.rs
│   │
│   ├── capabilities/
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── package.json
└── vite.config.js
```

---

# 40. 前端状态

Pinia：

```text
serverStore
statusStore
profileStore
routeStore
scheduleStore
logStore
```

不要把 Management Secret 放入 Pinia。

---

# 41. API Client

Rust 定义统一：

```text
CpaRouteControlClient
```

方法：

```rust
get_status()
get_credentials()
set_weights()
set_priorities()

get_aliases()
switch_alias()

get_profiles()
apply_profile()

get_schedules()

get_audit_logs()
rollback()
```

统一处理：

- Timeout；
- 401；
- 403；
- 404；
- 409；
- 5xx；
- JSON Decode；
- TLS。

---

# 42. 超时

建议：

```text
连接超时：3 秒
普通 API：10 秒
Apply Profile：15 秒
```

不要无限等待。

---

# 43. 重试

GET：

```text
允许重试
```

POST 修改操作：

```text
默认不要盲目自动重试
```

应使用：

```text
request_id
```

实现幂等。

示例：

```json
{
  "request_id": "01JXXXXX",
  "profile_id": "afternoon"
}
```

插件记录最近请求 ID。

重复请求：

```text
返回上一次执行结果
```

避免权重被重复操作。

---

# 44. 并发控制

同一时刻可能出现：

```text
Scheduler 正在切换
        +
Tauri 手动切换
```

因此插件必须有：

```text
RouteWriteMutex
```

并定义优先级：

```text
Manual Override > Scheduler
```

如果用户正在手动应用：

Scheduler：

```text
等待写锁
```

执行后再次检查 Override。

---

# 45. 配置版本

所有状态增加：

```json
{
  "revision": 128
}
```

客户端修改时：

```json
{
  "expected_revision": 128
}
```

如果当前已经：

```text
129
```

返回：

```http
409 Conflict
```

防止两个管理端互相覆盖。

---

# 46. 权重校验

规则：

```text
0 <= Weight <= CPA 支持最大值
```

UI 常规模式建议限制：

```text
0 - 100
```

高级模式：

```text
允许更大整数
```

因为实际比例只与相对值有关：

```text
80 : 20
```

等价于：

```text
4 : 1
```

---

# 47. Priority 校验

UI：

```text
Priority
100
```

并提示：

```text
Priority 先于 Weight。
不同 Priority 不属于同一个加权池。
```

避免误以为：

```text
priority=100 weight=1

priority=90 weight=100
```

会按照 1:100 分流。

---

# 48. 安全要求

## 48.1 Management Secret

禁止：

```text
localStorage
sessionStorage
Vue Store
console.log
普通 JSON
```

保存 Management Secret。

---

## 48.2 Tauri

Secret 由 Rust 层读取。

前端只使用：

```text
server_id
```

例如：

```javascript
invoke("get_status", {
  serverId: "cpa-home"
})
```

---

# 49. Tauri Capabilities

仅开启需要的能力：

- 网络请求；
- Store；
- Tray；
- Window；
- Notification；
- Global Shortcut（如果启用）。

不要为了开发方便直接给所有文件系统权限。

---

# 50. CPA 插件安全

Management API 必须使用：

```text
CPA Management Bearer Token
```

不得因为是插件就绕过 Management Authentication。

---

# 51. Web Secret

Web 插件登录后 Secret：

```text
只保存在内存
```

页面刷新：

```text
重新登录
```

除非后续明确实现安全 Session Token。

---

# 52. 日志脱敏

错误：

```text
request key=sk-abc123456
```

正确：

```text
request credential=glm-a
key=sk-***3456
```

---

# 53. HTTPS

远程 CPA：

```text
强烈要求 HTTPS
```

Tauri 默认拒绝无效证书。

开发环境可以提供：

```text
allow_invalid_certificate
```

但必须：

```text
默认 false
```

并在 UI 显示红色警告。

---

# 54. 健康检查

Dashboard 每：

```text
10 秒
```

请求：

```text
/status
```

显示：

```text
● Online
● Degraded
● Offline
```

不要 1 秒一次轮询。

---

# 55. Apply Profile 流程

完整过程：

```text
用户点击 下午策略
        │
        ▼
Tauri -> POST /profiles/dry-run
        │
        ▼
显示：
GLM 80 -> 20
Kimi 20 -> 80
code GLM -> Kimi
        │
        ▼
确认
        │
        ▼
POST /profiles/apply
        │
        ▼
Plugin Snapshot
        │
        ▼
Alias Update
        │
        ▼
Credential Update
        │
        ▼
Verify
        │
        ▼
Audit
        │
        ▼
成功
```

快捷按钮可以配置：

```text
skip_preview = true
```

实现真正一键切换。

---

# 56. 推荐默认 Profile

初始化可以生成：

## balanced

```text
GLM 50
Kimi 50
```

## glm

```text
GLM 100
Kimi 0
```

## kimi

```text
GLM 0
Kimi 100
```

## morning

```text
GLM 80
Kimi 20
```

## afternoon

```text
GLM 20
Kimi 80
```

---

# 57. 示例最终配置

你的实际业务可以配置：

```text
alias:
code
```

候选：

```text
GLM:
provider = glm
model = glm-5
credential = key-a

Kimi:
provider = kimi
model = kimi-k2.5
credential = key-b
```

---

# 58. 上午

08:00：

```text
code -> GLM

Key A:
priority = 100
weight = 80

Key B:
priority = 100
weight = 20
```

---

# 59. 下午

12:00：

```text
code -> Kimi

Key A:
priority = 100
weight = 20

Key B:
priority = 100
weight = 80
```

---

# 60. 临时强制 GLM

用户 Tauri：

```text
Tray
  -> GLM 100%
```

结果：

```text
code -> GLM

Key A weight = 100
Key B weight = 0

Override:
until_next_schedule
```

到下一个：

```text
18:00
```

自动恢复 Scheduler。

---

# 61. CPA 不可用

如果切换时 CPA Down：

```text
Schedule Job
   -> Failed
```

插件记录：

```text
pending reconcile
```

恢复后：

```text
重新计算当前应该生效的 Profile
```

而不是机械补执行所有历史任务。

---

# 62. 上游 Credential 不可用

如果目标 Profile：

```text
GLM = 100
```

但 GLM Credential 已失效：

应用前校验：

```text
warning
```

严格模式：

```text
拒绝应用
```

宽松模式：

```text
允许，但告警
```

配置：

```yaml
profile_validation: strict
```

默认：

```text
strict
```

---

# 63. 保护策略

防止误操作导致全部：

```text
weight = 0
```

默认校验：

```text
至少一个目标 Credential weight > 0
```

除非：

```text
allow_zero_route = true
```

---

# 64. 自动回滚

应用后检查：

```text
Alias
Credential Weight
Priority
```

与目标 Profile 不一致：

```text
Rollback
```

并返回：

```text
apply_failed_and_rolled_back
```

---

# 65. 审计页面

显示：

```text
09:00  Scheduler   上午策略       成功
12:00  Scheduler   下午策略       成功
14:21  Desktop     GLM 100%       成功
18:00  Scheduler   晚间策略       成功
```

点击查看 Diff。

---

# 66. Web 增强插件首页快捷区

CPA 管理页面插件重点不是漂亮，而是快。

顶部固定：

```text
code
当前：Kimi

[GLM]
[Kimi]
[GPT]
[自动]
```

第二行：

```text
GLM 20
Kimi 80

[100/0]
[80/20]
[50/50]
[20/80]
[0/100]
```

第三行：

```text
Scheduler: ON
Profile: Afternoon
Next: 18:00
```

---

# 67. 开发顺序

## Phase 1：插件基础

完成：

- plugin load；
- Management API；
- Credential read；
- Weight update；
- Priority update；
- state store。

---

## Phase 2：Profile

完成：

- Profile CRUD；
- Dry Run；
- Apply；
- Snapshot；
- Rollback。

---

## Phase 3：Schedule

完成：

- Cron；
- Manual Override；
- Startup Reconcile；
- Audit。

---

## Phase 4：Web UI

完成：

- Dashboard；
- Quick Control；
- Profiles；
- Schedule；
- Logs。

---

## Phase 5：Tauri

完成：

- Server 管理；
- Dashboard；
- Quick Control；
- Tray；
- Profile；
- Logs。

---

# 68. 单元测试

必须覆盖：

```text
Priority 筛选
Weight 校验
Weight=0
Profile Diff
Profile Apply
Apply Rollback
Manual Override
Schedule
Startup Reconcile
409 Revision Conflict
重复 request_id
Alias Switch
Credential 不存在
```

---

# 69. 集成测试

至少准备：

```text
CPA Test Instance
```

模拟：

```text
GLM-A
Kimi-B
```

验证：

```text
上午 Profile
下午 Profile
手动 Override
下一次 Schedule 恢复
```

---

# 70. 压力测试

不需要高并发业务压测。

重点测试控制层：

```text
10 个客户端同时修改
Scheduler 同时触发
重复点击 Profile
网络超时
CPA 重启
Plugin 重启
```

---

# 71. 验收标准

V1 验收必须满足：

1. 客户端始终使用 `model=code`；
2. Tauri 可以一键将 `code` 切到 GLM；
3. Tauri 可以一键将 `code` 切到 Kimi；
4. 能修改 Key S / Key Y Weight；
5. 支持 80/20、20/80、50/50、100/0；
6. 支持 Priority；
7. 支持 Profile；
8. 支持定时 Profile；
9. Tauri 关闭后定时规则继续执行；
10. 支持 Manual Override；
11. 下个定时点可以自动恢复；
12. 每次操作有日志；
13. 每次修改前有 Snapshot；
14. 可以一键 Rollback；
15. Web 插件可以脱离 Tauri 独立使用；
16. Secret 不进入 Web localStorage；
17. Tauri 前端拿不到完整 Management Secret；
18. 操作失败不会留下半应用状态；
19. CPA 重启后可以自动 reconcile；
20. 所有 Credential Key 默认脱敏显示。

---

# 72. V2 可扩展功能

后续可以增加：

- 根据额度自动调权；
- 根据 API 价格自动调权；
- 根据失败率自动调权；
- 根据延迟自动调权；
- Provider 熔断；
- 额度低于 10% 自动 weight=0；
- 错误率过高自动降低 Weight；
- token 成本策略；
- 节假日 Profile；
- Telegram / 企业微信通知；
- 多 CPA 批量应用；
- Prometheus；
- Grafana；
- 远程 WebSocket 状态推送。

---

# 73. V2 智能权重示例

未来：

```text
GLM:
剩余额度 80%
平均延迟 1.2s
错误率 1%

Kimi:
剩余额度 30%
平均延迟 2.8s
错误率 7%
```

自动计算：

```text
GLM weight = 80
Kimi weight = 20
```

但 V1 不建议做智能算法。

V1 首先保证：

```text
手动稳定
+
定时稳定
+
可回滚
```

---

# 74. 最终推荐实现

项目最终结构：

```text
                    ┌──────────────────┐
                    │     Codex        │
                    │ model = code     │
                    └────────┬─────────┘
                             │
                             ▼
                     ┌──────────────┐
                     │ CLIProxyAPI  │
                     └──────┬───────┘
                            │
              ┌─────────────┴─────────────┐
              │                           │
              ▼                           ▼
       cpa-key-policy             cpa-route-control
       Alias / Group              Schedule / Profile
              │                   Weight / Control
              └─────────────┬─────────────┘
                            │
                       ┌────┴─────┐
                       ▼          ▼
                     GLM         Kimi
                    Key A        Key B
```

管理端：

```text
              ┌───────────────────┐
              │ CPA Web Plugin UI │
              └─────────┬─────────┘
                        │
                        ▼
                Route Control API
                        ▲
                        │
              ┌─────────┴─────────┐
              │    Tauri Desktop  │
              │ Tray / Quick UI   │
              └───────────────────┘
```

整个系统的关键不是“做两个 UI”，而是：

> **只实现一套 Route Control Service，Web 和 Tauri 都调用同一套 API。**

这样不会出现：

```text
Web 一套逻辑
Tauri 一套逻辑
最后行为不一致
```

---

# 75. V1 开发决策总结

最终建议直接确定：

```text
桌面：
Tauri 2 + Vue 3 + JavaScript

CPA 插件：
Go

Alias：
优先复用 cpa-key-policy

动态权重：
调用 CPA Credential Weight

定时：
运行在 CPA 插件内部

状态：
JSON Atomic Store

安全：
Management Token + Tauri Rust Secret Store

快捷入口：
Tauri Tray

核心对象：
Profile

核心动作：
Apply Profile

恢复：
Snapshot + Rollback

冲突控制：
Revision + Mutex

自动覆盖：
Manual Override > Scheduler
```

这是 V1 最适合直接进入开发的结构。
