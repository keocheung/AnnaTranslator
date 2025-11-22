<script setup lang="ts">
import {
  NCard,
  NConfigProvider,
  NDivider,
  NForm,
  NFormItem,
  NInput,
  NInputNumber,
  NLayout,
  NLayoutContent,
  NSpace,
  NSwitch
} from "naive-ui";
import { useSettingsState } from "./settings";

const settings = useSettingsState();
</script>

<template>
  <n-config-provider>
    <n-layout content-style="padding: 12px 18px 28px 18px;">
      <n-layout-content>
        <n-space vertical size="large">
          <n-card class="card" title="接口设置" size="large" :bordered="false">
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

              <n-form-item label="监听剪贴板并自动翻译">
                <n-switch v-model:value="settings.monitorClipboard" />
              </n-form-item>

              <n-form-item label="OpenAI 兼容输入（/v1/chat/completions）">
                <n-switch v-model:value="settings.openaiCompatibleInput" />
              </n-form-item>

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
