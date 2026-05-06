import {
  McpConfigData,
  McpRequestMessage,
  ServerStatusResponse,
} from "./types";

export async function getClientsStatus(): Promise<
  Record<string, ServerStatusResponse>
> {
  return {};
}

export async function getClientTools(_clientId: string) {
  return null;
}

export async function getAvailableClientsCount() {
  return 0;
}

export async function getAllTools() {
  return [];
}

export async function initializeMcpSystem() {
  return;
}

export async function addMcpServer(_clientId: string, _config: unknown) {
  return;
}

export async function pauseMcpServer(_clientId: string) {
  return;
}

export async function resumeMcpServer(_clientId: string) {
  return;
}

export async function removeMcpServer(_clientId: string) {
  return;
}

export async function restartAllClients() {
  return;
}

export async function executeMcpAction(
  _clientId: string,
  _mcpRequest: McpRequestMessage,
) {
  return null;
}

export async function getMcpConfigFromFile(): Promise<McpConfigData> {
  return { mcpServers: {} };
}

export async function isMcpEnabled() {
  return false;
}
