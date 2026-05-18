# NextChat 集成 Goose

NextChat 桌面端通过 Tauri 命令启动并调用本机 Goose 后台服务：

- 前端按钮：聊天输入区的“本地Agent”
- 后端命令：`start_goose_agent`、`goose_status`、`goose_chat`
- 启动后台服务：优先查找 `bin/Goose-win32-x64/resources/bin/goosed.exe`
- 配置文件：项目根目录 `goose.config.json`
- 默认后台地址：`127.0.0.1:32123`，避免和 Next.js 的 `3000` 端口冲突
- 默认设置：`GOOSE_TLS=false`，NextChat 通过本地 HTTP `/reply` 调用后台服务

## Provider 配置

编辑项目根目录的 `goose.config.json`：

```json
{
  "provider": "openai",
  "providerType": "openai",
  "model": "gpt-4o-mini",
  "apiKey": "sk-...",
  "baseUrl": "https://api.openai.com",
  "host": "127.0.0.1",
  "port": 32123,
  "tls": false,
  "secretKey": "nextchat-local-goose-agent",
  "env": {}
}
```

启动 `goosed.exe agent` 时，NextChat 会自动转换这些字段：

- `provider` -> `GOOSE_PROVIDER`
- `providerType` -> `GOOSE_PROVIDER__TYPE`
- `model` -> `GOOSE_MODEL`
- `baseUrl` -> `GOOSE_PROVIDER__HOST`
- `host` -> `GOOSE_HOST`
- `port` -> `GOOSE_PORT`
- `tls` -> `GOOSE_TLS`
- `secretKey` -> `GOOSE_SERVER__SECRET_KEY`

`apiKey` 会写入 `GOOSE_PROVIDER__API_KEY`，并按 provider 额外自动映射：

- `openai` -> `OPENAI_API_KEY`
- `anthropic` -> `ANTHROPIC_API_KEY`
- `deepseek` -> `DEEPSEEK_API_KEY`
- `google` / `gemini` -> `GOOGLE_API_KEY`
- `openrouter` -> `OPENROUTER_API_KEY`
- `xai` -> `XAI_API_KEY`

如果 Goose 新增了环境变量，直接写到 `env` 里即可，`env` 会覆盖前面自动生成的环境变量：

```json
{
  "provider": "openai",
  "model": "gpt-4o-mini",
  "env": {
    "OPENAI_API_KEY": "sk-...",
    "GOOSE_PROVIDER__HOST": "https://api.openai.com"
  }
}
```

MiniMax 这类 OpenAI-compatible 端点可以这样配。注意 `providerType: "openai"` 会让 NextChat 实际传给 Goose 的 provider 为 `openai`，并把 `baseUrl` 转成 Goose 识别的 `OPENAI_BASE_URL`：

```json
{
  "provider": "MiniMax",
  "providerType": "openai",
  "model": "你的模型 API 名称",
  "apiKey": "你的 key",
  "baseUrl": "https://api.minimaxi.com/v1"
}
```

## 开发环境

安装并配置 Goose 后，确保 `goose` 在系统 `PATH` 中：

```bash
goose --version
```

也可以通过环境变量指定 Goose 可执行文件：

```bash
NEXTCHAT_GOOSE_BIN=/absolute/path/to/goose yarn app:dev
```

Windows PowerShell：

```powershell
$env:NEXTCHAT_GOOSE_BIN="C:\path\to\goose.exe"
yarn app:dev
```

## 打包

当前代码会按下面顺序查找 Goose：

1. `NEXTCHAT_GOOSE_AGENT_BIN` 指定的 Goose 后台服务
2. 安装目录或资源目录下的 `bin/Goose-win32-x64/resources/bin/goosed.exe`
3. 安装目录或资源目录下的 `goosed` / `goosed.exe`
4. 安装目录或资源目录下的 `bin/goosed` / `bin/goosed.exe`

要把 Goose 放进安装包，可以把对应平台的目录放到 `src-tauri/bin`，`tauri.conf.json` 已配置 `resources: ["bin/**/*", "../goose.config.json"]`，打包后会随安装包带上 Goose 和 `goose.config.json`。Windows 包推荐目录：

- Windows 后台服务：`src-tauri/bin/Goose-win32-x64/resources/bin/goosed.exe`
- macOS/Linux：`goose`

如果后续希望改为长连接模式，可以把 `app/utils/goose.ts` 和 `src-tauri/src/goose.rs` 中的实现替换为 `goose acp` 或本地 HTTP 服务，前端聊天入口不需要大改。
