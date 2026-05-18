"use client";

import { ApiPath, DEFAULT_MODELS, ServiceProvider } from "@/app/constant";
import { useAccessStore, useChatStore } from "@/app/store";
import { collectModelsWithDefaultModel } from "@/app/utils/model";
import { getMessageTextContent } from "@/app/utils";
import { fetch } from "@/app/utils/stream";
import { ChatOptions, LLMApi, LLMModel, LLMUsage, SpeechOptions } from "../api";

type GenericAgentEvent = {
  id?: string;
  content?: string;
  message?: string;
  items?: any[];
  name?: string;
  data?: any;
  recoverable?: boolean;
};

export class GenericAgentApi implements LLMApi {
  path(path: string): string {
    const accessStore = useAccessStore.getState();
    const baseUrl = (
      accessStore.genericAgentUrl || ApiPath.GenericAgent
    ).replace(/\/$/, "");
    return `${baseUrl}${path}`;
  }

  async chat(options: ChatOptions) {
    const accessStore = useAccessStore.getState();
    const session = useChatStore.getState().currentSession();
    const chatMode = accessStore.genericAgentChatMode || "";
    const selectedSkill =
      chatMode === "skill" ? accessStore.genericAgentSelectedSkill || "" : "";
    const useRag = chatMode === "rag";
    const controller = new AbortController();
    options.onController?.(controller);

    controller.signal.addEventListener("abort", () => {
      fetch(this.path("/v1/abort"), {
        method: "POST",
        headers: this.headers(),
        body: "{}",
      }).catch((err) => console.warn("[GenericAgent] abort failed", err));
    });

    const requestPayload = {
      request_id: session.id,
      conversation_id: session.id,
      input: getMessageTextContent(options.messages.at(-1) as any),
      messages: options.messages.map((message) => ({
        role: message.role,
        content: getMessageTextContent(message as any),
      })),
      model: accessStore.genericAgentModel || options.config.model,
      mode: chatMode,
      skill: {
        name: selectedSkill,
        mode: selectedSkill ? "force" : "off",
      },
      rag: {
        enabled: useRag,
        strict: useRag,
        mode: useRag ? "force" : "off",
      },
      permissions: {
        file_system: {
          enabled: accessStore.genericAgentAllowFileSystem,
          allowed_dirs: accessStore.genericAgentAllowedDirs
            .split(/\r?\n|,/)
            .map((item) => item.trim())
            .filter(Boolean),
        },
        shell: {
          enabled: accessStore.genericAgentAllowShell,
          allowlist: accessStore.genericAgentCommandAllowlist
            .split(/\r?\n|,/)
            .map((item) => item.trim())
            .filter(Boolean),
          denylist: accessStore.genericAgentCommandDenylist
            .split(/\r?\n|,/)
            .map((item) => item.trim())
            .filter(Boolean),
        },
        browser: { enabled: accessStore.genericAgentAllowBrowser },
        screen_control: {
          enabled: accessStore.genericAgentAllowScreenControl,
        },
      },
    };

    const res = await fetch(this.path("/v1/chat"), {
      method: "POST",
      headers: this.headers(),
      body: JSON.stringify(requestPayload),
      signal: controller.signal,
    });

    if (!res.ok || !res.body) {
      throw new Error(`GenericAgent request failed: ${res.status}`);
    }

    let message = "";
    let citations = "";
    let finalMessage = "";
    const tools = new Map<string, any>();
    const statusToolId = "generic-agent-status";
    let hasStatusTool = false;

    await readSse(res, (event, data) => {
      if (event === "delta") {
        message += data.content || "";
        options.onUpdate?.(message, data.content || "");
      } else if (event === "status") {
        const tool = {
          id: statusToolId,
          type: "function",
          function: {
            name: data.message || "GenericAgent is running",
          },
        };
        if (hasStatusTool) {
          options.onAfterTool?.(tool);
        } else {
          hasStatusTool = true;
          options.onBeforeTool?.(tool);
        }
      } else if (event === "citation") {
        citations = formatCitations(data.items || []);
      } else if (event === "tool_call") {
        const id = data.id || `${Date.now()}`;
        const tool = {
          id,
          type: "function",
          function: {
            name: data.name || data.data?.tool_name || "GenericAgent Tool",
            arguments: JSON.stringify(data.data || {}),
          },
        };
        tools.set(id, tool);
        options.onBeforeTool?.(tool);
      } else if (event === "tool_result") {
        const id = data.id || `${Date.now()}`;
        const tool = {
          ...(tools.get(id) || {
            id,
            type: "function",
            function: { name: data.name || "GenericAgent Tool" },
          }),
          content: data.content || data.message || "",
          isError: false,
        };
        options.onAfterTool?.(tool);
      } else if (event === "error") {
        const text = data.message || "GenericAgent error";
        if (data.recoverable) {
          message += `\n\n> ${text}\n\n`;
          options.onUpdate?.(message, text);
        } else {
          throw new Error(text);
        }
      } else if (event === "done") {
        finalMessage = data.content || message;
        if (isKnowledgeNotFound(finalMessage)) {
          citations = "";
        }
        if (hasStatusTool) {
          options.onAfterTool?.({
            id: statusToolId,
            type: "function",
            function: { name: "GenericAgent completed" },
            isError: false,
          });
        }
      }
    });

    options.onFinish(finalMessage + citations, res);
  }

