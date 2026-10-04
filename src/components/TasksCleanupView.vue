<script setup lang="ts">
import { CheckCircle2, ListChecks, Loader2, RefreshCw, ShieldAlert, Trash2 } from "@lucide/vue";
import type { TaskCleanupItem } from "../types";

defineProps<{
  authorizedAccounts: string[];
  taskResults: TaskCleanupItem[];
  isScanningTasks: boolean;
  isCleaningTasks: boolean;
  hasScannedTasks: boolean;
  hasPendingTasks: boolean;
  taskStatusText: (item: TaskCleanupItem) => string;
}>();

defineEmits<{
  scanTasks: [];
  cleanupTasks: [];
}>();
</script>

<template>
  <section class="tab-pane">
    <header class="pane-header">
      <div class="header-actions">
        <div>
          <h1>清理任务</h1>
        </div>
        <div class="btns">
          <button
            class="btn btn-outline"
            @click="$emit('scanTasks')"
            :disabled="isScanningTasks || isCleaningTasks || !authorizedAccounts.length"
          >
            <Loader2 v-if="isScanningTasks" class="spin" :size="18" />
            <RefreshCw v-else :size="18" />
            开始扫描
          </button>
          <button
            class="btn btn-danger"
            @click="$emit('cleanupTasks')"
            :disabled="isCleaningTasks || isScanningTasks || !hasPendingTasks"
          >
            <Trash2 :size="18" />
            清理任务
          </button>
        </div>
      </div>
    </header>

    <div class="results-container mt-6">
      <div v-if="!taskResults.length && !isScanningTasks && !hasScannedTasks" class="empty-card">
        <ListChecks :size="48" class="text-sec" />
        <p>点击上方“开始扫描”以列出任务列表和默认任务列表中的任务</p>
      </div>

      <div v-if="!taskResults.length && !isScanningTasks && hasScannedTasks" class="empty-card success">
        <CheckCircle2 :size="48" class="text-green" />
        <p>扫描完成，未发现可清理的任务列表或任务</p>
      </div>

      <div v-if="isScanningTasks" class="loading-state">
        <Loader2 class="spin" :size="32" />
        <p>正在扫描任务列表，请稍候...</p>
      </div>

      <div v-if="taskResults.length" class="results-table-wrapper">
        <table class="results-table">
          <thead>
            <tr>
              <th>任务列表</th>
              <th>账号</th>
              <th>处理方式</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="item in taskResults" :key="`${item.account}-${item.id}`">
              <td>
                <div class="task-title">
                  <ListChecks :size="16" class="text-sec" />
                  {{ item.title }}
                </div>
              </td>
              <td>{{ item.account }}</td>
              <td>
                <span class="task-action">
                  {{ item.is_default ? `清空 ${item.task_count} 个任务` : "删除任务列表" }}
                </span>
              </td>
              <td>
                <span class="row-status" :class="item.cleanStatus || 'idle'">
                  <Loader2 v-if="item.cleanStatus === 'cleaning'" class="spin" :size="16" />
                  <CheckCircle2 v-else-if="item.cleanStatus === 'done'" :size="16" />
                  <ShieldAlert v-else-if="item.cleanStatus === 'failed'" :size="16" />
                  <span>{{ taskStatusText(item) }}</span>
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </section>
</template>

<style scoped>
.task-title {
  display: flex;
  align-items: center;
  gap: 10px;
  max-width: 420px;
  overflow: hidden;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.task-action {
  color: var(--text-sec);
  font-size: 13px;
  font-weight: 700;
}
</style>
