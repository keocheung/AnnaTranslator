<script setup lang="ts">
import {
  NAlert,
  NButton,
  NCard,
  NCode,
  NConfigProvider,
  NDivider,
  NForm,
  NFormItem,
  NFlex,
  NIcon,
  NInput,
  NInputNumber,
  NMenu,
  NProgress,
  NSelect,
  NSpace,
  NSwitch,
  darkTheme,
  useOsTheme,
  createDiscreteApi,
} from "naive-ui";
import { Globe24Regular, Sparkle24Filled } from "@vicons/fluent";
import { useSettingsState } from "./settings";
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { getOpenAIConstructor } from "./openaiClient";
import { purpleThemeOverrides } from "./theme";
import { resolveLocale } from "./i18n";
import { getVersion } from "@tauri-apps/api/app";
import { check, type DownloadEvent } from "@tauri-apps/plugin-updater";

const settings = useSettingsState();
const { t } = useI18n();
const validating = ref(false);
const checkingUpdate = ref(false);
const installingUpdate = ref(false);
const updateProgress = ref<number | null>(null);
const downloadedBytes = ref(0);
const totalBytes = ref<number | null>(null);
const { message } = createDiscreteApi(["message"], {
  configProviderProps: {
    themeOverrides: purpleThemeOverrides,
  },
});
const activeMenu = ref("general");
const appVersion = ref("-");
const isTauriEnv = ref(false);
const availableUpdate = ref<Awaited<ReturnType<typeof check>> | null>(null);
const menuOptions = computed(() => [
  { label: t("settings.menu.general"), key: "general" },
  { label: t("settings.menu.translation"), key: "translation" },
  { label: t("settings.menu.appearance"), key: "appearance" },
  { label: t("settings.menu.input"), key: "input" },
  { label: t("settings.menu.preprocess"), key: "preprocess" },
  { label: t("settings.menu.about"), key: "about" },
]);

const systemLocaleLabel = computed(() => {
  const locale = resolveLocale("system");
  const labelMap: Record<string, string> = {
    en: "English",
    "zh-CN": "简体中文",
  };
  const matched = labelMap[locale] ?? locale;
  return t("settings.language.followSystemMatched", { language: matched });
});

const languageOptions = computed(() => [
  { label: systemLocaleLabel.value, value: "system" },
  { label: "English", value: "en" },
  { label: "简体中文", value: "zh-CN" },
]);
const handleMenuUpdate = (key: string) => {
  activeMenu.value = key;
};

const submitCommand = computed(
  () => `curl -X POST http://127.0.0.1:${settings.value.serverPort}/submit \\
              -H 'Content-Type: text/plain' \\
              --data-raw '${t("settings.input.payloadHint")}'`
);

const theme = computed(() => (useOsTheme().value === "dark" ? darkTheme : null));
hljs.registerLanguage("bash", bash);

const detectTauri = () =>
  typeof window !== "undefined" &&
  ("__TAURI_METADATA__" in window || "__TAURI_INTERNALS__" in window || "__TAURI_IPC__" in window);

onMounted(async () => {
  isTauriEnv.value = detectTauri();
  if (!isTauriEnv.value) return;

  try {
    appVersion.value = await getVersion();
  } catch (error) {
    console.error("Failed to read app version:", error);
  }
});

function addReplacementRule() {
  settings.value.replacements = [
    ...settings.value.replacements,
    { pattern: "", replacement: "", flags: "" },
  ];
}

function removeReplacementRule(index: number) {
  const next = [...settings.value.replacements];
  next.splice(index, 1);
  settings.value.replacements = next;
}

async function validateOpenAIConfig() {
  if (!settings.value.apiKey) {
    message.error(t("settings.translation.missingApiKey"));
    return;
  }

  validating.value = true;
  try {
    const OpenAI = await getOpenAIConstructor();
    const client = new OpenAI({
      apiKey: settings.value.apiKey,
      baseURL: settings.value.baseUrl.replace(/\/$/, "") || undefined,
      dangerouslyAllowBrowser: true,
    });

    await client.chat.completions.create({
      model: settings.value.model,
      messages: [
        { role: "system", content: "You are checking connectivity and credentials." },
        { role: "user", content: "ping" },
      ],
      max_tokens: 1,
    });

    message.success(t("settings.translation.validateSuccess"));
  } catch (error) {
    message.error((error as Error)?.message ?? t("settings.translation.validateFailed"));
  } finally {
    validating.value = false;
  }
}

