<script setup>
import { CircleAlert, X } from "@lucide/vue";

defineProps({
  confirmDialog: { type: Object, default: null },
});

const emit = defineEmits(["close", "confirm"]);
</script>

<template>
  <div
    v-if="confirmDialog"
    class="modal-backdrop confirm-backdrop"
    @click.self="emit('close')"
  >
    <section
      class="confirm-modal"
      :class="{ danger: confirmDialog.danger }"
      role="dialog"
      aria-modal="true"
    >
      <button class="modal-close" aria-label="关闭" @click="emit('close')">
        <X :size="16" />
      </button>
      <div class="modal-icon"><CircleAlert :size="21" /></div>
      <h3>{{ confirmDialog.title }}</h3>
      <p>{{ confirmDialog.message }}</p>
      <div class="modal-actions">
        <button class="secondary" @click="emit('close')">取消</button>
        <button
          :class="['modal-confirm', { danger: confirmDialog.danger }]"
          @click="emit('confirm')"
        >
          {{ confirmDialog.confirmLabel }}
        </button>
      </div>
    </section>
  </div>
</template>
