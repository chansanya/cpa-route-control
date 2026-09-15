<script setup>
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import {
  Activity,
  ArrowLeft,
  ArrowUpRight,
  Check,
  ChevronDown,
  CircleAlert,
  Command,
  Copy,
  Download,
  ExternalLink,
  Gauge,
  KeyRound,
  LayoutDashboard,
  LockKeyhole,
  LogOut,
  Plus,
  RefreshCw,
  Route,
  Save,
  Search,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  Terminal,
  Trash2,
  Wifi,
  WifiOff,
  X,
} from "@lucide/vue";
import {
  CpaManagementClient,
  DEFAULT_MANAGEMENT_URL,
  isTauri,
  openExternalUrl,
} from "./api";
import { normalizeSnapshot } from "./cpa-config";
import packageMeta from "../package.json";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import ConnectionPanel from "./components/ConnectionPanel.vue";
import CpaWebPanel from "./components/CpaWebPanel.vue";
import CredentialStatusModal from "./components/CredentialStatusModal.vue";
import AliasCreatorModal from "./components/AliasCreatorModal.vue";
import StrategyEditorModal from "./components/StrategyEditorModal.vue";

const appVersion = packageMeta.version;
const updateInfo = ref(null);
const checkingForUpdate = ref(false);
const client = new CpaManagementClient();
const page = ref("settings");
const managementUrl = ref(
  localStorage.getItem("cpa-management-url") || DEFAULT_MANAGEMENT_URL,
);
const desktop = isTauri();
const cpaWebUrl = computed(() => {
  try {
    return `${new URL(managementUrl.value).origin}/management.html`;
  } catch {
    return "/management.html";
  }
});
const rememberedBrowserKey = desktop
  ? ""
  : sessionStorage.getItem("cpa-management-key") || "";
const managementKey = ref(rememberedBrowserKey);
const rememberKey = ref(desktop || Boolean(rememberedBrowserKey));
const connected = ref(false);
const reconnecting = ref(false);
const sessionAvailable = computed(() => connected.value || reconnecting.value);
const loading = ref(false);
const notice = ref(null);
const confirmDialog = ref(null);
let confirmResolver;
const credentials = ref([]);
const manageableCredentials = ref([]);
const showCredentialStatus = ref(false);
const credentialStatusTab = ref("providers");
const aliases = ref([]);
const customAliasNames = ref(loadCustomAliasNames());
const showAliasCreator = ref(false);
const newAliasName = ref("");
const draftWeights = reactive({});
const draftPriorities = reactive({});
const draftDirty = ref(false);
const strategyWeights = reactive({});
const strategyPriorities = reactive({});
const strategyName = ref("");
const showStrategyEditor = ref(false);
const editingStrategyId = ref(null);
const selectedStrategyId = ref(null);
const strategies = ref(loadStrategies());
let refreshTimer;
let reconnectTimer;
let reconnectAttempts = 0;
let connectionCheckInFlight = false;
let connectionGeneration = 0;
let updateTimer;

const selectedAliasName = ref("code");
const aliasPageTab = ref("strategy");
const allAliases = computed(() => {
  const byName = new Map(aliases.value.map((item) => [item.alias, item]));
  for (const alias of customAliasNames.value) {
    if (!byName.has(alias)) {
      byName.set(alias, { alias, target_model: "未绑定模型", candidates: [] });
    }
  }
  return [...byName.values()].sort((a, b) => a.alias.localeCompare(b.alias));
});
const currentAlias = computed(
  () =>
    allAliases.value.find((item) => item.alias === "code") ||
    allAliases.value[0],
);
const selectedAlias = computed(
  () =>
    allAliases.value.find((item) => item.alias === selectedAliasName.value) ||
    currentAlias.value || {
      alias: selectedAliasName.value,
      target_model: "未绑定模型",
      candidates: [],
    },
);
function resolveAliasCredentials(alias) {
  return (alias?.candidates || []).map((candidate) => {
    const credential = credentials.value.find(
      (item) => item.id === candidate.credential_id,
    );
    return credential
      ? { ...credential, ...candidate, name: credential.name }
      : candidate;
  });
}
const selectedAliasCredentials = computed(() =>
  resolveAliasCredentials(selectedAlias.value),
);
const activeCredentials = computed(() =>
  credentials.value.filter((item) => item.enabled),
);
const credentialGroups = computed(() => [
  {
    id: "providers",
    title: "API 供应商",
    items: credentials.value.filter(
      (item) => item.config_section !== "auth-files",
    ),
  },
  {
    id: "auth-files",
    title: "授权文件",
    items: credentials.value.filter(
      (item) => item.config_section === "auth-files",
    ),
  },
]);
const activeCredentialGroups = computed(() =>
  credentialGroups.value.map((group) => ({
    ...group,
    items: group.items.filter((item) => item.enabled),
  })),
);
const aliasCredentialGroup = computed(() => ({
  id: "providers",
  title: "API 供应商",
  items: selectedAliasCredentials.value.filter(
    (item) => item.config_section !== "auth-files",
  ),
}));
const authFileCredentials = computed(() =>
  credentials.value.filter((item) => item.config_section === "auth-files"),
);
const providerStatusCredentials = computed(() => {
  const groups = new Map();
  for (const credential of manageableCredentials.value.filter(
    (item) => item.config_section !== "auth-files",
  )) {
    const key =
      credential.config_section === "openai-compatibility"
        ? `${credential.config_section}-${Math.floor(credential.config_index / 10000)}`
        : credential.id;
    if (!groups.has(key)) groups.set(key, credential);
  }
  return [...groups.values()];
});
const authStatusCredentials = computed(() =>
  manageableCredentials.value.filter(
    (item) => item.config_section === "auth-files",
  ),
);
const routeKeyOf = (item) =>
  `${Number(item?.priority) || 0}:${Number(item?.weight) || 0}`;
function routeWinner(items) {
  const active = items.filter((item) => item.enabled !== false);
  if (!active.length) return { item: null, tie: false };
  const sorted = [...active].sort(
    (a, b) =>
      (Number(b.priority) || 0) - (Number(a.priority) || 0) ||
      (Number(b.weight) || 0) - (Number(a.weight) || 0),
  );
  const top = sorted[0];
  const tie =
    sorted.filter((item) => routeKeyOf(item) === routeKeyOf(top)).length > 1;
  return { item: top, tie };
}
function isRouteActive(item, route) {
  if (!route?.item) return false;
  return route.tie
    ? routeKeyOf(item) === routeKeyOf(route.item)
    : item.id === route.item.id;
}
const authRouteSummary = computed(() => {
  const route = routeWinner(authFileCredentials.value);
  const item = route.item;
  return {
    ...route,
    label: item ? item.account_label || item.name : "暂无可用账号",
  };
});
const currentAuthAccountName = computed(() => {
  const label = authRouteSummary.value.label;
  return label.includes("@") ? label.slice(0, label.indexOf("@")) : label;
});
const aliasActiveRoute = computed(() =>
  routeWinner(aliasCredentialGroup.value.items),
);
const authActiveRoute = computed(() => routeWinner(authFileCredentials.value));
const aliasSearch = ref("");
const expandedCandidateProviders = reactive({});

function toggleCandidateProvider(credentialId) {
  expandedCandidateProviders[credentialId] =
    !expandedCandidateProviders[credentialId];
}

function collapseCandidateProviders() {
  for (const credentialId of Object.keys(expandedCandidateProviders)) {
    delete expandedCandidateProviders[credentialId];
  }
}

const boundAliasProviderGroups = computed(() => {
  const currentAlias = selectedAliasName.value;
  const list = [];
  for (const credential of credentials.value.filter(
    (item) => item.config_section !== "auth-files",
  )) {
    const boundModels = (credential.models || []).filter(
      (m) => m.alias === currentAlias,
    );
    if (boundModels.length > 0) {
      list.push({
        credential,
        service_url: credential.service_url || "默认服务地址",
        provider: credential.provider,
        priority: credential.priority ?? 0,
        weight: credential.weight ?? 1,
        models: boundModels,
      });
    }
  }
  return list.sort((a, b) => a.service_url.localeCompare(b.service_url));
});