async function checkForUpdates() {
  if (!isTauriEnv.value) {
    message.error(t("settings.about.notInAppMessage"));
    return;
  }

  checkingUpdate.value = true;
  availableUpdate.value = null;
  try {
    const update = await check();
    if (update) {
      availableUpdate.value = update;
      message.info(t("settings.about.updateAvailableToast", { version: update.version }));
    } else {
      message.success(t("settings.about.noUpdate"));
    }
  } catch (error) {
    console.error("Update check failed:", error);
    message.error((error as Error)?.message ?? t("settings.about.updateFailed"));
  } finally {
    checkingUpdate.value = false;
  }
}

async function installUpdate() {
  if (!availableUpdate.value) return;
  installingUpdate.value = true;
  updateProgress.value = null;
  downloadedBytes.value = 0;
  totalBytes.value = null;
  try {
    await availableUpdate.value.downloadAndInstall((event: DownloadEvent) => {
      switch (event.event) {
        case "Started":
          totalBytes.value = event.data.contentLength ?? null;
          downloadedBytes.value = 0;
          updateProgress.value = totalBytes.value ? 0 : null;
          break;
        case "Progress":
          downloadedBytes.value += event.data.chunkLength;
          if (totalBytes.value) {
            updateProgress.value = Math.min(
              100,
              Math.round((downloadedBytes.value / totalBytes.value) * 100)
            );
          }
          break;
        case "Finished":
          if (totalBytes.value) {
            updateProgress.value = 100;
          }
          break;
      }
    });

    console.log("update installed");
    await relaunch();
    message.success(t("settings.about.installStarted"));
  } catch (error) {
    message.error((error as Error)?.message ?? t("settings.about.installFailed"));
  } finally {
    installingUpdate.value = false;
    updateProgress.value = null;
  }
}

const downloadProgressText = computed(() => {
  if (!installingUpdate.value) return "";
  if (updateProgress.value != null && totalBytes.value) {
    return t("settings.about.progressWithTotal", {
      downloaded: formatBytes(downloadedBytes.value),
      total: formatBytes(totalBytes.value),
    });
  }
  if (updateProgress.value != null) {
    return t("settings.about.progressPercent", { percent: updateProgress.value });
  }
  return t("settings.about.downloading");
});

function formatBytes(bytes: number) {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** exponent;
  return `${value.toFixed(exponent === 0 ? 0 : 1)} ${units[exponent]}`;
}
</script>

