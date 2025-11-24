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
  NIcon,
  NInput,
  NInputNumber,
  NMenu,
  NSpace,
  NSwitch,
  darkTheme,
  useOsTheme,
  createDiscreteApi,
} from "naive-ui";
import { Sparkle24Filled } from "@vicons/fluent";
import { useSettingsState } from "./settings";
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import { computed, ref } from "vue";
import { getOpenAIConstructor } from "./openaiClient";

const settings = useSettingsState();
const validating = ref(false);
const { message } = createDiscreteApi(["message"]);
const activeMenu = ref("translation");
const menuOptions = [
  { label: "翻译", key: "translation" },
  { label: "外观", key: "appearance" },
  { label: "输入", key: "input" },
  { label: "输入处理", key: "preprocess" },
];
const handleMenuUpdate = (key: string) => {
  activeMenu.value = key;
};

const submitCommand = computed(
  () => `curl -X POST http://127.0.0.1:${settings.value.serverPort}/submit \\
              -H 'Content-Type: text/plain' \\
              --data-raw '<待翻译文本>'`
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
    message.error("请先填写 OpenAI API Key");
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

    message.success("验证成功");
  } catch (error) {
    message.error((error as Error)?.message ?? "验证失败");
  } finally {
    validating.value = false;
  }
}
</script>

<template>
  <n-config-provider :theme="theme" :hljs="hljs">
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
            <div v-if="activeMenu === 'translation'" class="settings-pane">
              <n-form label-placement="top" size="medium">
                <n-form-item label="OpenAI Base URL">
                  <n-input v-model:value="settings.baseUrl" placeholder="https://api.openai.com" />
                </n-form-item>
                <n-form-item label="OpenAI API Key">
                  <n-input
                    v-model:value="settings.apiKey"
                    placeholder="sk-..."
                    type="password"
                    class="code"
                    show-password-on="click"
                  />
                </n-form-item>
                <n-form-item label="模型">
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
                  验证设置
                </n-button>

                <n-divider />

                <n-form-item label="翻译提示词">
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
                <n-form-item label="字体">
                  <n-input
                    v-model:value="settings.fontFamily"
                    placeholder="自定义字体栈"
                    class="code"
                  />
                </n-form-item>
                <n-form-item label="字号">
                  <n-input-number v-model:value="settings.fontSize" :min="12" :max="32" />
                </n-form-item>
              </n-form>
            </div>

            <div v-else-if="activeMenu === 'input'" class="settings-pane">
              <n-form-item label="监听剪贴板">
                <n-switch v-model:value="settings.monitorClipboard" />
              </n-form-item>
              <n-form-item label="OpenAI兼容输入（/v1/chat/completions）">
                <n-switch v-model:value="settings.openaiCompatibleInput" />
              </n-form-item>
              <n-form-item label="本地HTTP推送示例">
                <n-code :code="submitCommand" language="bash" word-wrap class="pre-code" />
              </n-form-item>
            </div>

            <div v-else-if="activeMenu === 'preprocess'" class="settings-pane">
              <n-form label-placement="top" size="medium">
                <n-alert
                  title="按照顺序依次进行替换（Rust 正则语法）"
                  type="info"
                  class="preprocess-tip"
                  :closable="false"
                >
                  示例：使用<code>第(\\d+)个</code>替换为<code>$1. </code>可将“第12个”转为“12. ”。
                </n-alert>
                <div class="replacement-rules">
                  <div v-if="!settings.replacements.length" class="rules-placeholder">
                    暂无规则，点击下方“新增规则”开始配置。
                  </div>
                  <n-card
                    v-for="(rule, index) in settings.replacements"
                    :key="index"
                    size="small"
                    class="rule-card"
                    :bordered="true"
                  >
                    <n-space vertical size="small">
                      <n-form-item label="正则表达式">
                        <n-input v-model:value="rule.pattern" placeholder="例如：第(\\d+)个" />
                      </n-form-item>
                      <n-form-item label="替换为">
                        <n-input v-model:value="rule.replacement" placeholder="$1. " />
                      </n-form-item>
                      <n-form-item label="Flags（可选）">
                        <n-input v-model:value="rule.flags" placeholder="例如：im" />
                      </n-form-item>
                      <div class="rule-actions">
                        <n-button
                          text
                          type="error"
                          size="small"
                          @click="removeReplacementRule(index)"
                        >
                          删除
                        </n-button>
                      </div>
                    </n-space>
                  </n-card>
                </div>
                <n-space>
                  <n-button tertiary type="primary" @click="addReplacementRule">新增规则</n-button>
                </n-space>
              </n-form>
            </div>
          </div>
        </div>
      </n-card>
    </div>
  </n-config-provider>
</template>
<style>
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
  gap: 24px;
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

@media (prefers-color-scheme: dark) {
  .settings-menu-pane {
    background-color: rgba(255, 255, 255, 0.06);
  }
}

.settings-menu {
  width: 100%;
}

.n-menu .n-menu-item-content::before {
  border-radius: 8px;
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
</style>
