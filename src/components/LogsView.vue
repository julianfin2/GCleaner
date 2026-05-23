<script setup lang="ts">
defineProps<{
  logs: string[];
}>();
</script>

<template>
  <section class="tab-pane logs-pane">
    <header class="pane-header">
      <h1>操作日志</h1>
    </header>
    <div class="log-console logs-console">
      <div v-for="(log, i) in logs" :key="i" class="log-line">
        {{ log }}
      </div>
      <div v-if="!logs.length" class="empty-state">暂无日志记录</div>
    </div>
  </section>
</template>

<style scoped>
.logs-pane {
  min-height: calc(100vh - 80px);
  display: flex;
  flex-direction: column;
}

.logs-console {
  flex: 1;
  height: auto;
  min-height: 0;
}
</style>
