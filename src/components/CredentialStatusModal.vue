<script setup>
import { X } from "@lucide/vue";

defineProps({
  providerStatusCredentials: { type: Array, default: () => [] },
  authStatusCredentials: { type: Array, default: () => [] },
  credentialStatusTab: { type: String, required: true },
  loading: { type: Boolean, required: true },
  statusName: { type: Function, required: true },
});

const emit = defineEmits(["close", "update:tab", "toggle"]);
</script>

<template>
  <div class="modal-backdrop" @click.self="emit('close')">
    <section class="credential-status-modal" role="dialog" aria-modal="true">
      <button class="modal-close" aria-label="关闭" @click="emit('close')">
        <X :size="16" />
      </button>
      <h3>启用状态</h3>
      <div class="credential-status-tabs" role="tablist">
        <button
          role="tab"
          :aria-selected="credentialStatusTab === 'providers'"
          :class="{ active: credentialStatusTab === 'providers' }"
          @click="emit('update:tab', 'providers')"
        >
          供应商
        </button>
        <button
          role="tab"
          :aria-selected="credentialStatusTab === 'auth'"
          :class="{ active: credentialStatusTab === 'auth' }"
          @click="emit('update:tab', 'auth')"
        >
          授权文件
        </button>
      </div>
      <div class="credential-status-groups">
        <section v-if="credentialStatusTab === 'providers'">
          <h4>API 供应商</h4>
          <div class="credential-status-list">
            <div
              v-for="credential in providerStatusCredentials"
              :key="credential.id"
              class="credential-status-row"
            >
              <span class="provider-dot" :class="credential.provider"></span>
              <div>
                <strong :title="statusName(credential)">{{
                  statusName(credential)
                }}</strong>
                <small>{{ credential.provider }}</small>
              </div>
              <button
                :class="['status-toggle', { enabled: credential.enabled }]"
                :disabled="loading"
                @click="emit('toggle', credential)"
              >
                {{ credential.enabled ? "禁用" : "启用" }}
              </button>
            </div>
          </div>
        </section>
        <section v-if="credentialStatusTab === 'auth'">
          <h4>授权文件</h4>
          <div class="credential-status-list">
            <div
              v-for="credential in authStatusCredentials"
              :key="credential.id"
              class="credential-status-row"
            >
              <span class="provider-dot" :class="credential.provider"></span>
              <div>
                <strong :title="statusName(credential)">{{
                  statusName(credential)
                }}</strong>
                <small>{{ credential.provider }}</small>
              </div>
              <button
                :class="['status-toggle', { enabled: credential.enabled }]"
                :disabled="
                  loading || (credential.unavailable && !credential.disabled)
                "
                @click="emit('toggle', credential)"
              >
                {{
                  credential.unavailable && !credential.disabled
                    ? "不可用"
                    : credential.enabled
                      ? "禁用"
                      : "启用"
                }}
              </button>
            </div>
          </div>
        </section>
      </div>
    </section>
  </div>
</template>
