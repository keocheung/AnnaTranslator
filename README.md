# Local Translator (Tauri 2 + Vue + Naive UI)

一个类似 LunaTranslator 的 Galgame 翻译悬浮窗，使用 Tauri 2 + Vue 3 + Naive UI + Bun 构建。

## 功能概览
- 前置窗口：默认置顶，可切换。
- 本地 HTTP 监听：`POST http://127.0.0.1:17889/submit`，JSON `{ "text": "..." }` 触发翻译。
- OpenAI 兼容输入：可选开关，接受 `/v1/chat/completions` 形式请求，将用户消息推送到前端（响应固定 404）。
- OpenAI 翻译：可配置 Base URL、Key、Model、Prompt，支持流式输出。
- 字体与字号：自定义字体栈和大小，便于搭配 Gal 字体。
- 手动输入：可手动输入测试翻译，支持停止流。

## 快速开始
```bash
# 安装依赖（需要本机已有 Bun）
bun install

# 开发调试（启动 Vite 与 Tauri 2，并开启 devtools）
bun run tauri:dev

# 构建（生产模式关闭 devtools）
bunx tauri build
```

## 推送示例
```bash
curl -X POST http://127.0.0.1:17889/submit \
  -H "Content-Type: application/json" \
  -d '{"text":"こんにちは。"}'

# 开启“OpenAI 兼容输入”后，可接受以下格式（返回固定 404，仍会推送原文到前端）
curl -X POST http://127.0.0.1:17889/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{ "messages": [ { "role": "user", "content": "こんにちは。" } ] }'
```

## 配置项
- `OpenAI Base URL`：默认 `https://api.openai.com`，可改为代理或自建兼容接口。
- `API Key`：必填。
- `Model`：默认 `gpt-4o-mini`，可按需修改。
- `Prompt`：默认的 Gal 文本翻译提示词。
- `字体/字号`：译文/原文显示所用字体。
- `置顶开关`：窗口置顶切换。
- `OpenAI 兼容输入`：默认关闭，开启后接受 `/v1/chat/completions` 形式请求，仅提取用户消息并推送到前端，响应固定 404。
- 环境变量 `TRANSLATOR_PORT`：可覆盖监听端口（默认 17889）。

## 目录
- `src/`：Vue 3 + Naive UI 前端
- `src-tauri/`：Tauri 2 后端（Axum 本地 HTTP 监听）
```
