import { invoke } from "@tauri-apps/api/core";

export const DEFAULT_MANAGEMENT_URL = "http://127.0.0.1:8317/v0/management";

export function isTauri() {
  return "__TAURI_INTERNALS__" in window;
}

export class CpaManagementClient {
  constructor() {
    this.baseUrl = DEFAULT_MANAGEMENT_URL;
    this.managementKey = "";
  }

  async connect(baseUrl, managementKey, remember = false) {
    this.baseUrl = String(baseUrl).replace(/\/$/, "");
    this.managementKey = managementKey;
    if (isTauri())
      return invoke("connect_cpa", {
        baseUrl: this.baseUrl,
        managementKey,
        remember,
      });
    return this.request("/config");
  }

  async restore(baseUrl) {
    this.baseUrl = String(baseUrl).replace(/\/$/, "");
    if (!isTauri()) return null;
    return invoke("restore_cpa", { baseUrl: this.baseUrl });
  }

  async request(path, { method = "GET", body } = {}) {
    if (isTauri())
      return invoke("cpa_request", { method, path, body: body ?? null });
    if (!this.managementKey) throw new Error("请先输入 CPA 明文管理密钥");
    const local = this.baseUrl === DEFAULT_MANAGEMENT_URL;
    const url = local ? `/cpa-management${path}` : `${this.baseUrl}${path}`;
    const response = await fetch(url, {
      method,
      headers: {
        authorization: `Bearer ${this.managementKey}`,
        ...(body === undefined ? {} : { "content-type": "application/json" }),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const text = await response.text();
    let data = {};
    try {
      data = text ? JSON.parse(text) : {};
    } catch {
      data = { message: text };
    }
    if (!response.ok)
      throw new Error(
        data.message ||
          data.error ||
          `CPA Management API 返回 HTTP ${response.status}`,
      );
    return data;
  }

  config() {
    return this.request("/config");
  }
  authFiles() {
    return this.request("/auth-files");
  }
  async snapshot() {
    const [config, authFiles] = await Promise.all([
      this.config(),
      this.authFiles(),
    ]);
    return { config, authFiles };
  }

  async updateCredentials(credentials, field, values) {
    const sections = new Map();
    for (const change of values) {
      const credential = credentials.find(
        (item) => item.id === change.credential_id,
      );
      if (!credential)
        throw new Error(`Credential ${change.credential_id} 不存在`);
      const changes = sections.get(credential.config_section) || [];
      changes.push({ credential, value: change[field] });
      sections.set(credential.config_section, changes);
    }
    for (const [section, changes] of sections) {
      if (section === "auth-files") {
        for (const change of changes) {
          await this.request("/auth-files/fields", {
            method: "PATCH",
            body: { name: change.credential.auth_name, [field]: change.value },
          });
        }
        continue;
      }
      if (section === "openai-compatibility") {
        await this.updateOpenAiCompatibility(changes, field);
        continue;
      }
      const payload = await this.request(`/${section}`);
      const items = payload[section] || payload.items || payload;
      if (!Array.isArray(items))
        throw new Error(`CPA 返回的 ${section} 不是数组`);
      for (const change of changes) {
        const current = items[change.credential.config_index];
        if (!current)
          throw new Error(
            `${section}[${change.credential.config_index}] 不存在`,
          );
        current[field] = change.value;
      }
      await this.request(`/${section}`, { method: "PUT", body: items });
    }
    return this.snapshot();
  }

  async updateOpenAiCompatibility(changes, field) {
    const section = "openai-compatibility";
    const payload = await this.request(`/${section}`);
    const groups = payload[section] || payload.items || payload;
    if (!Array.isArray(groups))
      throw new Error("CPA 返回的 openai-compatibility 不是数组");
    for (const change of changes) {
      const groupIndex = Math.floor(change.credential.config_index / 10000);
      const entryIndex = change.credential.config_index % 10000;
      const entry = groups[groupIndex]?.["api-key-entries"]?.[entryIndex];
      if (!entry)
        throw new Error(
          `openai-compatibility[${groupIndex}] Credential 不存在`,
        );
      entry[field] = change.value;
    }
    await this.request(`/${section}`, { method: "PUT", body: groups });
  }

  setWeights(credentials, items) {
    return this.updateCredentials(credentials, "weight", items);
  }
  setPriorities(credentials, items) {
    return this.updateCredentials(credentials, "priority", items);
  }

  async updateModelAlias(credential, modelName, alias, resetRouting = false) {
    const section = credential.config_section;
    if (section === "auth-files") throw new Error("授权文件不能设置模型别名");
    if (section === "openai-compatibility")
      return this.updateOpenAiModelAlias(
        credential,
        modelName,
        alias,
        resetRouting,
      );
    const payload = await this.request(`/${section}`);
    const items = payload[section] || payload.items || payload;
    if (!Array.isArray(items))
      throw new Error(`CPA 返回的 ${section} 不是数组`);
    const current = items[credential.config_index];
    if (!current)
      throw new Error(`${section}[${credential.config_index}] 不存在`);
    const models = Array.isArray(current.models) ? current.models : [];
    const model = models.find(
      (item) => (typeof item === "string" ? item : item?.name) === modelName,
    );
    if (!model) throw new Error(`模型 ${modelName} 不存在`);
    if (typeof model === "string")
      current.models = models.map((item) =>
        item === modelName ? { name: item, alias } : item,
      );
    else model.alias = alias;
    if (resetRouting) {
      current.priority = 0;
      current.weight = 0;
    }
    await this.request(`/${section}`, { method: "PUT", body: items });
    return this.snapshot();
  }

  async updateOpenAiModelAlias(credential, modelName, alias, resetRouting) {
    const section = "openai-compatibility";
    const payload = await this.request(`/${section}`);
    const groups = payload[section] || payload.items || payload;
    if (!Array.isArray(groups))
      throw new Error("CPA 返回的 openai-compatibility 不是数组");
    const groupIndex = Math.floor(credential.config_index / 10000);
    const group = groups[groupIndex];
    const models = Array.isArray(group?.models) ? group.models : [];
    const model = models.find(
      (item) => (typeof item === "string" ? item : item?.name) === modelName,
    );
    if (!model) throw new Error(`OpenAI 兼容模型 ${modelName} 不存在`);
    if (typeof model === "string")
      group.models = models.map((item) =>
        item === modelName ? { name: item, alias } : item,
      );
    else model.alias = alias;
    if (resetRouting)
      for (const entry of Array.isArray(group?.["api-key-entries"])
        ? group["api-key-entries"]
        : []) {
        entry.priority = 0;
        entry.weight = 0;
      }
    await this.request(`/${section}`, { method: "PUT", body: groups });
    return this.snapshot();
  }
}
