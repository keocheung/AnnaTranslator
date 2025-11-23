<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  NButton,
  NButtonGroup,
  NCard,
  NConfigProvider,
  NGradientText,
  NIcon,
  NInput,
  NLayout,
  NLayoutContent,
  NSpace,
  NSwitch,
  NTag,
  NTooltip,
  createDiscreteApi,
} from "naive-ui";
import {
  CloseRound,
  HistoryRound,
  LayersClearRound,
  LayersRound,
  SettingsRound,
} from "@vicons/material";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useSettingsState } from "./settings";
import { recordTranslationHistory } from "./history";

const settings = useSettingsState();
const originalText = ref("");
const translatedText = ref("");
const streaming = ref(false);
const manualInput = ref("");
const { message } = createDiscreteApi(["message"]);
const unlistenFns: UnlistenFn[] = [];
const controller = ref<AbortController | null>(null);
const appWindow = getCurrentWindow();
const isTauri =
  typeof window !== "undefined" &&
  ("__TAURI_METADATA__" in window || "__TAURI_IPC__" in window || "__TAURI_INTERNALS__" in window);

const textStyle = computed(() => ({
  fontFamily: settings.value.fontFamily,
  fontSize: `${settings.value.fontSize}px`,
  lineHeight: 1.5,
}));

watch(
  () => settings.value.keepOnTop,
  (alwaysOnTop) => applyAlwaysOnTop(alwaysOnTop),
  { immediate: true }
);

watch(
  () => settings.value.monitorClipboard,
  (enabled) => syncClipboardWatch(enabled),
  { immediate: true }
);

watch(
  () => settings.value.openaiCompatibleInput,
  (enabled) => syncOpenAICompatibleInput(enabled),
  { immediate: true }
);

onMounted(async () => {
  try {
    const unlisten = await listen<{ text: string } | string>("incoming_text", async (event) => {
      const payload =
        typeof event.payload === "string" ? event.payload : (event.payload?.text ?? "");
      if (!payload) return;
      originalText.value = payload;
      await translate(payload);
    });
    unlistenFns.push(unlisten);
  } catch (error) {
    message.error("事件监听初始化失败");
    console.error(error);
  }
});

onBeforeUnmount(() => {
  controller.value?.abort();
  unlistenFns.forEach((fn) => fn());
});

async function translate(text: string) {
  const content = text.trim();
  if (!content) {
    message.warning("没有可翻译的文本");
    return;
  }

  controller.value?.abort();
  controller.value = null;
  streaming.value = false;

  if (isTauri) {
    try {
      const cached = await invoke<string | null>("get_cached_translation", { text: content });
      if (cached) {
        translatedText.value = cached;
        void persistTranslationHistory(content, translatedText.value);
        message.success("命中本地缓存");
        return;
      }
    } catch (error) {
      console.error("Failed to read cached translation:", error);
    }
  }

  if (!settings.value.apiKey) {
    message.error("请先填写 OpenAI API Key");
    return;
  }

  const client = new (await getOpenAIConstructor())({
    apiKey: settings.value.apiKey,
    baseURL: settings.value.baseUrl.replace(/\/$/, "") || undefined,
    dangerouslyAllowBrowser: true,
  });

  const abortController = new AbortController();
  controller.value = abortController;
  translatedText.value = "";
  streaming.value = true;
  let shouldPersist = false;

  try {
    const completion = await client.chat.completions.create(
      {
        model: settings.value.model,
        stream: true,
        messages: [
          { role: "system", content: settings.value.prompt },
          { role: "user", content },
        ],
      },
      { signal: abortController.signal }
    );

    for await (const part of completion) {
      const delta = part.choices?.[0]?.delta?.content;
      if (delta) translatedText.value += delta;
    }
    shouldPersist = true;
  } catch (error: unknown) {
    if ((error as Error)?.name === "AbortError") return;
    message.error((error as Error)?.message ?? "翻译失败");
  } finally {
    streaming.value = false;
    controller.value = null;
  }

  if (shouldPersist && translatedText.value && isTauri) {
    try {
      await invoke("store_translation", {
        text: content,
        translation: translatedText.value,
      });
    } catch (error) {
      console.error("Failed to store translation cache:", error);
    }
  }

  if (translatedText.value) {
    void persistTranslationHistory(content, translatedText.value);
  }
}

async function applyAlwaysOnTop(alwaysOnTop: boolean) {
  if (!isTauri) return;
  try {
    await appWindow.setAlwaysOnTop(alwaysOnTop);
  } catch (error) {
    message.error("无法设置置顶窗口");
    console.error(error);
  }
}

function handleKeepOnTop(alwaysOnTop: boolean) {
  settings.value.keepOnTop = alwaysOnTop;
}

function stopStream() {
  controller.value?.abort();
}

function handleManualTranslate() {
  originalText.value = manualInput.value;
  translate(manualInput.value);
}

function fillManualFromOriginal() {
  manualInput.value = originalText.value;
}

