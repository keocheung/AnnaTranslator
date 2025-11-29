# Anna Translator

一个视觉小说可用的实时翻译器，跨平台、轻量且现代。

## 功能概览

- 前置窗口：默认置顶，可切换。
- 本地 HTTP 监听：`POST http://127.0.0.1:17889/submit`，纯文本正文触发翻译。
- OpenAI 兼容输入：可选开关，接受 `/v1/chat/completions` 形式请求，将用户消息推送到前端（响应固定 404）。
- OpenAI 翻译：可配置 Base URL、Key、Model、Prompt，支持流式输出。
- 字体与字号：自定义字体栈和大小，便于搭配 Gal 字体。
- 手动输入：可手动输入测试翻译，支持停止流。

## 推送示例

```bash
curl -X POST http://127.0.0.1:17889/submit \
  -H "Content-Type: text/plain" \
  --data-raw 'こんにちは。'

# 开启“OpenAI 兼容输入”后，可接受以下格式（返回固定 404，仍会推送原文到前端）
curl -X POST http://127.0.0.1:17889/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{ "messages": [ { "role": "user", "content": "こんにちは。" } ] }'
```

### Ren'Py

Ren'Py游戏原生支持语音朗读，在不同的系统上会调用不同的朗读命令。

macOS在本仓库目录内用以下命令启动游戏（替换成实际的游戏路径），即可覆盖默认的朗读命令，将文本发送到Anna Translator
```bash
PATH=$(pwd)/script/renpy:$PATH {{GAME_APP_PATH}}.app/Contents/MacOS/{{GAME_BINARY_NAME}}
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

## 开发

```bash
# 安装依赖（需要本机已有 Bun）
bun install

# 开发调试（启动 Vite 与 Tauri 2，并开启 devtools）
bun tauri dev

# 构建（生产模式关闭 devtools）
bunx tauri build
```

## TODO
* [ ] logo
* [ ] 自带更新工具
* [ ] 日语分词及注音
  * [ ] 词典
* [ ] OCR输入
  * [ ] OCR区域选择窗口
  * [ ] macOS自带OCR
* [ ] 支持更多翻译接口
  * [ ] Ollama
  * [ ] 传统在线API
