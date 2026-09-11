const SECTIONS = [
  "claude-api-key",
  "gemini-api-key",
  "codex-api-key",
  "xai-api-key",
  "vertex-api-key",
  "interactions-api-key",
];

const array = (value) => (Array.isArray(value) ? value : value ? [value] : []);
const modelName = (item) =>
  typeof item === "string" ? item : String(item?.name || "");
const modelAlias = (item) =>
  typeof item === "object" && item ? String(item.alias || "") : "";
const integer = (value, fallback) =>
  Number.isInteger(value) ? value : fallback;

function providerOf(name, baseUrl, fallback) {
  const value = `${name} ${baseUrl}`.toLowerCase();
  if (value.includes("glm") || value.includes("bigmodel")) return "glm";
  if (value.includes("kimi") || value.includes("moonshot")) return "kimi";
  if (value.includes("minimax")) return "minimax";
  if (value.includes("grok") || value.includes("x.ai")) return "xai";
  if (value.includes("gemini")) return "gemini";
  if (value.includes("gpt") || value.includes("openai")) return "openai";
  if (value.includes("claude") || value.includes("anthropic")) return "claude";
  return fallback.replace(/-api-key$/, "");
}

function normalize(section, entry, index, group = {}) {
  const models = array(entry?.models || group.models)
    .map((item) => ({ name: modelName(item), alias: modelAlias(item) }))
    .filter((item) => item.name);
  const primary = models.find((item) => item.alias) || models[0];
  const baseUrl = String(entry?.["base-url"] || group["base-url"] || "");
  const provider = providerOf(primary?.name || "", baseUrl, section);
  const id = String(entry?.["auth-index"] || `${section}-${index}`);
  const disabled =
    entry?.disabled === true ||
    group?.disabled === true ||
    array(entry?.["excluded-models"]).includes("*");
  return {
    credential: {
      id,
      name: primary?.name || `${provider.toUpperCase()} ${index + 1}`,
      provider,
      priority: integer(entry?.priority, integer(group?.priority, 0)),
      weight: integer(entry?.weight, 1),
      enabled: !disabled,
      disabled,
      models,
      service_url: baseUrl || "默认服务地址",
      account_label: "",
      config_section: section,
      config_index: index,
    },
    aliases: models
      .filter((item) => item.alias)
      .map((item) => ({
        alias: item.alias,
        provider: providerOf(item.name, baseUrl, provider),
        target_model: item.name,
        credential_id: id,
      })),
  };
}

export function normalizeConfig(config = {}, { includeDisabled = false } = {}) {
  const rows = [];
  for (const section of SECTIONS)
    array(config[section]).forEach((entry, index) =>
      rows.push(normalize(section, entry, index)),
    );
  array(config["openai-compatibility"]).forEach((group, groupIndex) => {
    array(group?.["api-key-entries"]).forEach((entry, entryIndex) =>
      rows.push(
        normalize(
          "openai-compatibility",
          entry,
          groupIndex * 10000 + entryIndex,
          group,
        ),
      ),
    );
  });
  const selectedRows = includeDisabled
    ? rows
    : rows.filter((row) => row.credential.enabled);
  const credentials = selectedRows.map((row) => row.credential);
  const candidates = selectedRows
    .flatMap((row) => row.aliases)
    .map((alias) => {
      const credential = credentials.find(
        (item) => item.id === alias.credential_id,
      );
      return {
        ...alias,
        enabled: credential?.enabled !== false,
        priority: credential?.priority || 0,
        weight: credential?.weight || 0,
      };
    });
  const aliases = [...new Set(candidates.map((item) => item.alias))].map(
    (name) => {
      const matches = candidates
        .filter((item) => item.alias === name)
        .sort(
          (a, b) =>
            Number(b.enabled) - Number(a.enabled) ||
            b.priority - a.priority ||
            b.weight - a.weight,
        );
      return { ...matches[0], candidates: matches };
    },
  );
  return { credentials, aliases };
}

export function normalizeSnapshot(
  config = {},
  authFilesPayload = {},
  { includeDisabled = false } = {},
) {
  const configured = normalizeConfig(config, { includeDisabled });
  const runtimeFiles = array(authFilesPayload.files)
    .map((file, index) => ({
      id: String(
        file.auth_index ||
          file.authIndex ||
          file.id ||
          file.name ||
          `auth-file-${index}`,
      ),
      name: String(
        file.label || file.name || file.id || `授权文件 ${index + 1}`,
      ),
      provider: String(file.provider || file.type || "oauth").toLowerCase(),
      priority: integer(file.priority, 0),
      weight: integer(file.weight, 1),
      enabled: file.disabled !== true && file.unavailable !== true,
      disabled: file.disabled === true,
      unavailable: file.unavailable === true,
      status: String(file.status || ""),
      source: String(file.source || "file"),
      config_section: "auth-files",
      config_index: index,
      auth_name: String(file.name || file.id || ""),
      account_label: String(
        file.email ||
          file.account ||
          file.label ||
          file.id ||
          file.name ||
          `授权账号 ${index + 1}`,
      ),
      service_url: "",
      credential_type: "授权文件",
    }))
    .filter((file) => includeDisabled || file.enabled);
  const credentials = configured.credentials.map((item) => ({
    ...item,
    credential_type: "配置 Key",
  }));
  for (const runtime of runtimeFiles) {
    const existing = credentials.find((item) => item.id === runtime.id);
    if (existing) {
      Object.assign(existing, {
        enabled: runtime.enabled,
        status: runtime.status,
        source: runtime.source,
      });
    } else {
      credentials.push(runtime);
    }
  }
  return { credentials, aliases: configured.aliases };
}
