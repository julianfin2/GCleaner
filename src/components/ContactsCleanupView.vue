<script setup lang="ts">
import { CheckCircle2, Loader2, RefreshCw, ShieldAlert, Tags, Trash2, Users } from "@lucide/vue";
import type { ContactGroupItem, ContactItem } from "../types";

defineProps<{
  authorizedAccounts: string[];
  contactResults: ContactItem[];
  contactGroupResults: ContactGroupItem[];
  isScanningContacts: boolean;
  isDeletingContacts: boolean;
  hasScannedContacts: boolean;
  hasPendingContacts: boolean;
  contactStatusText: (contact: ContactItem) => string;
  contactGroupStatusText: (group: ContactGroupItem) => string;
}>();

defineEmits<{
  scanContacts: [];
  deleteContacts: [];
}>();
</script>

<template>
  <section class="tab-pane">
    <header class="pane-header">
      <div class="header-actions">
        <div>
          <h1>清理联系人</h1>
        </div>
        <div class="btns">
          <button
            class="btn btn-outline"
            @click="$emit('scanContacts')"
            :disabled="isScanningContacts || isDeletingContacts || !authorizedAccounts.length"
          >
            <Loader2 v-if="isScanningContacts" class="spin" :size="18" />
            <RefreshCw v-else :size="18" />
            开始扫描
          </button>
          <button
            class="btn btn-danger"
            @click="$emit('deleteContacts')"
            :disabled="isDeletingContacts || isScanningContacts || !hasPendingContacts"
          >
            <Trash2 :size="18" />
            清理联系人
          </button>
        </div>
      </div>
    </header>

    <div class="results-container mt-6">
      <div v-if="!contactResults.length && !contactGroupResults.length && !isScanningContacts && !hasScannedContacts" class="empty-card">
        <Users :size="48" class="text-sec" />
        <p>点击上方“开始扫描”以列出通讯录联系人和自定义标签，不包含“其他联系人”</p>
      </div>

      <div v-if="!contactResults.length && !contactGroupResults.length && !isScanningContacts && hasScannedContacts" class="empty-card success">
        <CheckCircle2 :size="48" class="text-green" />
        <p>扫描完成，未发现通讯录联系人或自定义标签</p>
      </div>

      <div v-if="isScanningContacts" class="loading-state">
        <Loader2 class="spin" :size="32" />
        <p>正在扫描通讯录联系人和自定义标签，请稍候...</p>
      </div>

      <div v-if="contactResults.length" class="results-table-wrapper">
        <div class="section-title">
          <Users :size="18" />
          <span>通讯录联系人</span>
          <small>{{ contactResults.length }} 个</small>
        </div>
        <table class="results-table">
          <thead>
            <tr>
              <th>联系人</th>
              <th>账号</th>
              <th>电话</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="contact in contactResults" :key="`${contact.account}-${contact.id}`">
              <td>
                <div class="contact-name">
                  <Users :size="16" class="text-sec" />
                  <div class="contact-text">
                    <span>{{ contact.display_name || "(无姓名)" }}</span>
                    <small>{{ contact.email || "无邮箱" }}</small>
                  </div>
                </div>
              </td>
              <td>{{ contact.account }}</td>
              <td>{{ contact.phone || "无电话" }}</td>
              <td>
                <span class="row-status" :class="contact.cleanStatus || 'idle'">
                  <Loader2 v-if="contact.cleanStatus === 'cleaning'" class="spin" :size="16" />
                  <CheckCircle2 v-else-if="contact.cleanStatus === 'done'" :size="16" />
                  <ShieldAlert v-else-if="contact.cleanStatus === 'failed'" :size="16" />
                  <span>{{ contactStatusText(contact) }}</span>
                </span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div v-if="contactGroupResults.length" class="results-table-wrapper contact-groups-table">
        <div class="section-title">
          <Tags :size="18" />
          <span>自定义标签</span>
          <small>{{ contactGroupResults.length }} 个</small>
        </div>
        <table class="results-table">
          <thead>
            <tr>
              <th>标签</th>
              <th>账号</th>
              <th>成员数</th>
              <th>状态</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="group in contactGroupResults" :key="`${group.account}-${group.id}`">
              <td>
                <div class="contact-name">
                  <Tags :size="16" class="text-sec" />
                  <div class="contact-text">
                    <span>{{ group.name || "(未命名标签)" }}</span>
                    <small>此处仅删除标签，联系人另外删除</small>
                  </div>
                </div>
              </td>
              <td>{{ group.account }}</td>
              <td>{{ group.member_count }}</td>
              <td>
                <span class="row-status" :class="group.cleanStatus || 'idle'">
                  <Loader2 v-if="group.cleanStatus === 'cleaning'" class="spin" :size="16" />
                  <CheckCircle2 v-else-if="group.cleanStatus === 'done'" :size="16" />
                  <ShieldAlert v-else-if="group.cleanStatus === 'failed'" :size="16" />
                  <span>{{ contactGroupStatusText(group) }}</span>
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
.results-container {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.section-title {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 18px 24px;
  color: var(--text-main);
  font-size: 16px;
  font-weight: 700;
  border-bottom: 1px solid #F3F4F6;
}

.section-title small {
  color: var(--text-sec);
  font-size: 13px;
  font-weight: 600;
}

.contact-name {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  font-weight: 600;
}

.contact-text {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.contact-text span,
.contact-text small {
  max-width: 360px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.contact-text small {
  color: var(--text-sec);
  font-size: 12px;
  font-weight: 500;
}

.contact-groups-table {
  margin-top: 2px;
}
</style>
