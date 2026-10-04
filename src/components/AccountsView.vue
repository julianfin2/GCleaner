<script setup lang="ts">
import { CheckCircle2, Clock3, Loader2, Trash2 } from "@lucide/vue";
import type { AuthorizedAccount } from "../types";

defineProps<{
  accessTokensText: string;
  authorizedAccounts: AuthorizedAccount[];
  isImportingTokens: boolean;
  isClearingAccounts: boolean;
  deletingAccount: string | null;
  importMessage: { type: "success" | "error"; text: string } | null;
  tokenTimeLeftText: (account: AuthorizedAccount) => string;
}>();

defineEmits<{
  "update:accessTokensText": [value: string];
  importTokens: [];
  clearAccounts: [];
  deleteAccount: [account: string];
}>();
</script>

<template>
  <section class="tab-pane">
    <header class="pane-header">
      <h1>账号授权</h1>
    </header>

    <div class="grid auth-grid">
      <div class="card glass">
        <h3>导入短期 access token</h3>
        <p>每行粘贴一个 Google access token。token 通常 1 小时有效，过期后需要重新导入。</p>
        <textarea
          class="token-input mt-4"
          autocomplete="off"
          autocapitalize="off"
          :spellcheck="false"
          :value="accessTokensText"
          :disabled="isImportingTokens"
          placeholder="ya29..."
          @input="$emit('update:accessTokensText', ($event.target as HTMLTextAreaElement).value)"
        ></textarea>
        <button class="btn btn-primary token-submit" @click="$emit('importTokens')" :disabled="isImportingTokens || !accessTokensText.trim()">
          <Loader2 v-if="isImportingTokens" class="spin" :size="18" />
          导入 token
        </button>
      </div>

      <div class="card glass">
        <div class="card-title-row">
          <h3>已导入账号 ({{ authorizedAccounts.length }})</h3>
          <button
            class="icon-btn danger"
            @click="$emit('clearAccounts')"
            :disabled="isClearingAccounts || !authorizedAccounts.length"
            title="清空全部 token"
          >
            <Trash2 :size="16" />
          </button>
        </div>
        <div class="account-list auth-account-list mt-4">
          <div v-for="acc in authorizedAccounts" :key="acc.email" class="account-item">
            <div class="account-name">
              <CheckCircle2 :size="16" class="text-green" />
              <span>{{ acc.email }}</span>
            </div>
            <div class="account-meta">
              <Clock3 :size="14" />
              <span>{{ tokenTimeLeftText(acc) }}</span>
            </div>
            <button
              class="icon-btn danger"
              @click="$emit('deleteAccount', acc.email)"
              :disabled="deletingAccount === acc.email"
              title="删除该账号 token"
            >
              <Loader2 v-if="deletingAccount === acc.email" class="spin" :size="16" />
              <Trash2 v-else :size="16" />
            </button>
          </div>
          <div v-if="!authorizedAccounts.length" class="empty-state">
            暂无已导入账号
          </div>
        </div>
        <div class="import-feedback">
          <div v-if="importMessage" class="import-message" :class="importMessage.type">
            <CheckCircle2 v-if="importMessage.type === 'success'" :size="16" />
            <span v-else class="error-dot"></span>
            <span>{{ importMessage.text }}</span>
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.auth-grid {
  min-height: calc(100vh - 160px);
  align-items: stretch;
}

.auth-grid .card {
  min-height: 0;
}

.auth-grid .card:first-child {
  display: flex;
  flex-direction: column;
}

.token-input {
  width: 100%;
  flex: 1;
  min-height: 260px;
  resize: vertical;
  padding: 12px;
  border: 1px solid var(--border-color);
  border-radius: 12px;
  color: var(--text-main);
  font: 13px/1.5 "JetBrains Mono", Consolas, monospace;
  outline: none;
}

.token-submit {
  margin-top: 16px;
}

.import-feedback {
  margin-top: 16px;
}

.import-message {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 600;
  line-height: 1.45;
}

.import-message.success {
  color: #1f8f45;
  background: rgba(52, 199, 89, 0.1);
}

.import-message.error {
  color: var(--danger-color);
  background: rgba(255, 59, 48, 0.08);
}

.error-dot {
  flex: 0 0 auto;
  width: 8px;
  height: 8px;
  margin-top: 5px;
  border-radius: 50%;
  background: var(--danger-color);
}

.token-input:focus {
  border-color: var(--primary-color);
  box-shadow: 0 0 0 3px rgba(0, 122, 255, 0.1);
}

.account-meta {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--text-sec);
  font-size: 12px;
  font-weight: 600;
  margin-left: auto;
  white-space: nowrap;
}

.auth-account-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}
</style>
