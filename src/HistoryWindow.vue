<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import {
  NButton,
  NCard,
  NConfigProvider,
  NDivider,
  NGradientText,
  NLayout,
  NLayoutContent,
  NScrollbar,
  NSpace,
  NTag
} from "naive-ui";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  fetchTranslationHistory,
  listenTranslationHistoryUpdates,
  type TranslationHistoryEntry
} from "./history";

const entries = ref<TranslationHistoryEntry[]>([]);
const loading = ref(false);
const appWindow = getCurrentWindow();
const unlistenFns: UnlistenFn[] = [];
const scrollbarRef = ref<InstanceType<typeof NScrollbar> | null>(null);

async function refreshHistory() {
  loading.value = true;
  entries.value = await fetchTranslationHistory();
  loading.value = false;
  await scrollToBottom();
}

async function startDragging(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  if (target?.closest(".no-drag")) return;
  try {
    await appWindow.startDragging();
  } catch (error) {
    console.error("Drag failed", error);
  }
}

async function scrollToBottom() {
  await nextTick();
  const target = scrollbarRef.value;
  if (target?.scrollTo) {
    target.scrollTo({ top: Number.MAX_SAFE_INTEGER, behavior: "auto" });
  }
}

onMounted(async () => {
  await refreshHistory();
  const unlisten = await listenTranslationHistoryUpdates(() => refreshHistory());
  if (unlisten) {
    unlistenFns.push(unlisten);
  }
});

onBeforeUnmount(() => {
  unlistenFns.forEach((fn) => fn());
});
</script>

<template>
  <n-config-provider>
    <div class="title-bar" data-tauri-drag-region @mousedown="startDragging">
      <div class="title-bar__left drag-region" data-tauri-drag-region>
        <n-gradient-text class="app-title" gradient="linear-gradient(110deg, #0ea5e9, #6366f1)">
          翻译历史
        </n-gradient-text>
        <n-tag size="small" bordered type="info">会话内记忆</n-tag>
      </div>
      <div class="title-bar__actions no-drag">
        <n-button size="tiny" tertiary @click="refreshHistory" :loading="loading">刷新</n-button>
      </div>
    </div>

    <n-layout content-style="padding: 12px 18px 28px 18px;">
      <n-layout-content>
        <n-space vertical size="large">
          <n-card class="card" size="large" :bordered="false">
            <div class="section-title">最近翻译（新 → 旧，自下而上）</div>
            <n-divider style="margin: 10px 0 14px 0;" />
            <n-scrollbar ref="scrollbarRef" style="max-height: 70vh;">
              <div v-if="entries.length" class="history-list">
                <div v-for="(entry, idx) in entries" :key="idx" class="history-item">
                  <div class="history-meta">
                    <span class="pill">#{{ idx + 1 }}</span>
                    <span class="hint">原文 / 译文</span>
                  </div>
                  <div class="history-block">
                    <n-tag size="small" type="default" round>原文</n-tag>
                    <div class="history-text">{{ entry.original }}</div>
                  </div>
                  <div class="history-block">
                    <n-tag size="small" type="success" round>译文</n-tag>
                    <div class="history-text translation">{{ entry.translation }}</div>
                  </div>
                </div>
              </div>
              <div v-else class="empty-state">
                <div class="empty-title">暂无翻译记录</div>
                <div class="empty-subtitle">开始翻译后，这里会留下最近 1000 条记录</div>
              </div>
            </n-scrollbar>
          </n-card>
        </n-space>
      </n-layout-content>
    </n-layout>
  </n-config-provider>
</template>

<style scoped>
.history-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.history-item {
  padding: 14px;
  border-radius: 12px;
  background: linear-gradient(145deg, rgba(12, 27, 46, 0.04), rgba(12, 27, 46, 0.02));
  border: 1px solid rgba(12, 27, 46, 0.06);
  box-shadow: 0 4px 12px rgba(17, 24, 39, 0.06);
}

.history-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
  font-size: 12px;
  color: #6c7a89;
}

.pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 2px 8px;
  border-radius: 999px;
  background: #eef2ff;
  color: #4f46e5;
  font-weight: 600;
  font-size: 12px;
}

.hint {
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.history-block {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  padding: 8px 10px;
  border-radius: 10px;
  background: #f7f9fc;
  border: 1px solid rgba(12, 27, 46, 0.05);
  margin-bottom: 8px;
}

.history-text {
  flex: 1;
  white-space: pre-wrap;
  line-height: 1.5;
  color: #16212b;
}

.translation {
  color: #0f172a;
  font-weight: 600;
}

.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: #6c7a89;
}

.empty-title {
  font-weight: 600;
  font-size: 16px;
  margin-bottom: 6px;
}

.empty-subtitle {
  font-size: 13px;
}
</style>
