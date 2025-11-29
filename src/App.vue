<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  NButton,
  NButtonGroup,
  NCard,
  NConfigProvider,
  NGradientText,
  NIcon,
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
  PauseRound,
  PlayArrowRound,
  RefreshRound,
  SettingsRound,
  StopRound,
  VisibilityOffRound,
  VisibilityRound,
} from "@vicons/material";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { PhysicalPosition, PhysicalSize, getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useSettingsState } from "./settings";
import { recordTranslationHistory } from "./history";
import { getOpenAIConstructor } from "./openaiClient";
import { purpleThemeOverrides } from "./theme";

const settings = useSettingsState();
const { t } = useI18n();
const originalText = ref("");
const translatedText = ref("");
const furiganaHtml = ref("");
const streaming = ref(false);
const manualInput = ref("");
const portError = ref<string | null>(null);
const { message } = createDiscreteApi(["message"], {
  configProviderProps: {
    themeOverrides: purpleThemeOverrides,
  },
});
const unlistenFns: UnlistenFn[] = [];
const controller = ref<AbortController | null>(null);
const appWindow = getCurrentWindow();
const titleBarRef = ref<HTMLElement | null>(null);
const titleBarVisible = ref(true);
const isTauri =
  typeof window !== "undefined" &&
  ("__TAURI_METADATA__" in window || "__TAURI_IPC__" in window || "__TAURI_INTERNALS__" in window);

const textStyle = computed(() => ({
  fontFamily: settings.value.fontFamily,
  fontSize: `${settings.value.fontSize}px`,
  lineHeight: 1.5,
}));
const showFurigana = computed(
  () => settings.value.showJapaneseFurigana && !!furiganaHtml.value
);
const originalPanelContent = computed(() =>
  showFurigana.value ? furiganaHtml.value : originalText.value || t("app.status.waitingInput")
);
const portTagType = computed(() => (portError.value ? "error" : "info"));
const portTagText = computed(() =>
  portError.value
    ? t("titleBar.portListenFailed", { port: settings.value.serverPort })
    : t("titleBar.listeningPort", { port: settings.value.serverPort })
);

let isPaused = ref(false);
let lastKnownTitleBarHeight = 0;
let handlingTitleBar = false;

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

watch(
  () => settings.value.alwaysShowTitleBar,
  (alwaysShow) => {
    if (alwaysShow) {
      void showTitleBar();
    } else if (!document.hasFocus()) {
      void hideTitleBar();
    }
  },
  { immediate: true }
);

watch(
  () => settings.value.showJapaneseFurigana,
  (enabled) => {
    if (enabled) {
      void refreshFurigana(originalText.value);
    } else {
      furiganaHtml.value = "";
    }
  },
  { immediate: true }
);

watch(
  () => originalText.value,
  (text) => {
    if (settings.value.showJapaneseFurigana) {
      void refreshFurigana(text);
    } else {
      furiganaHtml.value = "";
    }
  }
);

type HttpServerErrorPayload = {
  port?: number;
  message?: string;
};

onMounted(async () => {
  window.addEventListener("blur", handleWindowBlur);
  window.addEventListener("focus", handleWindowFocus);
  nextTick(rememberTitleBarHeight);

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
    message.error(t("app.messages.listenerInitFailed"));
    console.error(error);
  }

  if (isTauri) {
    try {
      const unlistenServerError = await listen<HttpServerErrorPayload>(
        "http_server_failed",
        (event) => {
          const payload = event.payload;
          const port = typeof payload?.port === "number" ? payload.port : settings.value.serverPort;
          if (typeof payload?.port === "number") {
            settings.value.serverPort = payload.port;
          }
          const warning = payload?.message ?? t("app.messages.portListenFailed", { port });
          portError.value = warning;
          message.error(warning);
        }
      );
      unlistenFns.push(unlistenServerError);
    } catch (error) {
      console.error("Failed to listen for HTTP server errors", error);
    }

    await refreshPortError();
  }
});

onBeforeUnmount(() => {
  controller.value?.abort();
  unlistenFns.forEach((fn) => fn());
  window.removeEventListener("blur", handleWindowBlur);
  window.removeEventListener("focus", handleWindowFocus);
});