async function copyTranslation() {
  try {
    await navigator.clipboard.writeText(translatedText.value);
    message.success("译文已复制");
  } catch (error) {
    message.error("复制失败");
  }
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

async function syncClipboardWatch(enabled: boolean) {
  if (!isTauri) return;
  try {
    await invoke("set_clipboard_watch", { enabled });
  } catch (error) {
    console.error("Failed to sync clipboard watch state", error);
  }
}

async function syncOpenAICompatibleInput(enabled: boolean) {
  if (!isTauri) return;
  try {
    await invoke("set_openai_compatible_input", { enabled });
  } catch (error) {
    console.error("Failed to sync OpenAI compatible input state", error);
  }
}

async function openSettingsWindow() {
  if (!isTauri) {
    message.info("设置窗口仅在 Tauri 应用内可用");
    return;
  }

  const existing = await WebviewWindow.getByLabel("settings");
  if (existing) {
    await existing.setFocus();
    return;
  }

  const settingsWindow = new WebviewWindow("settings", {
    url: "index.html#settings",
    title: "Anna Translator - 设置",
    width: 560,
    height: 720,
    alwaysOnTop: true,
    resizable: true,
    decorations: true,
    visible: true,
  });

  settingsWindow.once("tauri://error", (e) => {
    console.error("Failed to open settings window", e);
    message.error("设置窗口打开失败");
  });
}

async function openHistoryWindow() {
  if (!isTauri) {
    message.info("历史窗口仅在 Tauri 应用内可用");
    return;
  }

  const existing = await WebviewWindow.getByLabel("history");
  if (existing) {
    await existing.setFocus();
    return;
  }

  const historyWindow = new WebviewWindow("history", {
    url: "index.html#history",
    title: "Anna Translator - 历史",
    width: 640,
    height: 760,
    alwaysOnTop: true,
    resizable: true,
    decorations: true,
    visible: true,
  });

  historyWindow.once("tauri://error", (e) => {
    console.error("Failed to open history window", e);
    message.error("历史窗口打开失败");
  });
}

async function persistTranslationHistory(original: string, translation: string) {
  if (!translation.trim()) return;
  await recordTranslationHistory(original, translation);
}

let openAIConstructor: typeof import("openai").default | null = null;

async function getOpenAIConstructor() {
  if (openAIConstructor) return openAIConstructor;
  const mod = await import("openai");
  openAIConstructor = mod.default;
  return openAIConstructor;
}
</script>

<template>
  <n-config-provider>
    <div class="title-bar" data-tauri-drag-region @mousedown="startDragging">
      <div class="title-bar__left drag-region" data-tauri-drag-region>
        <n-gradient-text class="app-title" gradient="linear-gradient(120deg, #4c83ff, #4fd1c5)">
          Anna Translator
        </n-gradient-text>
        <n-tag size="small" type="success" bordered>监听端口 {{ settings.serverPort }}</n-tag>
      </div>
      <div class="title-bar__actions no-drag">
        <n-tooltip trigger="hover">
          <template #trigger>
            <n-switch size="large" :value="settings.keepOnTop" @update:value="handleKeepOnTop">
              <template #checked-icon>
                <n-icon :component="LayersRound" />
              </template>
              <template #unchecked-icon>
                <n-icon :component="LayersClearRound" />
              </template>
            </n-switch>
          </template>
          窗口置顶
        </n-tooltip>
        <n-button-group>
          <n-tooltip trigger="hover">
            <template #trigger>
              <n-button circle class="main-button" @click="openHistoryWindow">
                <n-icon>
                  <HistoryRound />
                </n-icon>
              </n-button>
            </template>
            历史记录 (H)
          </n-tooltip>
          <n-tooltip trigger="hover">
            <template #trigger>
              <n-button circle class="main-button" @click="openSettingsWindow">
                <n-icon>
                  <SettingsRound />
                </n-icon>
              </n-button>
            </template>
            设置
          </n-tooltip>
          <n-tooltip trigger="hover">
            <template #trigger>
              <n-button circle class="main-button" @click="appWindow.close()">
                <n-icon>
                  <CloseRound />
                </n-icon>
              </n-button>
            </template>
            退出
          </n-tooltip>
        </n-button-group>
      </div>
    </div>
    <n-layout content-style="padding: 12px 18px 28px 18px;">
      <n-layout-content>
        <n-space vertical size="large">
          <n-card class="card" size="large" :bordered="false">
            <n-space vertical size="large">
              <n-space align="center" justify="space-between">
                <div class="section-title">原文</div>
                <n-button size="tiny" tertiary @click="fillManualFromOriginal">
                  将原文填回输入
                </n-button>
              </n-space>
              <div
                style="
                  padding: 12px 14px;
                  border-radius: 12px;
                  background: #f5f7fb;
                  min-height: 80px;
                  white-space: pre-wrap;
                "
                :style="textStyle"
              >
                {{ originalText || "等待本地 HTTP 推送 / 手动输入" }}
              </div>
              <n-input
                v-model:value="manualInput"
                type="textarea"
                placeholder="手动输入以测试翻译"
                :autosize="{ minRows: 3, maxRows: 6 }"
              />
              <n-space>
                <n-button type="primary" @click="handleManualTranslate">翻译输入</n-button>
                <n-button secondary @click="stopStream" :disabled="!streaming">停止流</n-button>
              </n-space>
            </n-space>
          </n-card>

          <n-card class="card" size="large" :bordered="false">
            <n-space vertical size="large">
              <div class="section-title">译文</div>
              <div
                style="
                  padding: 16px;
                  border-radius: 12px;
                  background: #0b1727;
                  color: #e8f0ff;
                  min-height: 120px;
                  white-space: pre-wrap;
                "
                :style="textStyle"
              >
                {{ translatedText || (streaming ? "正在翻译..." : "尚未有译文") }}
              </div>
              <n-space>
                <n-button type="primary" ghost @click="copyTranslation" :disabled="!translatedText">
                  复制译文
                </n-button>
                <n-tag v-if="streaming" type="info" round bordered>流式输出中</n-tag>
              </n-space>
            </n-space>
          </n-card>
        </n-space>
      </n-layout-content>
    </n-layout>
  </n-config-provider>
</template>
<style>
.main-button {
  font-size: 20px;
}
</style>
