export type GooseStatus = {
  available: boolean;
  path?: string;
  version?: string;
  error?: string;
};

export type GooseResponse = {
  content: string;
  path: string;
};

export function isGooseSupported() {
  return !!window.__TAURI__?.invoke;
}

export async function getGooseStatus(): Promise<GooseStatus> {
  if (!isGooseSupported()) {
    return {
      available: false,
      error: "Goose is only available in the desktop app.",
    };
  }

  return window.__TAURI__!.invoke("goose_status");
}

export async function startGooseAgent(): Promise<string | null> {
  if (!isGooseSupported()) {
    return null;
  }

  return window.__TAURI__!.invoke("start_goose_agent");
}

export async function askGoose(prompt: string): Promise<GooseResponse> {
  if (!isGooseSupported()) {
    throw new Error("Goose is only available in the desktop app.");
  }

  return window.__TAURI__!.invoke("goose_chat", { prompt });
}
