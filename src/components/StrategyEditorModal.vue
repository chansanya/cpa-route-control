<script setup>
import { ArrowUpRight, Save, Trash2, X } from "@lucide/vue";

defineProps({
  editingStrategyId: { type: String, default: null },
  strategyScopeLabel: { type: String, required: true },
  strategyName: { type: String, required: true },
  strategyCredentialGroups: { type: Array, default: () => [] },
  strategyPriorities: { type: Object, required: true },
  strategyWeights: { type: Object, required: true },
});

const emit = defineEmits([
  "close",
  "delete",
  "save",
  "update:strategyName",
  "update:priority",
  "update:weight",
]);
</script>

<template>
  <div class="modal-backdrop" @click.self="emit('close')">
    <section class="strategy-modal" role="dialog" aria-modal="true">
      <button class="modal-close" aria-label="关闭" @click="emit('close')">
        <X :size="16" />
      </button>
      <div class="strategy-modal-head">
        <div class="modal-icon"><Save :size="20" /></div>
        <div>
          <h3>
            {{ editingStrategyId ? "编辑" : "新增" }}
            {{ strategyScopeLabel }} 策略
          </h3>
        </div>
      </div>
      <label class="strategy-name"
        >策略名称
        <input
          :value="strategyName"
          placeholder="例如：工作日下午"
          maxlength="40"
          @input="emit('update:strategyName', $event.target.value)"
        />
      </label>
      <div class="strategy-list">
        <div
          v-for="group in strategyCredentialGroups"
          :key="group.id"
          class="strategy-group"
        >
          <div class="strategy-group-title">
            <span>{{ group.title }}</span>
            <b>{{ group.items.length }}</b>
          </div>
          <div
            v-for="item in group.items"
            :key="item.id"
            class="strategy-weight"
          >
            <div class="strategy-credential">
              <span>{{ item.name }}</span>
              <small>{{ item.provider }}</small>
            </div>
            <div class="strategy-fields">
              <label>
                <small>优先级</small>
                <input
                  :value="strategyPriorities[item.id]"
                  type="number"
                  step="1"
                  @input="emit('update:priority', item.id, $event.target.value)"
                />
              </label>
              <label>
                <small>权重</small>
                <input
                  :value="strategyWeights[item.id]"
                  type="number"
                  min="0"
                  max="1000000"
                  step="1"
                  @input="emit('update:weight', item.id, $event.target.value)"
                />
              </label>
            </div>
          </div>
        </div>
      </div>
      <div class="modal-actions">
        <button
          v-if="editingStrategyId"
          class="secondary danger-btn"
          @click="emit('delete')"
        >
          <Trash2 :size="14" />删除策略
        </button>
        <button class="secondary" @click="emit('close')">取消</button>
        <button class="modal-confirm" @click="emit('save')">
          {{ editingStrategyId ? "保存修改" : "保存策略" }}
          <ArrowUpRight :size="14" />
        </button>
      </div>
    </section>
  </div>
</template>