async function translate(text: string, options: { force?: boolean } = {}) {
  const { force = false } = options;
  if (isPaused.value) {
    return;
  }
  const content = text.trim();
  if (!content) {
    message.warning(t("app.messages.noContent"));
    return;
  }

  controller.value?.abort();
  controller.value = null;
  streaming.value = false;

  if (isTauri && !force) {
    try {
      const cached = await invoke<string | null>("get_cached_translation", { text: content });
      if (cached) {
        translatedText.value = cached;
        void persistTranslationHistory(content, translatedText.value);
        // message.success("命中本地缓存");
        return;
      }
    } catch (error) {
      console.error("Failed to read cached translation:", error);
    }
  }

  if (!settings.value.apiKey) {
    message.error(t("app.messages.missingApiKey"));
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
    message.error((error as Error)?.message ?? t("app.messages.translationFailed"));
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
    message.error(t("app.messages.alwaysOnTopFailed"));
    console.error(error);
  }
}

function handleKeepOnTop(alwaysOnTop: boolean) {
  settings.value.keepOnTop = alwaysOnTop;
}

function handlePause(play: boolean) {
  isPaused.value = !play;
}

function stopStream() {
  controller.value?.abort();
}

function handleManualTranslate() {
  originalText.value = manualInput.value;
  translate(manualInput.value);
}

async function handleRetranslate() {
  const content = originalText.value.trim();
  if (!content) {
    message.warning(t("app.messages.noContentForRetranslate"));
    return;
  }
  await translate(content, { force: true });
}

function fillManualFromOriginal() {
  manualInput.value = originalText.value;
}

async function copyTranslation() {
  try {
    await navigator.clipboard.writeText(translatedText.value);
    message.success(t("app.messages.translationCopied"));
  } catch (error) {
    message.error(t("app.messages.copyFailed"));
  }
}

async function copyOriginal() {
  try {
    await navigator.clipboard.writeText(originalText.value);
    message.success(t("app.messages.originalCopied"));
  } catch (error) {
    message.error(t("app.messages.copyFailed"));
  }
}

