# Anna Translator

一个视觉小说可用的实时翻译器，跨平台、轻量且现代。

## 功能概览

- 前置窗口
  - 窗口置顶
  - 自动隐藏标题栏
- 文本输入
  - 本地HTTP请求
  - 剪贴板监听
- 翻译
  - OpenAI API：可配置 Base URL、Key、Model、Prompt，支持流式输出。
  - 基于SQLite的翻译缓存
  - 会话内历史记录
- 显示
  - 自定义正文和原文的字体和大小。

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

Deno 1.44+ 即可直接管理 npm 依赖，首次运行会自动拉取并生成缓存。
项目依赖统一写在 `deno.json` 的 `imports` 字段，`package.json` 仅保留脚本。

```bash
# 安装依赖（使用 Deno 自动管理 npm 包缓存）
deno task dev

# 开发调试（启动 Vite 与 Tauri 2，并开启 devtools）
deno task tauri:dev

# 构建（生产模式关闭 devtools）
deno task tauri:build
```

## TODO

- [ ] logo
- [x] 自带更新工具
- [x] 日语分词及注音
  - [ ] 词典
- [ ] OCR输入
  - [ ] OCR区域选择窗口
  - [ ] macOS自带OCR
- [ ] 支持更多翻译接口
  - [ ] Ollama
  - [ ] 传统在线API
