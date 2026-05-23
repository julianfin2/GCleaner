<script setup lang="ts">
import { AlertTriangle, X } from "lucide-vue-next";

defineProps<{
  open: boolean;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  danger?: boolean;
}>();

defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<template>
  <Teleport to="body">
    <Transition name="dialog-fade">
      <div v-if="open" class="dialog-backdrop" @click.self="$emit('cancel')">
        <Transition name="dialog-pop" appear>
          <section class="dialog-panel" role="dialog" aria-modal="true" :aria-label="title">
            <button class="close-btn" type="button" aria-label="关闭" @click="$emit('cancel')">
              <X :size="18" />
            </button>

            <div class="dialog-icon" :class="{ danger }">
              <AlertTriangle :size="22" />
            </div>

            <div class="dialog-content">
              <h2>{{ title }}</h2>
              <p>{{ message }}</p>
            </div>

            <div class="dialog-actions">
              <button class="dialog-btn secondary" type="button" @click="$emit('cancel')">
                {{ cancelText || "取消" }}
              </button>
              <button
                class="dialog-btn primary"
                :class="{ danger }"
                type="button"
                @click="$emit('confirm')"
              >
                {{ confirmText || "确定" }}
              </button>
            </div>
          </section>
        </Transition>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.dialog-backdrop {
  position: fixed;
  inset: 0;
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(15, 23, 42, 0.32);
  backdrop-filter: blur(8px);
}

.dialog-panel {
  position: relative;
  width: min(500px, 100%);
  background: #ffffff;
  border: 1px solid rgba(229, 229, 231, 0.9);
  border-radius: 20px;
  box-shadow: 0 28px 70px rgba(15, 23, 42, 0.18);
  padding: 28px;
}

.close-btn {
  position: absolute;
  top: 14px;
  right: 14px;
  width: 34px;
  height: 34px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #86868b;
  background: transparent;
  border: none;
  border-radius: 10px;
  cursor: pointer;
  transition: background 0.2s, color 0.2s;
}

.close-btn:hover {
  color: #1d1d1f;
  background: #f3f4f6;
}

.dialog-icon {
  width: 46px;
  height: 46px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: #007aff;
  background: rgba(0, 122, 255, 0.1);
  border-radius: 14px;
  margin-bottom: 18px;
}

.dialog-icon.danger {
  color: #ff3b30;
  background: rgba(255, 59, 48, 0.1);
}

.dialog-content h2 {
  color: #1d1d1f;
  font-size: 20px;
  line-height: 1.25;
  margin: 0 36px 8px 0;
}

.dialog-content p {
  color: #5f6368;
  font-size: 14px;
  line-height: 1.7;
  margin: 0;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 26px;
  flex-wrap: wrap;
}

.dialog-btn {
  min-width: 112px;
  height: 40px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  border: none;
  font-size: 14px;
  font-weight: 700;
  line-height: 1;
  padding: 0 18px;
  white-space: nowrap;
  cursor: pointer;
  transition: transform 0.2s, background 0.2s, border-color 0.2s;
}

.dialog-btn:hover {
  transform: translateY(-1px);
}

.dialog-btn.secondary {
  color: #1d1d1f;
  background: #ffffff;
  border: 1px solid #e5e5e7;
}

.dialog-btn.primary {
  min-width: 150px;
  color: #ffffff;
  background: #007aff;
}

@media (max-width: 420px) {
  .dialog-actions {
    flex-direction: column-reverse;
  }

  .dialog-btn {
    width: 100%;
  }
}

.dialog-btn.primary.danger {
  background: #ff3b30;
}

.dialog-fade-enter-active,
.dialog-fade-leave-active {
  transition: opacity 0.18s ease;
}

.dialog-fade-enter-from,
.dialog-fade-leave-to {
  opacity: 0;
}

.dialog-pop-enter-active,
.dialog-pop-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}

.dialog-pop-enter-from,
.dialog-pop-leave-to {
  opacity: 0;
  transform: translateY(8px) scale(0.98);
}
</style>
