<script setup>
defineProps({
  cpaWebUrl: { type: String, required: true },
});

defineEmits(["open-external"]);
</script>

<template>
  <section class="content web-page">
    <div class="web-frame-panel panel">
      <iframe
        class="cpa-web-frame"
        :src="cpaWebUrl"
        title="CPA 管理网站"
      ></iframe>
    </div>
  </section>
</template>
