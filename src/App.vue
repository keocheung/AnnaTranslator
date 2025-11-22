<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  NButton,
  NCard,
  NConfigProvider,
  NDivider,
  NForm,
  NFormItem,
  NGradientText,
  NInput,
  NInputNumber,
  NLayout,
  NLayoutContent,
  NSpace,
  NSwitch,
  NTag,
  createDiscreteApi
} from "naive-ui";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import OpenAI from "openai";

type Settings = {
  baseUrl: string;
  apiKey: string;
  model: string;
  prompt: string;
  fontFamily: string;
  fontSize: number;
  serverPort: number;
  keepOnTop: boolean;
};

const defaultSettings: Settings = {
  baseUrl: "https://api.openai.com",
  apiKey: "",
  model: "gpt-4o-mini",
  prompt: "你是一个 Galgame 文本翻译助手，请将原文翻译为简洁、流畅的中文对白，保留原有格式与人名。",
  fontFamily: "\"LXGW WenKai\", \"Noto Sans SC\", \"Space Grotesk\", sans-serif",
  fontSize: 18,
  serverPort: 17889,
  keepOnTop: true
};

const settings = ref<Settings>(loadSettings());
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
  settings,
  (val) => {
    localStorage.setItem("translator-settings", JSON.stringify(val));
  },
  { deep: true }
);

onMounted(async () => {
  await toggleOnTop(settings.value.keepOnTop);
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

function loadSettings(): Settings {
  const cached = localStorage.getItem("translator-settings");
  if (!cached) return { ...defaultSettings };
  try {
    const parsed = JSON.parse(cached) as Partial<Settings>;
    return { ...defaultSettings, ...parsed };
  } catch (error) {
    console.error("Failed to parse settings:", error);
    return { ...defaultSettings };
  }
}

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

async function toggleOnTop(alwaysOnTop: boolean) {
  try {
    await appWindow.setAlwaysOnTop(alwaysOnTop);
    settings.value.keepOnTop = alwaysOnTop;
  } catch (error) {
    message.error("无法设置置顶窗口");
    console.error(error);
  }
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
        <span class="section-title">置顶</span>
        <n-switch size="small" :value="settings.keepOnTop" @update:value="toggleOnTop" />
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
              <div class="section-title">译文 (流式)</div>
              <div
                style="padding: 16px; border-radius: 12px; background: #0b1727; color: #e8f0ff; min-height: 120px; white-space: pre-wrap;"
                :style="textStyle"
              >
                {{ translatedText || (streaming ? "正在流式翻译..." : "尚未有译文") }}
              </div>
              <n-space>
                <n-button type="primary" ghost @click="copyTranslation" :disabled="!translatedText">
                  复制译文
                </n-button>
                <n-tag v-if="streaming" type="info" round bordered>流式输出中</n-tag>
              </n-space>
            </n-space>
          </n-card>

          <n-card class="card" title="设置" size="large" :bordered="false">
            <n-form label-placement="top" size="medium">
              <n-space :wrap="true" :size="[24, 12]">
                <n-form-item label="OpenAI Base URL">
                  <n-input v-model:value="settings.baseUrl" placeholder="https://api.openai.com" />
                </n-form-item>
                <n-form-item label="OpenAI API Key">
                  <n-input
                    v-model:value="settings.apiKey"
                    placeholder="sk-..."
                    type="password"
                    show-password-on="click"
                  />
                </n-form-item>
                <n-form-item label="模型">
                  <n-input v-model:value="settings.model" placeholder="gpt-4o-mini" />
                </n-form-item>
              </n-space>

              <n-form-item label="翻译提示词">
                <n-input
                  v-model:value="settings.prompt"
                  type="textarea"
                  :autosize="{ minRows: 2, maxRows: 6 }"
                />
              </n-form-item>

              <n-space :wrap="true" :size="[24, 12]">
                <n-form-item label="字体">
                  <n-input v-model:value="settings.fontFamily" placeholder="自定义字体栈" />
                </n-form-item>
                <n-form-item label="字号">
                  <n-input-number v-model:value="settings.fontSize" :min="12" :max="32" />
                </n-form-item>
              </n-space>

              <n-divider />

              <div class="section-title">本地 HTTP 推送示例</div>
              <div class="mono" style="background: #0c1b2e; color: #b3d4ff; padding: 12px; border-radius: 8px;">
                curl -X POST http://127.0.0.1:{{ settings.serverPort }}/submit \\\n
                -H "Content-Type: application/json" \\\n
                -d '{"text":"<待翻译文本>"}'
              </div>
            </n-form>
          </n-card>
        </n-space>
      </n-layout-content>
    </n-layout>
  </n-config-provider>
</template>
