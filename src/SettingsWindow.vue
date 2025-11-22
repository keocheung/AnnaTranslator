<script setup lang="ts">
import {
  NCard,
  NCode,
  NConfigProvider,
  NForm,
  NFormItem,
  NInput,
  NInputNumber,
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

const submitCommand = `curl -X POST http://127.0.0.1:${settings.value.apiKey}/submit \\
              -H 'Content-Type: text/plain' \\
              --data-raw '<待翻译文本>'`;

const theme = computed(() => (useOsTheme().value === "dark" ? darkTheme : null));
hljs.registerLanguage("bash", bash);
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
</style>