  async speech(_options: SpeechOptions): Promise<ArrayBuffer> {
    throw new Error("GenericAgent does not support speech");
  }

  async usage(): Promise<LLMUsage> {
    return { used: 0, total: 0 };
  }

  async models(): Promise<LLMModel[]> {
    return collectModelsWithDefaultModel(
      DEFAULT_MODELS,
      "",
      "generic-agent@GenericAgent",
    )
      .filter(
        (m) =>
          !!m.provider &&
          m.provider.providerName === ServiceProvider.GenericAgent,
      )
      .map((m) => ({
        name: m.name,
        displayName: m.displayName,
        available: m.available,
        provider: m.provider!,
        sorted: m.sorted,
      }));
  }

  private headers(): Record<string, string> {
    const accessStore = useAccessStore.getState();
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
      Accept: "text/event-stream",
    };
    if (accessStore.genericAgentToken) {
      headers.Authorization = `Bearer ${accessStore.genericAgentToken}`;
    }
    return headers;
  }
}

async function readSse(
  res: Response,
  onEvent: (event: string, data: GenericAgentEvent) => void,
) {
  const reader = res.body!.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });

    let boundary = buffer.indexOf("\n\n");
    while (boundary >= 0) {
      const raw = buffer.slice(0, boundary).trim();
      buffer = buffer.slice(boundary + 2);
      dispatchSse(raw, onEvent);
      boundary = buffer.indexOf("\n\n");
    }
  }

  if (buffer.trim()) {
    dispatchSse(buffer.trim(), onEvent);
  }
}

function dispatchSse(
  raw: string,
  onEvent: (event: string, data: GenericAgentEvent) => void,
) {
  let event = "message";
  const dataLines: string[] = [];
  for (const line of raw.split(/\r?\n/)) {
    if (line.startsWith("event:")) {
      event = line.slice(6).trim();
    } else if (line.startsWith("data:")) {
      dataLines.push(line.slice(5).trim());
    }
  }
  if (dataLines.length === 0) return;
  onEvent(event, JSON.parse(dataLines.join("\n")));
}

function formatCitations(items: any[]): string {
  if (!items.length) return "";
  const rows = items.slice(0, 5).map((item, index) => {
    const docInfo = item.doc_info || {};
    const title =
      item.title ||
      item.knowledge_title ||
      item.doc_name ||
      item.knowledge_filename ||
      item.chunk_title ||
      docInfo.doc_name ||
      item.id ||
      `引用 ${index + 1}`;
    const score = item.score ? ` score=${item.score}` : "";
    const url = getCitationUrl(item);
    const source = url ? `[${title}](${url})` : title;
    const docId = item.doc_id || docInfo.doc_id;
    const chunkId = item.chunk_id;
    const meta = [
      docId ? `doc_id=${docId}` : "",
      chunkId !== undefined ? `chunk_id=${chunkId}` : "",
    ]
      .filter(Boolean)
      .join("; ");
    const refNo = index + 1;
    return `> [${refNo}](#kb-ref-${refNo} "kb-source-anchor") ${source}${score}${meta ? ` (${meta})` : ""}`;
  });
  return `\n\n### 知识库引用\n${rows.join("\n")}\n`;
}

function isKnowledgeNotFound(text: string): boolean {
  return text.includes("知识库未检索到相关内容");
}

function getCitationUrl(item: any): string {
  const candidates = [
    item.url,
    item.link,
    item.source_url,
    item.download_url,
    item.doc_url,
    item.href,
    item.doc_info?.url,
    item.doc_info?.link,
    item.doc_info?.source_url,
    item.doc_info?.download_url,
    item.doc_info?.doc_url,
    item.doc_info?.href,
    ...(Array.isArray(item.chunk_attachment)
      ? item.chunk_attachment.map((attachment: any) => attachment?.link)
      : []),
  ];
  return (
    candidates.find(
      (value) =>
        typeof value === "string" &&
        /^https?:\/\//i.test(value) &&
        value.toLowerCase() !== "javascript:void(0)",
    ) || ""
  );
}