<template>
  <n-config-provider :theme="theme" :hljs="hljs" :theme-overrides="purpleThemeOverrides">
    <div class="settings-wrapper">
      <n-card class="card settings-card" :bordered="false">
        <div class="settings-layout">
          <div class="settings-menu-pane">
            <n-menu
              class="settings-menu"
              :value="activeMenu"
              :options="menuOptions"
              @update:value="handleMenuUpdate"
            />
          </div>
          <div class="settings-content">
            <n-flex>
              <div v-if="activeMenu === 'general'" class="settings-pane">
                <n-flex align="center" :wrap="false" :size="20">
                  <n-flex align="center" :size="4">
                    <n-icon>
                      <Globe24Regular />
                    </n-icon>
                    <span>{{ t("settings.language.title") }}</span>
                  </n-flex>
                  <n-select
                    v-model:value="settings.language"
                    :options="languageOptions"
                    :consistent-menu-width="false"
                    style="width: 50%"
                  />
                </n-flex>
              </div>
              <div v-else-if="activeMenu === 'translation'" class="settings-pane">
                <n-form label-placement="top" size="medium">
                  <n-form-item :label="t('settings.translation.baseUrl')">
                    <n-input
                      v-model:value="settings.baseUrl"
                      placeholder="https://api.openai.com"
                    />
                  </n-form-item>
                  <n-form-item :label="t('settings.translation.apiKey')">
                    <n-input
                      v-model:value="settings.apiKey"
                      placeholder="sk-..."
                      type="password"
                      class="code"
                      show-password-on="click"
                    />
                  </n-form-item>
                  <n-form-item :label="t('settings.translation.model')">
                    <n-input v-model:value="settings.model" placeholder="gpt-4o-mini" />
                  </n-form-item>
                  <n-button
                    type="primary"
                    secondary
                    :loading="validating"
                    @click="validateOpenAIConfig"
                  >
                    <template #icon>
                      <n-icon>
                        <Sparkle24Filled />
                      </n-icon>
                    </template>
                    {{ t("settings.translation.validate") }}
                  </n-button>

                  <n-divider />

                  <n-form-item :label="t('settings.translation.prompt')">
                    <n-input
                      v-model:value="settings.prompt"
                      type="textarea"
                      :autosize="{ minRows: 2, maxRows: 6 }"
                    />
                  </n-form-item>
                </n-form>
              </div>

              <div v-else-if="activeMenu === 'appearance'" class="settings-pane">
                <n-form label-placement="top" size="medium">
                  <n-form-item :label="t('settings.appearance.fontFamily')">
                    <n-input
                      v-model:value="settings.fontFamily"
                      :placeholder="t('settings.appearance.fontFamilyPlaceholder')"
                      class="code"
                    />
                  </n-form-item>
                  <n-form-item :label="t('settings.appearance.fontSize')">
                    <n-input-number v-model:value="settings.fontSize" :min="12" :max="32" />
                  </n-form-item>
                  <n-form-item :label="t('settings.appearance.showFurigana')">
                    <n-switch v-model:value="settings.showJapaneseFurigana" />
                  </n-form-item>
                </n-form>
              </div>

              <div v-else-if="activeMenu === 'input'" class="settings-pane">
                <n-form-item :label="t('settings.input.monitorClipboard')">
                  <n-switch v-model:value="settings.monitorClipboard" />
                </n-form-item>
                <n-form-item :label="t('settings.input.openaiCompatibleInput')">
                  <n-switch v-model:value="settings.openaiCompatibleInput" />
                </n-form-item>
                <n-form-item :label="t('settings.input.httpExample')">
                  <n-code :code="submitCommand" language="bash" word-wrap class="pre-code" />
                </n-form-item>
              </div>

              <div v-else-if="activeMenu === 'preprocess'" class="settings-pane">
                <n-form label-placement="top" size="medium">
                  <n-alert
                    :title="t('settings.preprocess.tipTitle')"
                    type="info"
                    class="preprocess-tip"
                    :closable="false"
                  >
                    {{ t("settings.preprocess.tipExample") }}
                  </n-alert>
                  <div class="replacement-rules">
                    <div v-if="!settings.replacements.length" class="rules-placeholder">
                      {{ t("settings.preprocess.placeholder") }}
                    </div>
                    <n-card
                      v-for="(rule, index) in settings.replacements"
                      :key="index"
                      size="small"
                      class="rule-card"
                      :bordered="true"
                    >
                      <n-space vertical size="small">
                        <n-form-item :label="t('settings.preprocess.pattern')">
                          <n-input
                            v-model:value="rule.pattern"
                            :placeholder="t('settings.preprocess.patternPlaceholder')"
                          />
                        </n-form-item>
                        <n-form-item :label="t('settings.preprocess.replacement')">
                          <n-input
                            v-model:value="rule.replacement"
                            :placeholder="t('settings.preprocess.replacementPlaceholder')"
                          />
                        </n-form-item>
                        <n-form-item :label="t('settings.preprocess.flags')">
                          <n-input
                            v-model:value="rule.flags"
                            :placeholder="t('settings.preprocess.flagsPlaceholder')"
                          />
                        </n-form-item>
                        <div class="rule-actions">
                          <n-button
                            text
                            type="error"
                            size="small"
                            @click="removeReplacementRule(index)"
                          >
                            {{ t("settings.preprocess.delete") }}
                          </n-button>
                        </div>
                      </n-space>
                    </n-card>
                  </div>
                  <n-space>
                    <n-button tertiary type="primary" @click="addReplacementRule">
                      {{ t("settings.preprocess.addRule") }}
                    </n-button>
                  </n-space>
                </n-form>
              </div>

              <div v-else-if="activeMenu === 'about'" class="settings-pane">
                <n-space vertical :size="16">
                  <n-card size="small" class="about-card" :bordered="true">
                    <n-flex align="center" justify="space-between">
                      <div class="about-meta">
                        <div class="about-label">{{ t("settings.about.version") }}</div>
                        <div class="about-version">{{ appVersion }}</div>
                      </div>
                      <n-button
                        type="primary"
                        secondary
                        :loading="checkingUpdate"
                        @click="checkForUpdates"
                      >
                        {{ t("settings.about.checkForUpdates") }}
                      </n-button>
                    </n-flex>
                  </n-card>

                  <n-alert
                    v-if="availableUpdate"
                    type="info"
                    :title="
                      t('settings.about.updateAvailableTitle', { version: availableUpdate.version })
                    "
                    class="update-alert"
                  >
                    <p v-if="availableUpdate.body" class="release-notes">
                      {{ availableUpdate.body }}
                    </p>
                    <n-space v-if="installingUpdate" vertical size="small" class="update-progress">
                      <n-progress
                        type="line"
                        :percentage="updateProgress ?? 0"
                        :processing="updateProgress === null"
                        :status="updateProgress === 100 ? 'success' : undefined"
                      />
                      <div class="progress-text">{{ downloadProgressText }}</div>
                    </n-space>
                    <n-space>
                      <n-button type="primary" :loading="installingUpdate" @click="installUpdate">
                        {{ t("settings.about.installUpdate") }}
                      </n-button>
                    </n-space>
                  </n-alert>

                  <n-alert
                    v-else-if="!isTauriEnv"
                    type="warning"
                    :title="t('settings.about.notInAppTitle')"
                    :closable="false"
                  >
                    {{ t("settings.about.notInAppMessage") }}
                  </n-alert>
                </n-space>
              </div>
            </n-flex>
          </div>
        </div>
      </n-card>
    </div>
  </n-config-provider>
