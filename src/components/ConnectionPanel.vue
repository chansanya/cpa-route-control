<script setup>
import { KeyRound } from "@lucide/vue";

defineProps({
  managementUrl: { type: String, required: true },
  managementKey: { type: String, required: true },
  rememberKey: { type: Boolean, required: true },
  desktop: { type: Boolean, required: true },
  loading: { type: Boolean, required: true },
});

const emit = defineEmits([
  "update:managementUrl",
  "update:managementKey",
  "update:rememberKey",
  "connect",
]);
</script>

<template>
  <section class="content connection-page">
    <div class="connection-layout">
      <div class="panel settings-panel direct-settings">
        <div class="form-heading">
          <span class="form-icon"><KeyRound :size="18" /></span>
          <div>
            <h3>通行证</h3>
          </div>
        </div>
        <label
          >CPA URL
          <input
            :value="managementUrl"
            spellcheck="false"
            @input="emit('update:managementUrl', $event.target.value)"
          />
        </label>
        <label
          >管理密钥
          <input
            :value="managementKey"
            type="password"
            placeholder="不要填写 $2a$ 开头的 bcrypt 哈希"
            @keyup.enter="emit('connect')"
            @input="emit('update:managementKey', $event.target.value)"
          />
        </label>
        <label class="remember-row">
          <input
            :checked="rememberKey"
            type="checkbox"
            @change="emit('update:rememberKey', $event.target.checked)"
          />
          <span>记住密钥</span>
          <small>{{ desktop ? "保存到凭证" : "仅当前浏览器会话" }}</small>
        </label>
        <button
          class="connect-button"
          :disabled="loading || !managementKey"
          @click="emit('connect')"
        >
          <strong>{{ loading ? "连接中…" : "连接 CPA" }}</strong>
        </button>
      </div>
    </div>
  </section>
</template>
