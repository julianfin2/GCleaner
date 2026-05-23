<script setup lang="ts">
import { CheckCircle2, FileText, Loader2, RefreshCw, Share2, ShieldAlert, Trash2 } from "lucide-vue-next";
import type { SharedDriveFile } from "../types";

defineProps<{
  authorizedAccounts: string[];
  sharedResults: SharedDriveFile[];
  isScanningShared: boolean;
  isRemovingShared: boolean;
  hasScannedShared: boolean;
  hasPendingSharedFiles: boolean;
  sharedStatusText: (file: SharedDriveFile) => string;
}>();

defineEmits<{
  startSharedScan: [];
  removeSharedPermissions: [];
}>();
</script>

<template>
  <section class="tab-pane">
    <header class="pane-header">
      <div class="header-actions">
        <div>
          <h1>移除共享</h1>
        </div>
        <div class="btns">
          <button
            class="btn btn-outline"
            @click="$emit('startSharedScan')"
            :disabled="isScanningShared || isRemovingShared || !authorizedAccounts.length"
          >
            <Loader2 v-if="isScanningShared" class="spin" :size="18" />
            <RefreshCw v-else :size="18" />
            开始扫描
          </button>
          <button
            class="btn btn-danger"
            @click="$emit('removeSharedPermissions')"
            :disabled="isRemovingShared || isScanningShared || !hasPendingSharedFiles"
          >
            <Trash2 :size="18" />
            移除权限
          </button>
        </div>
      </div>
    </header>

    <div class="results-container mt-6">
      <div v-if="!sharedResults.length && !isScanningShared && !hasScannedShared" class="empty-card">
        <Share2 :size="48" class="text-sec" />
        <p>点击上方“开始扫描”以列出“与我共享”中可移除当前账号权限的文件</p>
      </div>

      <div v-if="!sharedResults.length && !isScanningShared && hasScannedShared" class="empty-card success">
        <CheckCircle2 :size="48" class="text-green" />
        <p>扫描完成，未发现可移除当前账号权限的共享文件</p>
      </div>

      <div v-if="isScanningShared" class="loading-state">
        <Loader2 class="spin" :size="32" />
        <p>正在扫描“与我共享”，请稍候...</p>
      </div>

      <div v-if="sharedResults.length" class="results-table-wrapper">
        <table class="results-table">
          <thead>
            <tr>
              <th>文件名</th>
              <th>账号</th>
              <th>所有者</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="file in sharedResults" :key="`${file.account}-${file.id}`">
              <td>
                <div class="file-name">
                  <FileText :size="16" class="text-sec" />
                  {{ file.name }}
                </div>
              </td>
              <td>{{ file.account }}</td>
              <td>{{ file.owner }}</td>
              <td>
                <span class="row-status" :class="file.cleanStatus || 'idle'">
                  <Loader2 v-if="file.cleanStatus === 'cleaning'" class="spin" :size="16" />
                  <CheckCircle2 v-else-if="file.cleanStatus === 'done'" :size="16" />
                  <ShieldAlert v-else-if="file.cleanStatus === 'failed'" :size="16" />
                  <span>{{ sharedStatusText(file) }}</span>
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </section>
</template>
