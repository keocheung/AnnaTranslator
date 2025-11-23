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
  NInput,
  NInputNumber,
  NSpace,
  NSwitch,
  NTabs,
  NTabPane,
  darkTheme,
  useOsTheme,
} from "naive-ui";
import { useSettingsState } from "./settings";
import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import { computed } from "vue";

const settings = useSettingsState();

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
</script>

<template>
  <n-config-provider :theme="theme" :hljs="hljs">
    <div class="settings-wrapper">
      <n-card class="card settings-card" :bordered="false">
        <n-tabs type="line" placement="left">
          <n-tab-pane name="translation" label="翻译">
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

              <n-form-item label="翻译提示词">
                <n-input
                  v-model:value="settings.prompt"
                  type="textarea"
                  :autosize="{ minRows: 2, maxRows: 6 }"
                />
              </n-form-item>
            </n-form>
          </n-tab-pane>

          <n-tab-pane name="appearance" label="外观">
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
          </n-tab-pane>

          <n-tab-pane name="input" label="输入">
            <n-form-item label="监听剪贴板">
              <n-switch v-model:value="settings.monitorClipboard" />
            </n-form-item>
            <n-form-item label="OpenAI兼容输入（/v1/chat/completions）">
              <n-switch v-model:value="settings.openaiCompatibleInput" />
            </n-form-item>
            <n-form-item label="本地HTTP推送示例">
              <n-code :code="submitCommand" language="bash" word-wrap class="pre-code" />
            </n-form-item>
            <n-divider />
          </n-tab-pane>

          <n-tab-pane name="preprocess" label="输入处理">
            <n-form label-placement="top" size="medium">
              <n-alert
                title="按照顺序依次进行替换（Rust 正则语法）"
                type="info"
                class="preprocess-tip"
                :closable="false"
              >
                示例：使用 <code>第(\\d+)个</code> 替换为 <code>$1. </code> 可将“第12个”转为“12. ”。
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
          </n-tab-pane>
        </n-tabs>
      </n-card>
    </div>
  </n-config-provider>
</template>
<style>
body {
  text-autospace: normal;
}

.settings-wrapper {
  min-height: 100vh;
  display: flex;
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
