<script setup lang="ts">
import { CheckCircle2, Loader2, Play, RefreshCw, ShieldAlert } from "lucide-vue-next";

export interface OverviewRow {
  key: string;
  title: string;
  summary: string;
  status: string;
  pending: number;
  scanning: boolean;
  cleaning: boolean;
  scanned: boolean;
}

defineProps<{
  rows: OverviewRow[];
  authorizedAccounts: string[];
  isBusy: boolean;
  canCleanAll: boolean;
}>();

defineEmits<{
  scanAll: [];
  cleanAll: [];
  scanItem: [key: string];
  cleanItem: [key: string];
}>();
</script>

<template>
  <section class="tab-pane">
    <header class="pane-header">
      <div class="header-actions">
        <div>
          <h1>项目总览</h1>
        </div>
        <div class="btns">
          <button class="btn btn-outline" @click="$emit('scanAll')" :disabled="isBusy || !authorizedAccounts.length">
            <RefreshCw :size="18" />
            一键扫描全部
          </button>
          <button class="btn btn-danger" @click="$emit('cleanAll')" :disabled="isBusy || !canCleanAll">
            <Play :size="18" />
            一键清理全部
          </button>
        </div>
      </div>
    </header>

    <div class="overview-list">
      <article v-for="row in rows" :key="row.key" class="overview-row">
        <div class="overview-main">
          <div class="overview-title">
            <Loader2 v-if="row.scanning || row.cleaning" class="spin" :size="18" />
            <CheckCircle2 v-else-if="row.scanned && row.pending === 0" :size="18" class="text-green" />
            <ShieldAlert v-else :size="18" class="text-sec" />
            <span>{{ row.title }}</span>
          </div>
          <p>{{ row.summary }}</p>
        </div>

        <div class="overview-status">{{ row.status }}</div>

        <div class="overview-actions">
          <button
            class="btn btn-outline"
            @click="$emit('scanItem', row.key)"
            :disabled="isBusy || !authorizedAccounts.length"
          >
            扫描
          </button>
          <button
            class="btn btn-danger"
            @click="$emit('cleanItem', row.key)"
            :disabled="isBusy || row.pending === 0"
          >
            清理
          </button>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.overview-list {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.overview-row {
  display: grid;
  grid-template-columns: minmax(260px, 1fr) 150px auto;
  align-items: center;
  gap: 18px;
  background: #ffffff;
  border-radius: var(--radius-card);
  box-shadow: var(--card-shadow);
  padding: 20px 22px;
}

.overview-main {
  min-width: 0;
}

.overview-title {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--text-main);
  font-size: 16px;
  font-weight: 700;
}

.overview-main p {
  margin-top: 6px;
  color: var(--text-sec);
  font-size: 13px;
}

.overview-status {
  color: var(--text-sec);
  font-size: 13px;
  font-weight: 700;
  white-space: nowrap;
}

.overview-actions {
  display: flex;
  gap: 10px;
}

.overview-actions .btn {
  min-width: 72px;
  padding: 9px 14px;
}

@media (max-width: 980px) {
  .overview-row {
    grid-template-columns: 1fr;
  }

  .overview-actions {
    justify-content: flex-start;
  }
}
</style>
