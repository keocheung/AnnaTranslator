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
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { getOpenAIConstructor } from "./openaiClient";
import { purpleThemeOverrides } from "./theme";
import { resolveLocale } from "./i18n";

const settings = useSettingsState();
const { t } = useI18n();
const validating = ref(false);
const { message } = createDiscreteApi(["message"], {
  configProviderProps: {
    themeOverrides: purpleThemeOverrides,
  },
});
const activeMenu = ref("general");
const menuOptions = computed(() => [
  { label: t("settings.menu.general"), key: "general" },
  { label: t("settings.menu.translation"), key: "translation" },
  { label: t("settings.menu.appearance"), key: "appearance" },
  { label: t("settings.menu.input"), key: "input" },
  { label: t("settings.menu.preprocess"), key: "preprocess" },
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
  padding-left: 4px;
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

.settings-content {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  max-height: 100%;
  padding-right: 4px;
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
