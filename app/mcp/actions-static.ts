import {
  DEFAULT_MCP_CONFIG,
  McpConfigData,
  McpRequestMessage,
  ServerConfig,
  ServerStatusResponse,
} from "./types";

export async function getClientsStatus(): Promise<
  Record<string, ServerStatusResponse>
> {
  return {};
}

export async function getClientTools() {
  return null;
}

export async function getAvailableClientsCount() {
  return 0;
}

export async function getAllTools() {
  return [];
}

export async function initializeMcpSystem() {}

export async function addMcpServer(
  clientId: string,
  config: ServerConfig,
): Promise<McpConfigData> {
  return {
    ...DEFAULT_MCP_CONFIG,
    mcpServers: {
      ...DEFAULT_MCP_CONFIG.mcpServers,
      [clientId]: config,
    },
  };
}

export async function pauseMcpServer(): Promise<McpConfigData> {
  return DEFAULT_MCP_CONFIG;
}

export async function resumeMcpServer(): Promise<void> {}

export async function removeMcpServer(): Promise<McpConfigData> {
  return DEFAULT_MCP_CONFIG;
}

export async function restartAllClients(): Promise<McpConfigData> {
  return DEFAULT_MCP_CONFIG;
}

export async function executeMcpAction(
  _clientId: string,
  _request: McpRequestMessage,
) {
  throw new Error("MCP is not available in the desktop static build.");
}

export async function getMcpConfigFromFile(): Promise<McpConfigData> {
  return DEFAULT_MCP_CONFIG;
}

export async function isMcpEnabled() {
  return false;
}