async function refreshFurigana(text: string) {
  if (!isTauri || !settings.value.showJapaneseFurigana) {
    furiganaHtml.value = "";
    return;
  }

  const content = text.trim();
  if (!content) {
    furiganaHtml.value = "";
    return;
  }

  try {
    const annotated = await invoke<string>("annotate_furigana", { text: content });
    furiganaHtml.value = annotated;
  } catch (error) {
    furiganaHtml.value = "";
    console.error("Failed to generate furigana:", error);
    message.error(t("app.messages.furiganaFailed"));
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

function rememberTitleBarHeight() {
  const height = titleBarRef.value?.offsetHeight ?? 0;
  if (height > 0) {
    lastKnownTitleBarHeight = height;
  }
}

async function getTitleBarPhysicalHeight() {
  if (!isTauri) return 0;
  rememberTitleBarHeight();
  if (!lastKnownTitleBarHeight) return 0;
  const factor = await appWindow.scaleFactor();
  return Math.round(lastKnownTitleBarHeight * factor);
}

function shouldAutoHideTitleBar() {
  return isTauri && !settings.value.alwaysShowTitleBar;
}

async function hideTitleBar() {
  if (!shouldAutoHideTitleBar() || handlingTitleBar || !titleBarVisible.value) return;
  const barHeight = await getTitleBarPhysicalHeight();
  if (!barHeight) return;
  handlingTitleBar = true;
  try {
    const [size, position] = await Promise.all([appWindow.outerSize(), appWindow.outerPosition()]);
    await Promise.all([
      appWindow.setSize(
        new PhysicalSize(size.width, Math.max(0, size.height - barHeight))
      ),
      appWindow.setPosition(new PhysicalPosition(position.x, position.y + barHeight)),
    ]);
    titleBarVisible.value = false;
  } catch (error) {
    console.error("Failed to hide title bar", error);
  } finally {
    handlingTitleBar = false;
  }
}

async function showTitleBar() {
  if (!isTauri) {
    titleBarVisible.value = true;
    return;
  }
  if (handlingTitleBar) return;
  const wasHidden = !titleBarVisible.value;
  titleBarVisible.value = true;
  if (!wasHidden) return;

  const barHeight = await getTitleBarPhysicalHeight();
  if (!barHeight) return;
  handlingTitleBar = true;
  try {
    const [size, position] = await Promise.all([appWindow.outerSize(), appWindow.outerPosition()]);
    await Promise.all([
      appWindow.setSize(new PhysicalSize(size.width, size.height + barHeight)),
      appWindow.setPosition(new PhysicalPosition(position.x, position.y - barHeight)),
    ]);
  } catch (error) {
    console.error("Failed to show title bar", error);
  } finally {
    handlingTitleBar = false;
  }
}

function handleWindowBlur() {
  if (shouldAutoHideTitleBar()) {
    void hideTitleBar();
  }
}

function handleWindowFocus() {
  void showTitleBar();
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

async function openSettingsWindow(event: MouseEvent) {
  if (!isTauri) {
    message.info(t("app.messages.settingsOnlyInApp"));
    return;
  }

  const existing = await WebviewWindow.getByLabel("settings");
  if (existing) {
    await existing.setFocus();
    return;
  }

  const settingsWindow = new WebviewWindow("settings", {
    url: "index.html#settings",
    title: `${t("common.appName")} - ${t("titleBar.settings")}`,
    width: 800,
    height: 600,
    alwaysOnTop: settings.value.keepOnTop,
    resizable: true,
    decorations: true,
    visible: true,
  });

  settingsWindow.once("tauri://error", (e) => {
    console.error("Failed to open settings window", e);
    message.error(t("app.messages.openSettingsFailed"));
  });

  event.target?.dispatchEvent(
    new MouseEvent("mouseleave", {
      bubbles: true,
      cancelable: true,
      view: window,
    })
  );
}

async function openHistoryWindow(event: MouseEvent) {
  if (!isTauri) {
    message.info(t("app.messages.historyOnlyInApp"));
    return;
  }

  const existing = await WebviewWindow.getByLabel("history");
  if (existing) {
    await existing.setFocus();
    return;
  }

  const historyWindow = new WebviewWindow("history", {
    url: "index.html#history",
    title: `${t("common.appName")} - ${t("titleBar.history")}`,
    width: 640,
    height: 760,
    alwaysOnTop: settings.value.keepOnTop,
    resizable: true,
    decorations: true,
    visible: true,
  });

  historyWindow.once("tauri://error", (e) => {
    console.error("Failed to open history window", e);
    message.error(t("app.messages.openHistoryFailed"));
  });

  console.log(event.target);
  event.target?.dispatchEvent(
    new MouseEvent("mouseleave", {
      bubbles: true,
      cancelable: true,
      view: window,
    })
  );
}

async function persistTranslationHistory(original: string, translation: string) {
  if (!translation.trim()) return;
  await recordTranslationHistory(original, translation);
}

async function refreshPortError() {
  if (!isTauri) return;
  try {
    const payload = await invoke<HttpServerErrorPayload | null>("get_http_server_error");
    if (!payload) return;
    const port = typeof payload.port === "number" ? payload.port : settings.value.serverPort;
    settings.value.serverPort = port;
    const warning = payload.message ?? t("app.messages.portListenFailed", { port });
    portError.value = warning;
  } catch (error) {
    console.error("Failed to refresh HTTP server error", error);
  }
}
</script>

<template>
  <n-config-provider :theme-overrides="purpleThemeOverrides">
    <div class="app-shell">
      <div
        ref="titleBarRef"
        class="title-bar"
        data-tauri-drag-region
        @mousedown="startDragging"
        v-show="titleBarVisible"
      >
        <div class="title-bar__left drag-region" data-tauri-drag-region>
          <n-gradient-text class="app-title" gradient="linear-gradient(120deg, #4c83ff, #4fd1c5)">
            {{ t("common.appName") }}
          </n-gradient-text>
          <n-tooltip v-if="portError" trigger="hover">
            <template #trigger>
              <n-tag size="small" :type="portTagType" bordered>
                {{ portTagText }}
              </n-tag>
            </template>
            {{ portError }}
          </n-tooltip>
          <n-tag v-else size="small" :type="portTagType" bordered>
            {{ portTagText }}
          </n-tag>
        </div>
        <div class="title-bar__actions no-drag">
          <n-tooltip trigger="hover">
            <template #trigger>
              <n-switch size="large" :value="!isPaused" @update:value="handlePause">
                <template #checked-icon>
                  <n-icon :component="PlayArrowRound" />
                </template>
                <template #unchecked-icon>
                  <n-icon :component="PauseRound" />
                </template>
              </n-switch>
            </template>
            {{ t("titleBar.translatePause") }}
          </n-tooltip>
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
            {{ t("titleBar.alwaysOnTop") }}
          </n-tooltip>
          <n-tooltip trigger="hover">
            <template #trigger>
              <n-switch
                size="large"
                :value="settings.alwaysShowTitleBar"
                @update:value="settings.alwaysShowTitleBar = $event"
              >
                <template #checked-icon>
                  <n-icon :component="VisibilityRound" />
                </template>
                <template #unchecked-icon>
                  <n-icon :component="VisibilityOffRound" />
                </template>
              </n-switch>
            </template>
            {{ t("titleBar.alwaysShowTitleBar") }}
          </n-tooltip>
          <n-button-group>
            <n-tooltip trigger="hover">
              <template #trigger>
                <n-button
                  circle
                  tertiary
                  :focusable="false"
                  @click="stopStream"
                  :disabled="!streaming"
                  class="main-button"
                >
                  <n-icon>
                    <StopRound />
                  </n-icon>
                </n-button>
              </template>
              {{ t("app.actions.stopStream") }}
            </n-tooltip>
            <n-tooltip trigger="hover">
              <template #trigger>
                <n-button
                  circle
                  tertiary
                  :focusable="false"
                  @click="handleRetranslate"
                  :disabled="!originalText || streaming"
                  class="main-button"
                >
                  <n-icon>
                    <RefreshRound />
                  </n-icon>
                </n-button>
              </template>
              {{ t("app.actions.retranslate") }}
            </n-tooltip>
            <n-tooltip trigger="hover">
              <template #trigger>
                <n-button
                  circle
                  tertiary
                  :focusable="false"
                  @click="openHistoryWindow($event)"
                  class="main-button"
                >
                  <n-icon>
                    <HistoryRound />
                  </n-icon>
                </n-button>
              </template>
              {{ t("titleBar.history") }}
            </n-tooltip>
            <n-tooltip trigger="hover">
              <template #trigger>
                <n-button
                  circle
                  tertiary
                  :focusable="false"
                  @click="openSettingsWindow($event)"
                  class="main-button"
                >
                  <n-icon>
                    <SettingsRound />
                  </n-icon>
                </n-button>
              </template>
              {{ t("titleBar.settings") }}
            </n-tooltip>
            <n-tooltip trigger="hover">
              <template #trigger>
                <n-button
                  circle
                  tertiary
                  type="error"
                  @click="appWindow.close()"
                  class="main-button"
                >
                  <n-icon>
                    <CloseRound />
                  </n-icon>
                </n-button>
              </template>
              {{ t("titleBar.quit") }}
            </n-tooltip>
          </n-button-group>
        </div>
      </div>
      <div class="text-panels">
        <div class="panel-content translated-panel" :style="textStyle">
          {{
            translatedText ||
            (streaming ? t("app.status.translating") : t("app.status.noTranslation"))
          }}
        </div>

        <div class="panel-content original-panel" :style="textStyle">
          <div v-if="showFurigana" class="furigana-content" v-html="originalPanelContent"></div>
          <template v-else>
            {{ originalPanelContent }}
          </template>
        </div>
        <!-- <n-input
            v-model:value="manualInput"
            type="textarea"
            :placeholder="t('app.placeholders.manualInput')"
            :autosize="{ minRows: 3, maxRows: 6 }"
          />
          <n-flex>
            <n-button type="primary" @click="handleManualTranslate">
              {{ t("app.actions.translateInput") }}
            </n-button>
            <n-button secondary @click="stopStream" :disabled="!streaming">
              {{ t("app.actions.stopStream") }}
            </n-button>
          </n-flex> -->
      </div>
    </div>
  </n-config-provider>
</template>
<style>
body {
  text-autospace: normal;
}

html,
body {
  overscroll-behavior: none;
}

.app-title {
  font-family:
    Google Sans Flex,
    sans-serif;
  font-weight: 800;
  font-size: 20px;
}

.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  min-height: 0;
}

.text-panels {
  flex: 1;
  display: grid;
  grid-template-rows: 1fr 1fr;
  gap: 0;
  padding: 0;
  min-height: 0;
}

.text-panel {
  min-height: 0;
  display: flex;
  border-radius: 0;
}

.text-panel .n-card__content {
  display: flex;
  flex: 1;
  padding: 0;
}

.panel-content {
  flex: 1;
  white-space: pre-wrap;
  overflow: auto;
  padding: 12px;
  border-radius: 0;
}

.translated-panel {
  background: #0b1727;
  color: #e8f0ff;
}

.original-panel {
  background: #f5f7fb;
  color: #16212b;
}

.furigana-content ruby {
  ruby-position: over;
}

.furigana-content rt {
  font-size: 0.75em;
  color: #42526b;
}

.main-button {
  font-size: 20px;
}

.n-card > .n-card__content:first-child,
.n-card > .n-card__content {
  padding: 0;
}

.n-float-button {
  font-size: 20px;
}

/* Use default arrow cursor for interactive Naive UI components */
.n-button,
.n-button *,
.n-float-button,
.n-float-button *,
.n-switch,
.n-switch *,
.n-tag,
.n-tag *,
.n-menu,
.n-menu * {
  cursor: default !important;
}
</style>