const availableAliasProviderGroups = computed(() => {
  const currentAlias = selectedAliasName.value;
  const boundCredentialIds = new Set(
    boundAliasProviderGroups.value.map((g) => g.credential.id),
  );
  const keyword = aliasSearch.value.trim().toLowerCase();
  const list = [];

  for (const credential of credentials.value.filter(
    (item) => item.config_section !== "auth-files",
  )) {
    if (boundCredentialIds.has(credential.id)) continue;

    const models = (credential.models || []).filter((m) => {
      if (!m?.name) return false;
      if (m.alias) return false;
      if (!keyword) return true;
      return (
        m.name.toLowerCase().includes(keyword) ||
        (credential.service_url || "").toLowerCase().includes(keyword) ||
        (credential.provider || "").toLowerCase().includes(keyword)
      );
    });

    if (models.length > 0) {
      list.push({
        credential,
        service_url: credential.service_url || "默认服务地址",
        provider: credential.provider,
        models,
      });
    }
  }

  return list.sort((a, b) => a.service_url.localeCompare(b.service_url));
});
const authFileGroup = computed(() => ({
  id: "auth-files",
  title: "授权文件",
  items: authFileCredentials.value,
}));
const editableCredentials = computed(() =>
  page.value === "alias"
    ? aliasCredentialGroup.value.items.filter((item) => item.enabled !== false)
    : page.value === "auth"
      ? authFileCredentials.value.filter((item) => item.enabled !== false)
      : activeCredentials.value,
);
const strategyCredentialGroups = computed(() =>
  page.value === "auth"
    ? [
        {
          ...authFileGroup.value,
          items: authFileGroup.value.items.filter(
            (item) => item.enabled !== false,
          ),
        },
      ]
    : [
        {
          ...aliasCredentialGroup.value,
          items: aliasCredentialGroup.value.items.filter(
            (item) => item.enabled !== false,
          ),
        },
      ],
);
const aliasProviderPresets = computed(() =>
  [
    ...new Set(
      editableCredentials.value.map((item) => item.provider).filter(Boolean),
    ),
  ].sort(),
);
const strategyScope = computed(() =>
  page.value === "auth" ? "auth-files" : selectedAliasName.value,
);
const strategyScopeLabel = computed(() =>
  page.value === "auth" ? "授权文件" : selectedAliasName.value,
);
const selectedAliasStrategies = computed(() =>
  strategies.value.filter(
    (strategy) =>
      strategy.alias === strategyScope.value ||
      (!strategy.alias && strategyScope.value === "code"),
  ),
);
function ensureScopeStrategies(scope, credentialsList) {
  if (!credentialsList || !credentialsList.length) return;
  const deletedPresetIds = new Set(loadDeletedPresetIds());
  const existing = strategies.value.filter(
    (s) => s.alias === scope || (!s.alias && scope === "code"),
  );
  let changed = false;

  if (
    !existing.some((s) => s.name === "默认") &&
    !deletedPresetIds.has(`preset-${scope}-default`)
  ) {
    strategies.value.push({
      id: `preset-${scope}-default`,
      alias: scope,
      name: "默认",
      items: credentialsList.map((item) => ({
        credential_id: item.id,
        credential_name: item.name || item.account_label || item.id,
        weight: Number(item.weight ?? 1),
        priority: Number(item.priority ?? 0),
      })),
      created_at: new Date().toISOString(),
    });
    changed = true;
  }

  const providers = [
    ...new Set(credentialsList.map((item) => item.provider).filter(Boolean)),
  ];
  for (const provider of providers) {
    const pName = `${provider.toUpperCase()} 100%`;
    if (
      !existing.some(
        (s) => s.name === pName || s.name === provider.toUpperCase(),
      ) &&
      !deletedPresetIds.has(`preset-${scope}-${provider}`)
    ) {
      strategies.value.push({
        id: `preset-${scope}-${provider}`,
        alias: scope,
        name: pName,
        items: credentialsList.map((item) => ({
          credential_id: item.id,
          credential_name: item.name || item.account_label || item.id,
          weight: item.provider === provider ? 100 : 0,
          priority: 0,
        })),
        created_at: new Date().toISOString(),
      });
      changed = true;
    }
  }

  if (changed) {
    persistStrategies();
  }
}

function ensureAllScopeStrategies() {
  if (authFileCredentials.value.length) {
    ensureScopeStrategies("auth-files", authFileCredentials.value);
  }
  for (const alias of aliases.value) {
    const creds = resolveAliasCredentials(alias).filter(
      (c) => c.config_section !== "auth-files",
    );
    if (creds.length) {
      ensureScopeStrategies(alias.alias, creds);
    }
  }
}

function selectAlias(alias) {
  selectedAliasName.value = typeof alias === "string" ? alias : alias.alias;
  aliasSearch.value = "";
  aliasPageTab.value = "strategy";
  collapseCandidateProviders();
  showStrategyEditor.value = false;
  page.value = "alias";
  ensureScopeStrategies(
    selectedAliasName.value,
    aliasCredentialGroup.value.items,
  );
  const current = selectedAliasStrategies.value.find(
    (s) => s.id === selectedStrategyId.value,
  );
  if (!current) {
    const def =
      selectedAliasStrategies.value.find((s) => s.name === "默认") ||
      selectedAliasStrategies.value[0];
    selectedStrategyId.value = def ? def.id : null;
  }
}

function selectAuthFiles() {
  showStrategyEditor.value = false;
  page.value = "auth";
  ensureScopeStrategies("auth-files", authFileCredentials.value);
  const current = selectedAliasStrategies.value.find(
    (s) => s.id === selectedStrategyId.value,
  );
  if (!current) {
    const def =
      selectedAliasStrategies.value.find((s) => s.name === "默认") ||
      selectedAliasStrategies.value[0];
    selectedStrategyId.value = def ? def.id : null;
  }
}

function buildRouteChartGroup({ id, title, kind, items }) {
  const uniqueItems = [
    ...new Map(items.map((item) => [item.id, item])).values(),
  ];
  const winner = routeWinner(uniqueItems);
  const total = uniqueItems.reduce(
    (sum, item) => sum + (Number(item.weight) || 0),
    0,
  );

  return {
    id,
    title,
    kind,
    total,
    activeTie: winner.tie,
    items: [...uniqueItems]
      .sort((a, b) => (Number(b.weight) || 0) - (Number(a.weight) || 0))
      .map((item) => ({
        ...item,
        share: total ? ((Number(item.weight) || 0) / total) * 100 : 0,
        active: isRouteActive(item, winner),
      })),
  };
}

const routeChartGroups = computed(() => [
  ...allAliases.value.map((alias) =>
    buildRouteChartGroup({
      id: `alias-${alias.alias}`,
      title: alias.alias,
      kind: "alias",
      items: resolveAliasCredentials(alias).filter(
        (item) => item.enabled !== false,
      ),
    }),
  ),
  buildRouteChartGroup({
    id: "auth-files",
    title: "授权文件",
    kind: "auth",
    items: authFileCredentials.value.filter((item) => item.enabled !== false),
  }),
]);

const chartColors = [
  "#c69cff",
  "#72d8c7",
  "#f0be72",
  "#ff8f83",
  "#8bb8ff",
  "#d7f87b",
  "#ef8ed9",
  "#8bd5ff",
];
function poolGradient(pool) {
  if (!pool.total) return "#202a35";
  let cursor = 0;
  const segments = pool.items.map((item, index) => {
    const start = cursor;
    cursor += item.share;
    return `${chartColors[index % chartColors.length]} ${start}% ${cursor}%`;
  });
  return `conic-gradient(from -90deg, ${segments.join(", ")})`;
}
function chartColor(index) {
  return chartColors[index % chartColors.length];
}
function loadCustomAliasNames() {
  try {
    const stored = JSON.parse(
      localStorage.getItem("cpa-custom-aliases") || "[]",
    );
    return Array.isArray(stored)
      ? stored.filter((item) => typeof item === "string" && item.trim())
      : [];
  } catch {
    return [];
  }
}

