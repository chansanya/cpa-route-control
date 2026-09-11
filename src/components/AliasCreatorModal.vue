<script setup>
import { ArrowUpRight, Plus, X } from "@lucide/vue";

defineProps({
  newAliasName: { type: String, required: true },
});

const emit = defineEmits(["close", "update:newAliasName", "create"]);
</script>

<template>
  <div class="modal-backdrop" @click.self="emit('close')">
    <section class="alias-creator-modal" role="dialog" aria-modal="true">
      <button class="modal-close" aria-label="关闭" @click="emit('close')">
        <X :size="16" />
      </button>
      <div class="modal-icon"><Plus :size="20" /></div>
      <h3>新增模型别名</h3>
      <label class="alias-creator-field">
        模型别名
        <input
          :value="newAliasName"
          autofocus
          maxlength="48"
          placeholder="例如：fast-code"
          @input="emit('update:newAliasName', $event.target.value)"
          @keyup.enter="emit('create')"
        />
      </label>
      <div class="modal-actions">
        <button class="secondary" @click="emit('close')">取消</button>
        <button class="modal-confirm" @click="emit('create')">
          创建并编辑<ArrowUpRight :size="14" />
        </button>
      </div>
    </section>
  </div>
</template>
