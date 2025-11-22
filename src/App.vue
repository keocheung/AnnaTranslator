<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  NButton,
  NCard,
  NConfigProvider,
  NGradientText,
  NInput,
  NLayout,
  NLayoutContent,
  NSpace,
  NSwitch,
  NTag,
  createDiscreteApi
} from "naive-ui";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import OpenAI from "openai";
import { useSettingsState } from "./settings";

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
  (("__TAURI_METADATA__" in window) || ("__TAURI_IPC__" in window) || ("__TAURI_INTERNALS__" in window));

const textStyle = computed(() => ({
  fontFamily: settings.value.fontFamily,
  fontSize: `${settings.value.fontSize}px`,
  lineHeight: 1.5
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

onMounted(async () => {
  try {
    const unlisten = await listen<{ text: string } | string>("incoming_text", async (event) => {
      const payload = typeof event.payload === "string" ? event.payload : event.payload?.text ?? "";
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
  if (!settings.value.apiKey) {
    message.error("请先填写 OpenAI API Key");
    return;
  }

  // Defer client creation to avoid SSR/Sandbox surprises.
  const client = new OpenAI({
    apiKey: settings.value.apiKey,
    baseURL: settings.value.baseUrl.replace(/\/$/, "") || undefined,
    dangerouslyAllowBrowser: true
  });

  controller.value?.abort();
  const abortController = new AbortController();
  controller.value = abortController;
  translatedText.value = "";
  streaming.value = true;

  try {
    const completion = await client.chat.completions.create(
      {
        model: settings.value.model,
        stream: true,
        messages: [
          { role: "system", content: settings.value.prompt },
          { role: "user", content }
        ]
      },
      { signal: abortController.signal }
    );

    for await (const part of completion) {
      const delta = part.choices?.[0]?.delta?.content;
      if (delta) translatedText.value += delta;
    }
  } catch (error: unknown) {
    if ((error as Error)?.name === "AbortError") return;
    message.error((error as Error)?.message ?? "翻译失败");
  } finally {
    streaming.value = false;
    controller.value = null;
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
    title: "Local Translator - 设置",
    width: 560,
    height: 720,
    alwaysOnTop: true,
    resizable: true,
    decorations: true,
    visible: true
  });

  settingsWindow.once("tauri://error", (e) => {
    console.error("Failed to open settings window", e);
    message.error("设置窗口打开失败");
  });
}

</script>

<template>
  <n-config-provider>
    <div class="title-bar" data-tauri-drag-region @mousedown="startDragging">
      <div class="title-bar__left drag-region" data-tauri-drag-region>
        <n-gradient-text class="app-title" gradient="linear-gradient(120deg, #4c83ff, #4fd1c5)">
          Local Translator
        </n-gradient-text>
        <n-tag size="small" type="success" bordered>监听端口 {{ settings.serverPort }}</n-tag>
      </div>
      <div class="title-bar__actions no-drag">
        <n-button size="tiny" quaternary @click="openSettingsWindow">设置</n-button>
        <span class="section-title">置顶</span>
        <n-switch size="small" :value="settings.keepOnTop" @update:value="handleKeepOnTop" />
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
                style="padding: 12px 14px; border-radius: 12px; background: #f5f7fb; min-height: 80px; white-space: pre-wrap;"
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
                style="padding: 16px; border-radius: 12px; background: #0b1727; color: #e8f0ff; min-height: 120px; white-space: pre-wrap;"
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
