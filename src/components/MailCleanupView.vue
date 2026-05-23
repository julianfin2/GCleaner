<script setup lang="ts">
import { CheckCircle2, Link2, Loader2, Mail, RefreshCw, ShieldAlert, Trash2 } from "lucide-vue-next";
import type { DeleteMode, MailCleanupCandidate } from "../types";

defineProps<{
  authorizedAccounts: string[];
  mailQuery: string;
  mailDeleteMode: DeleteMode;
  mailResults: MailCleanupCandidate[];
  isScanningMail: boolean;
  isCleaningMail: boolean;
  hasScannedMail: boolean;
  hasPendingMailCandidates: boolean;
  modeLabel: (mode: DeleteMode) => string;
  mailStatusText: (candidate: MailCleanupCandidate) => string;
}>();

defineEmits<{
  "update:mailQuery": [value: string];
  "update:mailDeleteMode": [value: DeleteMode];
  scanMail: [];
  cleanupMail: [];
}>();
</script>

<template>
  <section class="tab-pane">
    <header class="pane-header">
      <div class="header-actions">
        <div>
          <h1>清理邮件</h1>
        </div>
        <div class="btns">
          <div class="delete-mode-control" aria-label="邮件删除方式">
            <div class="segmented-control">
              <button
                class="segment-btn"
                :class="{ active: mailDeleteMode === 'trash' }"
                @click="$emit('update:mailDeleteMode', 'trash')"
                :disabled="isScanningMail || isCleaningMail"
              >
                移入回收站
              </button>
              <button
                class="segment-btn danger"
                :class="{ active: mailDeleteMode === 'permanent' }"
                @click="$emit('update:mailDeleteMode', 'permanent')"
                :disabled="isScanningMail || isCleaningMail"
              >
                永久删除
              </button>
            </div>
          </div>
          <button
            class="btn btn-outline"
            @click="$emit('scanMail')"
            :disabled="isScanningMail || isCleaningMail || !authorizedAccounts.length"
          >
            <Loader2 v-if="isScanningMail" class="spin" :size="18" />
            <RefreshCw v-else :size="18" />
            开始扫描
          </button>
          <button
            class="btn btn-danger"
            @click="$emit('cleanupMail')"
            :disabled="isCleaningMail || isScanningMail || !hasPendingMailCandidates"
          >
            <Trash2 :size="18" />
            处理并{{ modeLabel(mailDeleteMode) }}
          </button>
        </div>
      </div>
    </header>

    <div class="mail-query-card">
      <label for="mail-query">Gmail 过滤条件</label>
      <input
        id="mail-query"
        :value="mailQuery"
        type="text"
        class="mail-query-input"
        :disabled="isScanningMail || isCleaningMail"
        @input="$emit('update:mailQuery', ($event.target as HTMLInputElement).value)"
      />
    </div>

    <div class="results-container mt-6">
      <div v-if="!mailResults.length && !isScanningMail && !hasScannedMail" class="empty-card">
        <Mail :size="48" class="text-sec" />
        <p>点击上方“开始扫描”以根据邮件线索反查 Drive 文件权限</p>
      </div>

      <div v-if="!mailResults.length && !isScanningMail && hasScannedMail" class="empty-card success">
        <CheckCircle2 :size="48" class="text-green" />
        <p>扫描完成，未发现带 Drive 文件链接的邮件</p>
      </div>

      <div v-if="isScanningMail" class="loading-state">
        <Loader2 class="spin" :size="32" />
        <p>正在扫描 Gmail 并检查 Drive 权限，请稍候...</p>
      </div>

      <div v-if="mailResults.length" class="results-table-wrapper mail-table-wrapper">
        <table class="results-table mail-table">
          <colgroup>
            <col class="mail-col" />
            <col class="account-col" />
            <col class="files-col" />
            <col class="status-col" />
          </colgroup>
          <thead>
            <tr>
              <th>邮件</th>
              <th>账号</th>
              <th>Drive 文件</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="candidate in mailResults" :key="`${candidate.account}-${candidate.id}`">
              <td>
                <div class="mail-cell">
                  <div class="mail-title">{{ candidate.subject || "(无主题)" }}</div>
                  <div class="mail-meta">{{ candidate.from || "未知发件人" }}</div>
                </div>
              </td>
              <td>
                <div class="account-cell">{{ candidate.account }}</div>
              </td>
              <td>
                <div class="mail-file-summary">
                  <Link2 :size="15" />
                  <span :class="{ 'summary-good': candidate.removable_permissions > 0 }">
                    {{ candidate.removable_permissions }} 可移除
                  </span>
                </div>
              </td>
              <td>
                <span class="row-status" :class="candidate.cleanStatus || 'idle'">
                  <Loader2 v-if="candidate.cleanStatus === 'cleaning'" class="spin" :size="16" />
                  <CheckCircle2 v-else-if="candidate.cleanStatus === 'done'" :size="16" />
                  <ShieldAlert v-else-if="candidate.cleanStatus === 'failed'" :size="16" />
                  <span>{{ mailStatusText(candidate) }}</span>
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
.mail-query-card {
  display: flex;
  align-items: center;
  gap: 14px;
  background: #ffffff;
  border-radius: var(--radius-card);
  box-shadow: var(--card-shadow);
  padding: 18px 20px;
}

.mail-query-card label {
  color: var(--text-sec);
  font-size: 13px;
  font-weight: 700;
  white-space: nowrap;
}

.mail-query-input {
  flex: 1;
  height: 40px;
  padding: 0 12px;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  color: var(--text-main);
  font-size: 14px;
  outline: none;
}

.mail-query-input:focus {
  border-color: var(--primary-color);
  box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.1);
}

.mail-table-wrapper {
  max-width: 100%;
}

.mail-table {
  table-layout: fixed;
}

.mail-col {
  width: auto;
}

.account-col {
  width: 220px;
}

.files-col {
  width: 230px;
}

.status-col {
  width: 110px;
}

.mail-table th,
.mail-table td {
  min-width: 72px;
}

.mail-cell,
.account-cell {
  min-width: 0;
  max-width: 100%;
}

.mail-table th:first-child,
.mail-table td:first-child {
  min-width: 180px;
}

.mail-title {
  max-width: 100%;
  overflow: hidden;
  color: var(--text-main);
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mail-meta {
  max-width: 100%;
  overflow: hidden;
  margin-top: 4px;
  color: var(--text-sec);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mail-file-summary {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-sec);
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
}

.account-cell {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 980px) {
  .account-col {
    width: 170px;
  }

  .files-col {
    width: 210px;
  }
}

.summary-good {
  color: #34c759;
}

</style>
