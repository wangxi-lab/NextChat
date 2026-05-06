# NextChat 桌面端 AI 问答工具 TODO

## Phase 1: 打通本地开发链路

- [x] 确认 NextChat、GenericAgent、本地端口和启动命令。
- [x] 在 GenericAgent 中新增本地 HTTP/SSE Adapter 文件。
- [x] Adapter 启动 `GeneraticAgent`，并提供 `GET /health`。
- [x] 提供 `POST /v1/chat`，把用户输入提交给 GenericAgent。
- [x] 提供 `POST /v1/abort`，支持用户中断当前任务。
- [x] 用 curl 或 PowerShell 验证 `/health`、`/v1/chat`、`/v1/abort`。

## Phase 2: 接入 NextChat Provider

- [x] 在 NextChat 中新增 `GenericAgent` 服务商常量和模型占位。
- [x] 新增 GenericAgent 客户端实现，调用本地 Adapter 的 SSE 接口。
- [x] 将 `delta`、`done`、`error` 映射到 NextChat 现有聊天回调。
- [x] 将 `status`、`tool_call`、`tool_result`、`citation` 转成可展示内容。
- [x] 在设置页加入 GenericAgent Endpoint、访问令牌、默认模型名。
- [ ] 在桌面端选择 GenericAgent 后完成一次普通流式问答。

## Phase 3: 接入火山方舟 RAG Skill

- [x] 在 GenericAgent `memory/volc_ark_rag` 中新增 RAG skill。
- [x] Skill 支持读取 `ARK_API_KEY`、`ARK_BASE_URL`、`ARK_MODEL`、`ARK_KNOWLEDGE_BASE_IDS`、`ARK_TOP_K`。
- [x] Skill 实现火山方舟私域知识库搜索调用。
- [x] Adapter 不再直接调用 RAG，只提示 GenericAgent 可使用 RAG skill。
- [x] NextChat 只保留启用企业 RAG skill 的开关，不保存火山方舟密钥。
- [x] 验证 API Key 缺失、知识库 ID 错误、接口失败时的错误提示。

## Phase 4: 本地操作授权

- [x] 设计本地能力授权配置结构。
- [x] 在 NextChat 设置页加入文件、命令、浏览器、屏幕/鼠标/键盘能力开关。
- [x] 支持配置允许访问的工作目录。
- [x] 支持配置命令白名单或危险命令黑名单。
- [ ] GenericAgent 执行工具前检查授权策略。
- [ ] 未授权操作通过 SSE 返回拒绝原因。
- [ ] 验证授权目录内操作成功、未授权目录操作失败。

## Phase 5: 联调与验收

- [x] 启动 NextChat Web/桌面开发服务和 GenericAgent 本地服务。
- [ ] 验证普通问答、多轮问答、用户中断。
- [ ] 验证 RAG 问答能返回知识库引用。
- [ ] 验证本地文件操作和命令执行受授权策略控制。
- [ ] 验证错误状态在聊天界面可读。
- [ ] 记录本地开发版启动步骤和必要环境变量。