function loadDeletedPresetIds() {
  try {
    const stored = JSON.parse(
      localStorage.getItem("cpa-deleted-preset-ids") || "[]",
    );
    return Array.isArray(stored)
      ? stored.filter((item) => typeof item === "string")
      : [];
  } catch {
    return [];
  }
}

function persistDeletedPresetIds(ids) {
  localStorage.setItem("cpa-deleted-preset-ids", JSON.stringify(ids));
}

function persistCustomAliasNames() {
  localStorage.setItem(
    "cpa-custom-aliases",
    JSON.stringify(customAliasNames.value),
  );
}

function openAliasCreator() {
  newAliasName.value = "";
  showAliasCreator.value = true;
}

function createAlias() {
  const alias = newAliasName.value.trim();
  if (!alias) return flash("error", "请输入模型别名");
  if (/\s/.test(alias)) return flash("error", "模型别名不能包含空格");
  if (allAliases.value.some((item) => item.alias === alias)) {
    return flash("error", `模型别名「${alias}」已存在`);
  }
  customAliasNames.value.push(alias);
  persistCustomAliasNames();
  showAliasCreator.value = false;
  selectAlias(alias);
  flash("success", `已创建模型别名「${alias}」，请添加供应商模型`);
}

async function deleteAlias(alias) {
  const aliasName = typeof alias === "string" ? alias : alias?.alias;
  if (!aliasName) return;
  if (
    !(await askConfirm({
      title: "删除别名",
      message: `确定删除「${aliasName}」并清空远程配置中的该别名？`,
      confirmLabel: "确认删除",
      danger: true,
    }))
  )
    return;

  loading.value = true;
  try {
    const affected = [];
    for (const credential of credentials.value.filter(
      (item) => item.config_section !== "auth-files",
    )) {
      for (const model of credential.models || []) {
        if (model.alias === aliasName) {
          affected.push({ credential, modelName: model.name });
        }
      }
    }

    let snapshot;
    for (const item of affected) {
      snapshot = await client.updateModelAlias(
        item.credential,
        item.modelName,
        "",
        true,
      );
    }

    customAliasNames.value = customAliasNames.value.filter(
      (name) => name !== aliasName,
    );
    persistCustomAliasNames();

    strategies.value = strategies.value.filter(
      (strategy) => strategy.alias !== aliasName,
    );
    persistStrategies();

    applyConfig(snapshot || (await client.snapshot()));
    if (selectedAliasName.value === aliasName) {
      selectedAliasName.value = "";
      selectedStrategyId.value = null;
      page.value = "dashboard";
    }
    flash("success", `别名「${aliasName}」已删除`);
  } catch (error) {
    flash("error", error.message);
  } finally {
    loading.value = false;
  }
}

function loadStrategies() {
  try {
    const stored = JSON.parse(
      localStorage.getItem("cpa-weight-strategies") || "[]",
    );
    return Array.isArray(stored)
      ? stored.filter((item) => item?.name !== "均衡")
      : [];
  } catch {
    return [];
  }
}

function persistStrategies() {
  localStorage.setItem(
    "cpa-weight-strategies",
    JSON.stringify(strategies.value),
  );
}

function flash(type, message) {
  notice.value = { type, message };
  setTimeout(() => {
    notice.value = null;
  }, 3500);
}

function askConfirm({ title, message, confirmLabel = "确认", danger = false }) {
  confirmDialog.value = { title, message, confirmLabel, danger };
  return new Promise((resolve) => {
    confirmResolver = resolve;
  });
}

function resolveConfirm(value) {
  confirmDialog.value = null;
  confirmResolver?.(value);
  confirmResolver = null;
}

function applyConfig(snapshot) {
  const normalized = normalizeSnapshot(snapshot.config, snapshot.authFiles);
  const manageable = normalizeSnapshot(snapshot.config, snapshot.authFiles, {
    includeDisabled: true,
  });
  credentials.value = normalized.credentials;
  manageableCredentials.value = manageable.credentials;
  aliases.value = normalized.aliases;
  for (const item of normalized.credentials) {
    draftWeights[item.id] = item.weight;
    draftPriorities[item.id] = item.priority;
  }
  draftDirty.value = false;
  strategies.value = strategies.value.filter((s) => s.name !== "均衡");
  persistStrategies();
  ensureAllScopeStrategies();
}

function errorMessage(error) {
  if (error instanceof Error) return error.message;
  return String(error || "连接失败");
}

function startRefreshTimer() {
  clearInterval(refreshTimer);
  refreshTimer = setInterval(() => refresh(true), 10000);
}

function resetReconnectState() {
  connectionGeneration += 1;
  clearTimeout(reconnectTimer);
  reconnectTimer = undefined;
  reconnectAttempts = 0;
  reconnecting.value = false;
  connectionCheckInFlight = false;
}

function reconnectDelay() {
  if (reconnectAttempts === 0) return 1500;
  return Math.min(2000 * 2 ** Math.min(reconnectAttempts - 1, 4), 30000);
}

function scheduleReconnect() {
  clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(() => attemptReconnect(), reconnectDelay());
}

function markConnectionInterrupted(error) {
  const firstFailure = !reconnecting.value;
  connected.value = false;
  reconnecting.value = true;
  if (firstFailure) {
    flash("error", `CPA 连接中断，正在自动重连：${errorMessage(error)}`);
  }
  scheduleReconnect();
}

async function attemptReconnect(manual = false) {
  if (!sessionAvailable.value) return;
  if (connectionCheckInFlight) {
    if (!reconnectTimer) scheduleReconnect();
    return;
  }

  clearTimeout(reconnectTimer);
  reconnectTimer = undefined;
  const generation = connectionGeneration;
  connectionCheckInFlight = true;
  if (manual) loading.value = true;
  try {
    const snapshot = await client.snapshot();
    if (generation !== connectionGeneration) return;
    if (!draftDirty.value) applyConfig(snapshot);
    reconnectAttempts = 0;
    reconnecting.value = false;
    connected.value = true;
    startRefreshTimer();
    flash(
      "success",
      draftDirty.value
        ? "CPA 已重新连接，未保存修改已保留"
        : "CPA 已重新连接并刷新配置",
    );
  } catch (error) {
    if (generation !== connectionGeneration) return;
    reconnectAttempts += 1;
    connected.value = false;
    reconnecting.value = true;
    if (manual) flash("error", `重连失败：${errorMessage(error)}`);
    scheduleReconnect();
  } finally {
    if (generation === connectionGeneration) connectionCheckInFlight = false;
    if (manual) loading.value = false;
  }
}

async function connect() {
  if (!managementKey.value) return flash("error", "请输入 管理密钥");
  resetReconnectState();
  loading.value = true;
  try {
    await client.connect(
      managementUrl.value,
      managementKey.value,
      rememberKey.value,
    );
    const config = await client.snapshot();
    localStorage.setItem("cpa-management-url", managementUrl.value);
    if (!desktop) {
      if (rememberKey.value)
        sessionStorage.setItem("cpa-management-key", managementKey.value);
      else sessionStorage.removeItem("cpa-management-key");
    }
    applyConfig(config);
    connected.value = true;
    page.value = "dashboard";
    startRefreshTimer();
    flash(
      "success",
      `已连接 CPA，加载 ${credentials.value.length} 个 Credential`,
    );
  } catch (error) {
    connected.value = false;
    reconnecting.value = false;
    flash("error", error.message);
  } finally {
    loading.value = false;
  }
}