</template>
<style>
html,
body {
  overscroll-behavior: none;
}

body {
  text-autospace: normal;
}

.settings-wrapper {
  height: 100vh;
  display: flex;
  overflow: hidden;
}

.settings-card {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.settings-card .n-card__content {
  height: 100%;
  display: flex;
  flex-direction: column;
}

.settings-card .n-card__content-inner {
  flex: 1;
  display: flex;
  flex-direction: column;
}

.settings-layout {
  flex: 1;
  display: flex;
  gap: 20px;
  align-items: stretch;
  overflow: hidden;
}

.settings-menu-pane {
  width: 180px;
  flex-shrink: 0;
  overflow-y: auto;
  max-height: 100%;
  background-color: rgba(0, 0, 0, 0.04);
  border: 1px solid var(--n-border-color);
  border-radius: 8px;
  padding: 8px;
}

.settings-content {
  padding: 4px;
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  max-height: 100%;
}

.settings-menu {
  width: 100%;
  --n-item-color-hover: rgba(0, 0, 0, 0.08);
}

.settings-menu .n-menu-item-content::before {
  border-radius: 8px;
  --n-item-color-hover: rgba(0, 0, 0, 0.08);
}

@media (prefers-color-scheme: dark) {
  .settings-menu-pane {
    background-color: rgba(255, 255, 255, 0.06);
  }

  .settings-menu .n-menu-item-content::before {
    --n-item-color-hover: rgba(255, 255, 255, 0.187);
  }
}

.settings-pane {
  width: 100%;
}

.code {
  font-family: monospace;
}

.pre-code {
  border: 1px solid var(--n-border-color);
  padding: 12px;
}

.preprocess-tip {
  margin-bottom: 10px;
}

.replacement-rules {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 12px;
}

.rule-card {
  box-shadow: none;
}

.rule-actions {
  display: flex;
  justify-content: flex-end;
}

.rules-placeholder {
  color: var(--n-text-color-3);
  padding: 6px 2px;
}

.about-card {
  box-shadow: none;
}

.about-meta {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.about-label {
  color: var(--n-text-color-2);
  font-size: 14px;
}

.about-version {
  font-weight: 600;
  font-size: 18px;
}

.release-notes {
  margin: 4px 0 12px 0;
  white-space: pre-wrap;
}

.update-progress {
  margin: 8px 0;
}

.progress-text {
  color: var(--n-text-color-2);
  font-size: 14px;
}

.update-alert {
  box-shadow: none;
}

/* Use default arrow cursor for Naive UI interactive elements */
.n-button,
.n-button *,
.n-switch,
.n-switch *,
.n-select,
.n-select *,
.n-base-select-option,
.n-tag,
.n-tag *,
.n-menu,
.n-menu * {
  cursor: default !important;
}
</style>
