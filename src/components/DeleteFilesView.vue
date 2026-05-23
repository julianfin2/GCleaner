<script setup lang="ts">
import { CheckCircle2, FileText, Loader2, RefreshCw, ShieldAlert, Trash2 } from "lucide-vue-next";
import type { DeleteMode, DriveFile } from "../types";

defineProps<{
  authorizedAccounts: string[];
  scanResults: DriveFile[];
  isScanning: boolean;
  isCleaning: boolean;
  hasScanned: boolean;
  hasPendingDeleteFiles: boolean;
  deleteMode: DeleteMode;
  modeLabel: (mode: DeleteMode) => string;
  rowStatusText: (file: DriveFile) => string;
}>();

defineEmits<{
  "update:deleteMode": [value: DeleteMode];
  startScan: [];
  deleteFiles: [];
}>();
</script>

<template>
  <section class="tab-pane">
    <header class="pane-header">
      <div class="header-actions">
        <div>
          <h1>删除文件</h1>
        </div>
        <div class="btns">
          <div class="delete-mode-control" aria-label="删除方式">
            <div class="segmented-control">
              <button
                class="segment-btn"
                :class="{ active: deleteMode === 'trash' }"
                @click="$emit('update:deleteMode', 'trash')"
                :disabled="isScanning || isCleaning"
              >
                移入回收站
              </button>
              <button
                class="segment-btn danger"
                :class="{ active: deleteMode === 'permanent' }"
                @click="$emit('update:deleteMode', 'permanent')"
                :disabled="isScanning || isCleaning"
              >
                永久删除
              </button>
            </div>
          </div>
          <button class="btn btn-outline" @click="$emit('startScan')" :disabled="isScanning || isCleaning || !authorizedAccounts.length">
            <Loader2 v-if="isScanning" class="spin" :size="18" />
            <RefreshCw v-else :size="18" />
            开始扫描
          </button>
          <button class="btn btn-danger" @click="$emit('deleteFiles')" :disabled="isCleaning || isScanning || !hasPendingDeleteFiles">
            <Trash2 :size="18" />
            {{ modeLabel(deleteMode) }}
          </button>
        </div>
      </div>
    </header>

    <div class="results-container mt-6">
      <div v-if="!scanResults.length && !isScanning && !hasScanned" class="empty-card">
        <ShieldAlert :size="48" class="text-sec" />
        <p>点击上方“开始扫描”以列出我的云端硬盘中的自有文件</p>
      </div>

      <div v-if="!scanResults.length && !isScanning && hasScanned" class="empty-card success">
        <CheckCircle2 :size="48" class="text-green" />
        <p>扫描完成，未发现可删除的自有文件或目录</p>
      </div>

      <div v-if="isScanning" class="loading-state">
        <Loader2 class="spin" :size="32" />
        <p>正在努力扫描中，请稍候...</p>
      </div>

      <div v-if="scanResults.length" class="results-table-wrapper">
        <table class="results-table">
          <thead>
            <tr>
              <th>文件名</th>
              <th>账号</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="file in scanResults" :key="file.id">
              <td>
                <div class="file-name">
                  <FileText :size="16" class="text-sec" />
                  {{ file.name }}
                </div>
              </td>
              <td>{{ file.account }}</td>
              <td>
                <span class="row-status" :class="file.cleanStatus || 'idle'">
                  <Loader2 v-if="file.cleanStatus === 'cleaning'" class="spin" :size="16" />
                  <CheckCircle2 v-else-if="file.cleanStatus === 'done'" :size="16" />
                  <ShieldAlert v-else-if="file.cleanStatus === 'failed'" :size="16" />
                  <span>{{ rowStatusText(file) }}</span>
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </section>
</template>