async function disconnect() {
  if (
    !(await askConfirm({
      title: "退出连接",
      message: draftDirty.value
        ? "退出并丢弃未保存修改？"
        : "退出当前 CPA 连接？",
      confirmLabel: "确认退出",
      danger: true,
    }))
  )
    return;

  resetReconnectState();
  loading.value = true;
  try {
    await client.disconnect();
    clearInterval(refreshTimer);
    refreshTimer = undefined;
    sessionStorage.removeItem("cpa-management-key");
    managementKey.value = "";
    rememberKey.value = false;
    connected.value = false;
    credentials.value = [];
    manageableCredentials.value = [];
    showCredentialStatus.value = false;
    aliases.value = [];
    selectedAliasName.value = "";
    selectedStrategyId.value = null;
    draftDirty.value = false;
    for (const key of Object.keys(draftWeights)) delete draftWeights[key];
    for (const key of Object.keys(draftPriorities)) delete draftPriorities[key];
    page.value = "settings";
    flash("success", "已退出 CPA 连接");
  } catch (error) {
    flash("error", error.message);
  } finally {
    loading.value = false;
  }
}

async function openCpaWeb() {
  try {
    await openExternalUrl(cpaWebUrl.value);
  } catch (error) {
    flash("error", error.message || "无法打开系统浏览器");
  }
}

function openCpaSite() {
  if (desktop) {
    page.value = "web";
    return;
  }
  openCpaWeb();
}

async function copyManagementKey() {
  try {
    const key = managementKey.value || (await client.currentManagementKey());
    if (!key) throw new Error("当前连接没有可复制的管理密钥");
    await navigator.clipboard.writeText(key);
    flash("success", "管理密钥已复制");
  } catch (error) {
    flash("error", error.message || "复制失败，请手动输入管理密钥");
  }
}

function versionParts(version) {
  return String(version || "")
    .replace(/^v/i, "")
    .split(/[.-]/)
    .map((part) => Number.parseInt(part, 10) || 0);
}

function isNewerVersion(latest, current) {
  const left = versionParts(latest);
  const right = versionParts(current);
  const length = Math.max(left.length, right.length);
  for (let index = 0; index < length; index += 1) {
    const diff = (left[index] || 0) - (right[index] || 0);
    if (diff !== 0) return diff > 0;
  }
  return false;
}

