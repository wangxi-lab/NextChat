# NextChat 桌面端 AI 问答工具方案

## Summary

基于当前 NextChat 做 Windows 桌面交互层，新增 `GenericAgent` 作为一个本地 AI Provider；NextChat 只负责聊天 UI、会话管理、流式展示和权限配置。后台由同级目录 `GenericAgent` 负责 LLM 调度、任务编排、本地操作和 RAG 调用。

第一版采用“本地开发版”：桌面端连接本机 GenericAgent 服务，RAG 后端使用火山方舟私域知识库搜索接口。

参考：

- 火山方舟私域知识库搜索文档：https://www.volcengine.com/docs/84313/1350012?lang=zh
- 火山方舟 Responses API / 工具调用相关文档：https://www.volcengine.com/docs/82379/1873396

## Key Changes

### NextChat 桌面端

- 新增 `GenericAgent` Provider。
- 增加服务商、模型占位和配置项：
  - `GenericAgent Endpoint`
  - 访问令牌
  - 默认模型名
  - 火山方舟 API Key
  - 火山方舟知识库 ID 列表
- 复用现有聊天流：`onUserInput -> getClientApi(...).llm.chat(...)`。
- 复用 Tauri 当前 `stream_fetch` 能力，在桌面端稳定接收本地 HTTP/SSE 流。

### GenericAgent 本地 Adapter

- 在 GenericAgent 中新增本地 HTTP/SSE Adapter。
- 启动 `GeneraticAgent` 后暴露：
  - `GET /health`
  - `POST /v1/chat`
  - `POST /v1/abort`
- `POST /v1/chat` 接收 NextChat 的消息、会话 ID、权限上下文、RAG 配置，并以 SSE 返回：
  - `delta`：回答增量文本
  - `status`：思考或执行状态
  - `tool_call` / `tool_result`：本地工具调用过程
  - `citation`：RAG 引用
  - `done` / `error`：完成或失败
- GenericAgent 保留任务编排权，NextChat 不直接调用 LLM 或 RAG。

### RAG 后端

- RAG 采用火山方舟私域知识库搜索接口。
- GenericAgent 内新增 `VolcArkRagClient`，负责调用火山方舟 Responses API / Knowledge Search。
- 配置项包括：
  - `ARK_API_KEY`
  - `ARK_BASE_URL`
  - `ARK_MODEL`
  - `ARK_KNOWLEDGE_BASE_IDS`
  - `ARK_TOP_K`
- GenericAgent 在需要企业知识库时调用 RAG，再把检索结果、引用和最终回答统一流式返回给 NextChat。
- NextChat 第一版只展示引用，不做知识库管理后台。

### 本地操作授权

- NextChat 设置页增加本地能力开关：
  - 文件读写
  - 命令执行
  - 浏览器或网页访问
  - 屏幕、鼠标、键盘能力
- 配置授权后的工作目录、命令白名单或危险命令黑名单。
- GenericAgent 在执行本地工具前读取授权策略。
- 未授权能力直接拒绝，并通过 SSE 返回说明。
- 第一版不做每次弹窗确认，按“授权后自动”执行。

## Test Plan

### NextChat 桌面端

- 能选择 `GenericAgent` Provider。
- 普通问答可以流式输出。
- 中断生成能调用 `/v1/abort`。
- 设置项保存后重启仍可用。

### GenericAgent Adapter

- `/health` 返回运行状态。
- `/v1/chat` 能把 GenericAgent 队列输出转换成 SSE。
- 出错、超时、用户中断都有明确事件返回。
- 多轮会话不会把 NextChat 历史和 GenericAgent 内部历史重复注入。

### RAG

- 配置火山方舟 Key 和知识库 ID 后，问题能触发私域知识库搜索。
- 回答中能展示引用来源。
- API Key 缺失、知识库 ID 错误、火山接口失败时，前端显示可理解错误。

### 本地操作

- 未授权目录不可读取或写入。
- 授权目录内可执行文件操作。
- 命令执行受策略约束。
- 工具调用过程能在聊天界面展示为状态或工具结果。

## Assumptions

- 第一版按“本地开发版”实现，不做完整安装包和 Python sidecar 打包。
- GenericAgent 位于 `e:\code\app\GenericAgent`，NextChat 位于 `e:\code\app\NextChat`。
- RAG 后端使用火山方舟私域知识库搜索接口。
- 火山方舟的 API Key、模型名、知识库 ID 由本地配置提供，不写死在代码里。
- NextChat 继续作为 UI，不承担 Agent 编排、本地工具执行或知识库检索决策。