async function checkForUpdate(notify = false) {
  if (checkingForUpdate.value) return;
  checkingForUpdate.value = true;
  try {
    const endpoint = new URL(
      "https://api.github.com/repos/chansanya/cpa-route-control/releases/latest",
    );
    endpoint.searchParams.set("_", Date.now().toString());
    const response = await fetch(endpoint, {
      cache: "no-store",
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!response.ok) {
      throw new Error(`GitHub Releases API 返回 HTTP ${response.status}`);
    }
    const release = await response.json();
    if (!release?.tag_name) throw new Error("GitHub Release 缺少版本标签");
    if (!isNewerVersion(release.tag_name, `v${appVersion}`)) {
      updateInfo.value = null;
      if (notify) flash("success", `当前 v${appVersion} 已是最新版本`);
      return;
    }
    const assets = Array.isArray(release.assets) ? release.assets : [];
    const asset =
      assets.find((item) => /setup\.exe$/i.test(item.name)) ||
      assets.find((item) => /\.exe$/i.test(item.name));
    updateInfo.value = {
      version: release.tag_name,
      url: asset?.browser_download_url || release.html_url,
    };
    if (notify) flash("success", `发现新版本 ${release.tag_name}`);
  } catch (error) {
    if (notify) flash("error", error.message || "检查更新失败");
  } finally {
    checkingForUpdate.value = false;
  }
}

async function openUpdateDownload() {
  if (!updateInfo.value?.url) return;
  try {
    await openExternalUrl(updateInfo.value.url);
  } catch (error) {
    flash("error", error.message || "无法打开更新下载页面");
  }
}

function credentialStatusName(credential) {
  return credential.config_section === "auth-files"
    ? credential.account_label || credential.name
    : credential.service_url || credential.name;
}

async function toggleCredentialEnabled(credential) {
  if (credential.unavailable && !credential.disabled) {
    return flash("error", "该授权文件当前不可用，无法手动启用");
  }
  const enabled = credential.enabled === false;
  const action = enabled ? "启用" : "禁用";
  if (
    !(await askConfirm({
      title: `${action}凭据`,
      message: `${action}「${credentialStatusName(credential)}」？`,
      confirmLabel: `确认${action}`,
      danger: !enabled,
    }))
  )
    return;

  loading.value = true;
  try {
    applyConfig(await client.setCredentialEnabled(credential, enabled));
    flash("success", `已${action}「${credentialStatusName(credential)}」`);
  } catch (error) {
    flash("error", error.message);
  } finally {
    loading.value = false;
  }
}

async function refresh(silent = false) {
  if (reconnecting.value) {
    if (!silent) await attemptReconnect(true);
    return;
  }
  if (!connected.value || connectionCheckInFlight) return;
  if (silent && loading.value) return;
  if (draftDirty.value) {
    if (!silent) flash("error", "存在未保存修改，请先应用后再刷新");
    return;
  }

  const generation = connectionGeneration;
  connectionCheckInFlight = true;
  if (!silent) loading.value = true;
  try {
    const snapshot = await client.snapshot();
    if (generation !== connectionGeneration) return;
    applyConfig(snapshot);
    if (!silent) flash("success", "已从 CPA 重新拉取配置");
  } catch (error) {
    if (generation === connectionGeneration) markConnectionInterrupted(error);
  } finally {
    if (generation === connectionGeneration) connectionCheckInFlight = false;
    if (!silent) loading.value = false;
  }
}

function isRemoteModelMissing(error) {
  const message = errorMessage(error);
  return (
    /模型 .+ 不存在/.test(message) ||
    /OpenAI 兼容模型 .+ 不存在/.test(message) ||
    /openai-compatibility\[\d+\].*不存在/.test(message)
  );
}

function isConnectionFailure(error) {
  const message = errorMessage(error);
  return /CPA (connection|response) failed|Failed to fetch|NetworkError|HTTP (401|403|408|429|5\d\d)/i.test(
    message,
  );
}

async function handleAliasMutationError(error) {
  if (isConnectionFailure(error)) {
    markConnectionInterrupted(error);
    return;
  }
  if (!isRemoteModelMissing(error)) {
    flash("error", errorMessage(error));
    return;
  }

  try {
    applyConfig(await client.snapshot());
    flash(
      "error",
      `${errorMessage(error)}。远程配置已变化，已刷新本地列表，本次操作未写入。`,
    );
  } catch (refreshError) {
    markConnectionInterrupted(refreshError);
  }
}

async function assignAlias(credential, modelName) {
  if (
    !(await askConfirm({
      title: "设置别名",
      message: `将「${modelName}」设置为「${selectedAliasName.value}」？`,
      confirmLabel: "确认设置",
    }))
  )
    return;
  loading.value = true;
  try {
    applyConfig(
      await client.updateModelAlias(
        credential,
        modelName,
        selectedAliasName.value,
        true,
      ),
    );
    flash("success", `已将 ${modelName} 设置为 ${selectedAliasName.value}`);
  } catch (error) {
    await handleAliasMutationError(error);
  } finally {
    loading.value = false;
  }
}

async function removeAlias(credential, modelName) {
  if (
    !(await askConfirm({
      title: "移除别名",
      message: `移除「${modelName}」的「${selectedAliasName.value}」别名？`,
      confirmLabel: "确认移除",
      danger: true,
    }))
  )
    return;
  loading.value = true;
  try {
    applyConfig(await client.updateModelAlias(credential, modelName, "", true));
    flash("success", `已移除 ${modelName} 的别名`);
  } catch (error) {
    await handleAliasMutationError(error);
  } finally {
    loading.value = false;
  }
}

async function applySettings(items, label) {
  if (!items.length) {
    flash("error", "没有可修改的 Credential");
    return false;
  }
  if (
    !(await askConfirm({
      title: "确认应用",
      message: `应用「${label}」？`,
      confirmLabel: "确认应用",
    }))
  )
    return false;
  loading.value = true;
  try {
    let snapshot;
    const weights = items
      .filter((item) => item.weight !== undefined)
      .map((item) => ({
        credential_id: item.credential_id,
        weight: item.weight,
      }));
    const priorities = items
      .filter((item) => item.priority !== undefined)
      .map((item) => ({
        credential_id: item.credential_id,
        priority: item.priority,
      }));
    if (weights.length)
      snapshot = await client.setWeights(credentials.value, weights);
    if (priorities.length)
      snapshot = await client.setPriorities(credentials.value, priorities);
    applyConfig(snapshot || (await client.snapshot()));
    flash("success", `${label} 已写入 CPA 并热重载`);
    return true;
  } catch (error) {
    flash("error", error.message);
    return false;
  } finally {
    loading.value = false;
  }
}

function applyWeights(items, label) {
  return applySettings(items, label);
}

function presetLabel(provider) {
  return provider === "balanced" ? "均衡" : `${provider.toUpperCase()} 100%`;
}

function applyPresetToCredentials(provider, source, aliasName) {
  const active = source.filter((item) => item.enabled !== false);
  if (
    provider !== "balanced" &&
    !active.some((item) => item.provider === provider)
  )
    return flash("error", `${aliasName} 没有可用的 ${provider} Credential`);
  const items = active.map((item) => ({
    credential_id: item.id,
    weight: provider === "balanced" ? 1 : item.provider === provider ? 100 : 0,
  }));
  applyWeights(items, `${aliasName} · ${presetLabel(provider)}`);
}

function setPreset(provider) {
  const active = editableCredentials.value;
  if (
    provider !== "balanced" &&
    !active.some((item) => item.provider === provider)
  )
    return flash("error", `当前 CPA 没有可用的 ${provider} Credential`);
  const items = active.map((item) => ({
    credential_id: item.id,
    weight: provider === "balanced" ? 1 : item.provider === provider ? 100 : 0,
  }));
  applyWeights(items, presetLabel(provider));
}

async function saveDraftSettings() {
  const items = editableCredentials.value.map((item) => ({
    credential_id: item.id,
    credential_name: item.name || item.account_label || item.id,
    weight: Number(draftWeights[item.id]),
    priority: Number(draftPriorities[item.id]),
  }));
  if (
    items.some(
      (item) =>
        !Number.isInteger(item.weight) ||
        item.weight < 0 ||
        item.weight > 1000000,
    )
  )
    return flash("error", "权重必须是 0 到 1,000,000 的整数");
  if (items.some((item) => !Number.isInteger(item.priority)))
    return flash("error", "优先级必须是整数");
  if (!items.some((item) => item.weight > 0))
    return flash("error", "至少保留一个大于 0 的权重");
  const ok = await applySettings(items, "优先级 / 权重设置");
  if (!ok) return;
  draftDirty.value = false;

  let targetId = selectedStrategyId.value;
  let index = strategies.value.findIndex((item) => item.id === targetId);
  if (index === -1) {
    index = strategies.value.findIndex(
      (item) =>
        (item.alias === strategyScope.value ||
          (!item.alias && strategyScope.value === "code")) &&
        item.name === "默认",
    );
  }
  if (index !== -1) {
    strategies.value[index] = {
      ...strategies.value[index],
      items,
      updated_at: new Date().toISOString(),
    };
    selectedStrategyId.value = strategies.value[index].id;
    persistStrategies();
    flash(
      "success",
      `已写入 CPA 并同步保存到策略「${strategies.value[index].name}」`,
    );
  }
}

function selectStrategy(strategy) {
  if (!strategy) return;
  selectedStrategyId.value = strategy.id;
  for (const item of editableCredentials.value) {
    draftWeights[item.id] = 0;
    draftPriorities[item.id] = 0;
  }
  for (const item of strategy.items || []) {
    const cid = item.credential_id || item.id;
    if (
      draftWeights[cid] !== undefined ||
      editableCredentials.value.some((c) => c.id === cid)
    ) {
      draftWeights[cid] = Number(item.weight ?? 0);
      draftPriorities[cid] = Number(item.priority ?? 0);
    }
  }
  draftDirty.value = true;
}

function openStrategyEditor(alias = selectedAlias.value, strategy = null) {
  if (alias?.alias && typeof alias === "object")
    selectedAliasName.value = alias.alias;
  editingStrategyId.value = strategy?.id || null;
  strategyName.value = strategy?.name || "";
  const source = strategy?.items || editableCredentials.value;
  const available = strategy
    ? source.filter((item) =>
        editableCredentials.value.some(
          (current) => current.id === (item.credential_id || item.id),
        ),
      )
    : source;
  for (const key of Object.keys(strategyWeights)) delete strategyWeights[key];
  for (const key of Object.keys(strategyPriorities))
    delete strategyPriorities[key];
  for (const item of available) {
    const credentialId = item.credential_id || item.id;
    strategyWeights[credentialId] = Number(item.weight ?? item.priority ?? 0);
    strategyPriorities[credentialId] = Number(item.priority ?? 0);
  }
  for (const cred of editableCredentials.value) {
    if (strategyWeights[cred.id] === undefined) {
      strategyWeights[cred.id] = Number(
        draftWeights[cred.id] ?? cred.weight ?? 1,
      );
      strategyPriorities[cred.id] = Number(
        draftPriorities[cred.id] ?? cred.priority ?? 0,
      );
    }
  }
  showStrategyEditor.value = true;
}

function saveStrategy() {
  const name = strategyName.value.trim();
  if (!name) return flash("error", "请输入策略名称");
  const items = editableCredentials.value.map((item) => ({
    credential_id: item.id,
    credential_name: item.name || item.account_label || item.id,
    weight: Number(strategyWeights[item.id]),
    priority: Number(strategyPriorities[item.id]),
  }));
  if (
    items.some(
      (item) =>
        !Number.isInteger(item.weight) ||
        item.weight < 0 ||
        item.weight > 1000000,
    )
  )
    return flash("error", "策略权重必须是 0 到 1,000,000 的整数");
  if (items.some((item) => !Number.isInteger(item.priority)))
    return flash("error", "策略优先级必须是整数");
  if (!items.some((item) => item.weight > 0))
    return flash("error", "策略至少保留一个大于 0 的权重");
  if (editingStrategyId.value) {
    const index = strategies.value.findIndex(
      (item) => item.id === editingStrategyId.value,
    );
    if (index === -1) return flash("error", "原策略不存在，请重新新增");
    strategies.value[index] = {
      ...strategies.value[index],
      name,
      items,
      updated_at: new Date().toISOString(),
    };
    selectedStrategyId.value = editingStrategyId.value;
  } else {
    const created = {
      id: crypto.randomUUID(),
      alias: strategyScope.value,
      name,
      items,
      created_at: new Date().toISOString(),
    };
    strategies.value.push(created);
    selectedStrategyId.value = created.id;
  }
  persistStrategies();
  for (const item of items) {
    draftWeights[item.credential_id] = item.weight;
    draftPriorities[item.credential_id] = item.priority;
  }
  editingStrategyId.value = null;
  showStrategyEditor.value = false;
  flash("success", `策略「${name}」已保存并更新当前参数`);
}

async function deleteEditingStrategy() {
  if (!editingStrategyId.value) return;
  const target = strategies.value.find((s) => s.id === editingStrategyId.value);
  if (!target) return;
  if (
    !(await askConfirm({
      title: "删除本地策略",
      message: `确定删除策略「${target.name}」？`,
      confirmLabel: "确认删除",
      danger: true,
    }))
  )
    return;
  strategies.value = strategies.value.filter((s) => s.id !== target.id);
  if (selectedStrategyId.value === target.id) {
    selectedStrategyId.value = null;
  }
  if (typeof target.id === "string" && target.id.startsWith("preset-")) {
    const deleted = new Set(loadDeletedPresetIds());
    deleted.add(target.id);
    persistDeletedPresetIds([...deleted]);
  }
  persistStrategies();
  showStrategyEditor.value = false;
  editingStrategyId.value = null;
  flash("success", `策略「${target.name}」已删除`);
}

function applyStrategy(strategy) {
  const items = strategy.items || strategy.weights || [];
  const missing = items.filter(
    (item) =>
      !credentials.value.some(
        (credential) => credential.id === item.credential_id,
      ),
  );
  if (missing.length)
    return flash(
      "error",
      `策略已失效，缺少：${missing.map((item) => item.credential_name).join("、")}`,
    );
  applySettings(items, strategy.name);
}

async function deleteStrategy(strategy) {
  if (
    !(await askConfirm({
      title: "删除本地策略",
      message: `确定删除「${strategy.name}」？`,
      confirmLabel: "删除策略",
      danger: true,
    }))
  )
    return;
  strategies.value = strategies.value.filter((item) => item.id !== strategy.id);
  if (selectedStrategyId.value === strategy.id) {
    selectedStrategyId.value = null;
  }
  if (typeof strategy.id === "string" && strategy.id.startsWith("preset-")) {
    const deleted = new Set(loadDeletedPresetIds());
    deleted.add(strategy.id);
    persistDeletedPresetIds([...deleted]);
  }
  persistStrategies();
  flash("success", `策略「${strategy.name}」已删除`);
}

const bootstrapSourceLabels = {
  install_env: "安装目录 .env",
  process_env: "系统环境变量",
  credential_manager: "Windows Credential Manager",
};

onUnmounted(() => {
  clearInterval(refreshTimer);
  clearTimeout(reconnectTimer);
  clearInterval(updateTimer);
});
onMounted(async () => {
  checkForUpdate();
  updateTimer = setInterval(() => checkForUpdate(), 30 * 60 * 1000);
  if (!desktop) return;
  loading.value = true;
  try {
    const result = await client.bootstrap(managementUrl.value);
    if (result.status === "connected") {
      managementUrl.value = result.baseUrl;
      managementKey.value = "";
      localStorage.setItem("cpa-management-url", result.baseUrl);
      applyConfig({
        config: result.config,
        authFiles: await client.authFiles(),
      });
      resetReconnectState();
      connected.value = true;
      page.value = "dashboard";
      startRefreshTimer();
      const source = bootstrapSourceLabels[result.source] || "自动配置";
      flash("success", `已从 ${source} 自动连接 CPA`);
      return;
    }
    if (result.status === "failed") {
      if (result.baseUrl) managementUrl.value = result.baseUrl;
      const source = bootstrapSourceLabels[result.source] || "自动配置";
      flash("error", `${source} 连接失败：${result.message}`);
    }
  } catch (error) {
    flash("error", error.message);
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <div class="shell">
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark"><Command :size="17" /></div>
        <div><strong>CPA</strong><span>DIRECT CONTROL</span></div>
      </div>
      <div class="instance">
        <div class="instance-button">
          <span
            class="status-dot"
            :class="{ offline: !sessionAvailable, reconnecting }"
          ></span>
          <span class="instance-status-text">{{
            reconnecting
              ? "CPA 重连中"
              : connected
                ? "CPA 已连接"
                : "CPA 未连接"
          }}</span>
        </div>
        <div v-if="sessionAvailable" class="instance-actions">
          <button
            v-if="reconnecting"
            class="instance-action"
            title="立即重连"
            aria-label="立即重连"
            :disabled="loading"
            @click="attemptReconnect(true)"
          >
            <RefreshCw :size="14" class="spin" />
          </button>
          <button
            class="instance-action"
            title="复制管理密钥"
            aria-label="复制管理密钥"
            @click="copyManagementKey"
          >
            <Copy :size="14" />
          </button>
          <button
            class="instance-action"
            title="在系统浏览器打开 CPA 管理后台"
            aria-label="在系统浏览器打开 CPA 管理后台"
            @click="openCpaWeb"
          >
            <ExternalLink :size="14" />
          </button>
        </div>
      </div>
      <nav>
        <template v-if="sessionAvailable">
          <div class="nav-label">CONTROL CENTER</div>
          <button
            class="nav-item"
            :class="{ active: page === 'dashboard' }"
            @click="page = 'dashboard'"
          >
            <LayoutDashboard :size="17" /><span>控制台</span
            ><ArrowUpRight
              v-if="page === 'dashboard'"
              :size="14"
              class="nav-arrow"
            />
          </button>
          <button
            class="nav-item"
            :class="{ active: page === 'web' }"
            @click="openCpaSite"
          >
            <ExternalLink :size="17" /><span>CPA 网站</span
            ><ArrowUpRight v-if="page === 'web'" :size="14" class="nav-arrow" />
          </button>
        </template>
        <template v-else>
          <div class="nav-label">CONNECT</div>
          <button class="nav-item active" @click="page = 'settings'">
            <Settings2 :size="17" /><span>CPA 连接</span>
          </button>
        </template>
      </nav>
      <div class="sidebar-foot">
        <div class="daemon">
          <span
            class="pulse"
            :class="{ offline: !sessionAvailable, reconnecting }"
          ></span
          ><span>OFFICIAL API</span
          ><b>{{
            reconnecting ? "RETRYING" : connected ? "ONLINE" : "OFFLINE"
          }}</b>
        </div>
        <button
          class="version version-check"
          :disabled="checkingForUpdate"
          title="检查新版本"
          aria-label="检查新版本"
          @click="checkForUpdate(true)"
        >
          DIRECT CLIENT
          <span>{{ checkingForUpdate ? "CHECKING" : `v${appVersion}` }}</span>
        </button>
        <button
          v-if="updateInfo"
          class="sidebar-update"
          :title="`下载新版本 ${updateInfo.version}`"
          :aria-label="`下载新版本 ${updateInfo.version}`"
          @click="openUpdateDownload"
        >
          <Download :size="12" />
          <span>更新至 {{ updateInfo.version }}</span>
        </button>
      </div>
    </aside>

    <main class="main" :class="{ 'web-shell': page === 'web' }">
      <header v-if="page !== 'web'" class="topbar">
        <div>
          <div class="breadcrumb">
            CLI PROXY API <span>/</span>
            {{
              page === "dashboard"
                ? "DASHBOARD"
                : page === "alias"
                  ? `ALIAS · ${selectedAliasName}`
                  : page === "auth"
                    ? "AUTH FILES"
                    : page === "web"
                      ? "CPA WEB"
                      : "SETTINGS"
            }}
          </div>
          <h1>
            {{
              page === "dashboard"
                ? "控制台"
                : page === "alias"
                  ? `权重控制`
                  : page === "auth"
                    ? "授权文件权重"
                    : page === "web"
                      ? "CPA 网站"
                      : "连接 CPA"
            }}
          </h1>
        </div>
        <div class="top-actions">
          <button
            v-if="updateInfo"
            class="icon-button update-button"
            :title="`新版本 ${updateInfo.version}`"
            aria-label="下载新版本"
            @click="openUpdateDownload"
          >
            <Download :size="17" />
          </button>
          <button
            v-if="page === 'web'"
            class="icon-button"
            @click="openCpaWeb"
            title="在系统浏览器打开"
          >
            <ExternalLink :size="17" />
          </button>
          <button
            v-if="connected"
            class="icon-button"
            @click="showCredentialStatus = true"
            title="启用状态"
            aria-label="管理供应商和授权文件启用状态"
          >
            <SlidersHorizontal :size="17" />
          </button>
          <button
            class="icon-button"
            :disabled="!sessionAvailable || loading"
            @click="refresh()"
            title="从 CPA 刷新"
          >
            <RefreshCw :size="17" :class="{ spin: loading }" />
          </button>
          <button
            v-if="sessionAvailable"
            class="icon-button logout-icon"
            :disabled="loading"
            @click="disconnect"
            title="退出连接"
            aria-label="退出连接"
          >
            <LogOut :size="17" />
          </button>
        </div>
      </header>
      <div v-if="notice" :class="['notice', notice.type]">
        <Check v-if="notice.type === 'success'" :size="16" /><CircleAlert
          v-else
          :size="16"
        />{{ notice.message
        }}<button @click="notice = null"><X :size="14" /></button>
      </div>
      <ConfirmDialog
        :confirm-dialog="confirmDialog"
        @close="resolveConfirm(false)"
        @confirm="resolveConfirm(true)"
      />
      <StrategyEditorModal
        v-if="showStrategyEditor"
        :editing-strategy-id="editingStrategyId"
        :strategy-scope-label="strategyScopeLabel"
        :strategy-name="strategyName"
        :strategy-credential-groups="strategyCredentialGroups"
        :strategy-priorities="strategyPriorities"
        :strategy-weights="strategyWeights"
        @close="showStrategyEditor = false"
        @delete="deleteEditingStrategy"
        @save="saveStrategy"
        @update:strategy-name="strategyName = $event"
        @update:priority="
          (id, value) =>
            (strategyPriorities[id] = value === '' ? '' : Number(value))
        "
        @update:weight="
          (id, value) =>
            (strategyWeights[id] = value === '' ? '' : Number(value))
        "
      />

      <AliasCreatorModal
        v-if="showAliasCreator"
        :new-alias-name="newAliasName"
        @close="showAliasCreator = false"
        @update:new-alias-name="newAliasName = $event"
        @create="createAlias"
      />

      <CredentialStatusModal
        v-if="showCredentialStatus"
        :provider-status-credentials="providerStatusCredentials"
        :auth-status-credentials="authStatusCredentials"
        :credential-status-tab="credentialStatusTab"
        :loading="loading"
        :status-name="credentialStatusName"
        @close="showCredentialStatus = false"
        @update:tab="credentialStatusTab = $event"
        @toggle="toggleCredentialEnabled"
      />

      <ConnectionPanel
        v-if="page === 'settings'"
        :management-url="managementUrl"
        :management-key="managementKey"
        :remember-key="rememberKey"
        :desktop="desktop"
        :loading="loading"
        @update:management-url="managementUrl = $event"
        @update:management-key="managementKey = $event"
        @update:remember-key="rememberKey = $event"
        @connect="connect"
      />

      <section v-else-if="page === 'dashboard'" class="content dashboard">
        <div class="hero-grid">
          <div class="hero-card alias-overview-card">
            <section class="entry-group alias-entry-group">
              <div class="entry-group-head">
                <span>模型别名</span>
                <button
                  class="mini-action alias-add-button"
                  @click="openAliasCreator"
                >
                  <Plus :size="14" />新增别名
                </button>
              </div>
              <div class="hero-alias-list">
                <div
                  v-for="alias in allAliases"
                  :key="alias.alias"
                  class="alias-preset-shell"
                >
                  <button
                    class="dashboard-preset alias-preset"
                    :title="`${alias.target_model || '未绑定模型'} · ${alias.candidates?.length || 0} 项`"
                    @click="selectAlias(alias)"
                  >
                    <span>{{ alias.alias }}</span
                    ><small>{{ alias.target_model || "未绑定模型" }}</small>
                  </button>
                  <span
                    class="alias-delete"
                    title="删除别名"
                    @click.stop="deleteAlias(alias)"
                    >×</span
                  >
                </div>
              </div>
              <div v-if="!allAliases.length" class="group-empty compact">
                暂无模型别名
              </div>
            </section>
            <section class="entry-group auth-entry-group">
              <div class="entry-group-head auth"><span>授权文件</span></div>
              <button
                class="dashboard-preset alias-preset auth-file-preset"
                title="授权文件不参与模型别名，独立管理优先级 / 权重"
                @click="selectAuthFiles"
              >
                <span>授权文件</span>
                <small :title="authRouteSummary.label">{{
                  currentAuthAccountName
                }}</small>
                <b class="auth-count-badge" title="授权文件账号数量">{{
                  authFileCredentials.length
                }}</b>
              </button>
            </section>
          </div>
        </div>
        <div class="section-head">
          <div>
            <h2>权重分布</h2>
          </div>
          <button class="text-button" @click="refresh()">
            刷新配置 <RefreshCw :size="14" />
          </button>
        </div>
        <div class="routing-chart panel">
          <div class="routing-chart-head">
            <div>
              <h3>权重占比</h3>
            </div>
            <span class="chart-total">模型别名 / 授权文件</span>
          </div>
          <div class="donut-grid">
            <article
              v-for="group in routeChartGroups"
              :key="group.id"
              class="donut-card"
            >
              <div
                class="donut-visual"
                :style="{ background: poolGradient(group) }"
              >
                <div class="donut-hole">
                  <small>{{
                    group.kind === "alias" ? "模型别名" : "授权文件"
                  }}</small
                  ><strong>{{ group.items.length }}</strong
                  ><span>总权重 {{ group.total }}</span>
                </div>
              </div>
              <div class="donut-detail">
                <div class="donut-title">
                  <span>{{
                    group.kind === "alias"
                      ? `别名 · ${group.title}`
                      : group.title
                  }}</span
                  ><b
                    >{{ group.items.length }}
                    {{ group.kind === "alias" ? "个供应商" : "个账号" }}</b
                  >
                </div>
                <div v-if="group.items.length" class="donut-legend">
                  <div
                    v-for="(item, index) in group.items"
                    :key="item.id"
                    class="donut-legend-row"
                    :class="{ active: item.active }"
                  >
                    <i :style="{ background: chartColor(index) }"></i
                    ><span :title="item.name">{{ item.name }}</span
                    ><small
                      >优先级 {{ item.priority }} · 权重
                      {{ item.weight }}</small
                    >
                    <strong v-if="item.active && group.activeTie">轮询</strong>
                  </div>
                </div>
                <div v-else class="donut-empty">暂无数据</div>
              </div>
            </article>
          </div>
        </div>
      </section>

      <section v-else-if="page === 'alias'" class="content alias-page">
        <div class="alias-action-bar">
          <button class="back-pill-btn" @click="page = 'dashboard'">
            <ArrowLeft :size="14" />
            <span>返回控制台</span>
          </button>
        </div>

        <div class="alias-page-tabs" role="tablist" aria-label="别名管理">
          <button
            role="tab"
            :aria-selected="aliasPageTab === 'strategy'"
            :class="{ active: aliasPageTab === 'strategy' }"
            @click="aliasPageTab = 'strategy'"
          >
            快捷策略
          </button>
          <button
            role="tab"
            :aria-selected="aliasPageTab === 'models'"
            :class="{ active: aliasPageTab === 'models' }"
            @click="aliasPageTab = 'models'"
          >
            模型绑定
          </button>
        </div>

        <div class="alias-two-col">
          <div v-if="aliasPageTab === 'strategy'" class="alias-col-weights">
            <div class="alias-quick panel">
              <div class="panel-head">
                <div>
                  <h3>快捷策略（{{ strategyScopeLabel }}）</h3>
                </div>
                <button class="mini-action" @click="openStrategyEditor">
                  <Plus :size="14" />新增策略
                </button>
              </div>
              <div class="strategy-card-grid">
                <button
                  v-for="strategy in selectedAliasStrategies"
                  :key="strategy.id"
                  class="dashboard-preset strategy-preset"
                  :class="{
                    selected: selectedStrategyId === strategy.id,
                    balanced: strategy.name.includes('均衡'),
                  }"
                  @click="selectStrategy(strategy)"
                  @dblclick="openStrategyEditor(selectedAlias, strategy)"
                >
                  <span>{{ strategy.name }}</span>
                  <small>双击编辑</small>
                  <span
                    class="strategy-delete"
                    title="删除策略"
                    @click.stop="deleteStrategy(strategy)"
                  >
                    ×
                  </span>
                </button>
              </div>
            </div>

            <div class="panel routing-panel">
              <div class="panel-head">
                <div>
                  <h3>设置</h3>
                </div>
                <Activity :size="18" />
              </div>
              <div class="routing-table-head">
                <span>服务 / 模型</span>
                <span>优先级</span>
                <span>权重</span>
              </div>
              <div class="routing-group">
                <div
                  v-for="item in aliasCredentialGroup.items"
                  :key="item.id"
                  class="pool-row routing-table-row"
                  :class="{
                    disabled: !item.enabled,
                    active: isRouteActive(item, aliasActiveRoute),
                  }"
                >
                  <div class="pool-name stacked">
                    <span class="provider-dot" :class="item.provider"></span>
                    <span
                      >{{ item.name
                      }}<small>{{ item.service_url }}</small></span
                    >
                  </div>
                  <input
                    v-model.number="draftPriorities[item.id]"
                    class="weight-input"
                    type="number"
                    step="1"
                    :disabled="!item.enabled"
                    aria-label="优先级"
                    @input="draftDirty = true"
                  />
                  <input
                    v-model.number="draftWeights[item.id]"
                    class="weight-input"
                    type="number"
                    min="0"
                    max="1000000"
                    step="1"
                    :disabled="!item.enabled"
                    aria-label="权重"
                    @input="draftDirty = true"
                  />
                </div>
                <div
                  v-if="!aliasCredentialGroup.items.length"
                  class="group-empty compact"
                >
                  该 Alias 暂无 API 供应商 Credential
                </div>
              </div>
              <button
                class="primary apply-all"
                :disabled="loading"
                @click="saveDraftSettings"
              >
                应用
              </button>
            </div>
          </div>

          <div v-else class="alias-col-models">
            <!-- 已绑定供应商与模型 (按供应商分类) -->
            <div class="alias-models panel current-models-card">
              <div class="panel-head">
                <div>
                  <h3>已绑定供应商与模型</h3>
                </div>
                <span class="panel-count-pill">{{
                  boundAliasProviderGroups.length
                }}</span>
              </div>
              <div
                v-if="boundAliasProviderGroups.length"
                class="alias-provider-groups"
              >
                <div
                  v-for="group in boundAliasProviderGroups"
                  :key="group.credential.id"
                  class="provider-binding-card"
                >
                  <div class="provider-binding-head">
                    <div class="provider-info">
                      <span class="provider-dot" :class="group.provider"></span>
                      <strong :title="group.service_url">{{
                        group.service_url
                      }}</strong>
                      <span class="provider-chip">{{ group.provider }}</span>
                    </div>
                    <div class="provider-meta">
                      <span>优先级 {{ group.priority }}</span>
                      <span>权重 {{ group.weight }}</span>
                    </div>
                  </div>
                  <div class="provider-models-list">
                    <div
                      v-for="model in group.models"
                      :key="model.name"
                      class="bound-model-row"
                    >
                      <div class="model-badge-info">
                        <span class="model-name" :title="model.name">{{
                          model.name
                        }}</span>
                        <small class="bound-alias-tag">{{
                          selectedAliasName
                        }}</small>
                      </div>
                      <button
                        class="remove-btn"
                        @click="removeAlias(group.credential, model.name)"
                      >
                        <Trash2 :size="12" />
                        <span>移除</span>
                      </button>
                    </div>
                  </div>
                </div>
              </div>
              <div v-else class="group-empty compact">
                当前别名暂无绑定的供应商与模型
              </div>
            </div>

            <!-- 可加入该别名 (仅列出尚未绑定该别名的供应商及其模型) -->
            <div class="alias-models panel available-models-card">
              <div class="panel-head">
                <div>
                  <h3>设置别名： {{ selectedAliasName }}</h3>
                </div>
                <span class="panel-count-pill">{{
                  availableAliasProviderGroups.length
                }}</span>
              </div>
              <div class="alias-search-wrap">
                <Search :size="13" class="search-ico" />
                <input
                  v-model="aliasSearch"
                  class="alias-search-input"
                  placeholder="检索未绑定供应商 / 服务地址 / 模型"
                  spellcheck="false"
                />
              </div>
              <div
                v-if="availableAliasProviderGroups.length"
                class="alias-provider-groups available-scroll"
              >
                <div
                  v-for="group in availableAliasProviderGroups"
                  :key="group.credential.id"
                  class="provider-binding-card candidate-card"
                >
                  <button
                    class="provider-binding-head candidate-provider-toggle"
                    :aria-expanded="
                      Boolean(expandedCandidateProviders[group.credential.id])
                    "
                    @click="toggleCandidateProvider(group.credential.id)"
                  >
                    <div class="provider-info">
                      <span class="provider-dot" :class="group.provider"></span>
                      <strong :title="group.service_url">{{
                        group.service_url
                      }}</strong>
                      <span class="provider-chip">{{ group.provider }}</span>
                    </div>
                    <span class="candidate-toggle-meta">
                      <span class="model-count-hint"
                        >{{ group.models.length }} 个模型</span
                      >
                      <ChevronDown
                        :size="15"
                        :class="{
                          expanded:
                            expandedCandidateProviders[group.credential.id],
                        }"
                      />
                    </span>
                  </button>
                  <div
                    v-if="expandedCandidateProviders[group.credential.id]"
                    class="provider-models-list"
                  >
                    <div
                      v-for="model in group.models"
                      :key="model.name"
                      class="candidate-model-row"
                    >
                      <span class="model-name" :title="model.name">{{
                        model.name
                      }}</span>
                      <button
                        class="add-model-btn"
                        @click="assignAlias(group.credential, model.name)"
                      >
                        <Plus :size="12" />
                        <span>设置别名</span>
                      </button>
                    </div>
                  </div>
                </div>
              </div>
              <div v-else class="group-empty compact">
                {{
                  aliasSearch
                    ? "没有匹配的供应商或模型"
                    : "所有可用供应商均已绑定该别名"
                }}
              </div>
            </div>
          </div>
        </div>
      </section>

      <section v-else-if="page === 'auth'" class="content auth-page">
        <div class="alias-action-bar">
          <button class="back-pill-btn" @click="page = 'dashboard'">
            <ArrowLeft :size="14" />
            <span>返回控制台</span>
          </button>
        </div>
        <div class="alias-quick panel">
          <div class="panel-head">
            <div>
              <h3>授权账号快捷策略</h3>
            </div>
            <button class="mini-action" @click="openStrategyEditor">
              <Plus :size="14" />新增策略
            </button>
          </div>
          <div class="strategy-card-grid">
            <button
              v-for="strategy in selectedAliasStrategies"
              :key="strategy.id"
              class="dashboard-preset strategy-preset"
              :class="{
                selected: selectedStrategyId === strategy.id,
                balanced: strategy.name.includes('均衡'),
              }"
              @click="selectStrategy(strategy)"
              @dblclick="openStrategyEditor(selectedAlias, strategy)"
            >
              <span>{{ strategy.name }}</span
              ><small>双击编辑</small>
              <span
                class="strategy-delete"
                title="删除策略"
                @click.stop="deleteStrategy(strategy)"
                >×</span
              >
            </button>
          </div>
        </div>
        <div class="panel routing-panel">
          <div class="panel-head">
            <div>
              <h3>逐项设置</h3>
            </div>
            <Activity :size="19" />
          </div>
          <div class="routing-table-head">
            <span>认证账号</span><span>优先级</span><span>权重</span>
          </div>
          <div class="routing-group">
            <div
              v-for="item in authFileCredentials"
              :key="item.id"
              class="pool-row routing-table-row"
              :class="{
                disabled: !item.enabled,
                active: isRouteActive(item, authActiveRoute),
              }"
            >
              <div class="pool-name stacked">
                <span class="provider-dot" :class="item.provider"></span
                ><span
                  >{{ item.account_label
                  }}<small
                    >{{ item.provider }} · {{ item.auth_name }}</small
                  ></span
                >
              </div>
              <input
                v-model.number="draftPriorities[item.id]"
                class="weight-input"
                type="number"
                step="1"
                :disabled="!item.enabled"
                aria-label="优先级"
                @input="draftDirty = true"
              /><input
                v-model.number="draftWeights[item.id]"
                class="weight-input"
                type="number"
                min="0"
                max="1000000"
                step="1"
                :disabled="!item.enabled"
                aria-label="权重"
                @input="draftDirty = true"
              />
            </div>
            <div v-if="!authFileCredentials.length" class="group-empty compact">
              暂无授权文件账号
            </div>
          </div>
          <button
            class="primary apply-all"
            :disabled="loading"
            @click="saveDraftSettings"
          >
            应用
          </button>
        </div>
      </section>
      <CpaWebPanel v-else-if="page === 'web'" :cpa-web-url="cpaWebUrl" />
    </main>
  </div>
</template>
