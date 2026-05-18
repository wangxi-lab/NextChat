import { getServerSideConfig } from "@/app/config/server";
import {
  ApiPath,
  GOOSE_BASE_URL,
  ModelProvider,
  ServiceProvider,
} from "@/app/constant";
import { auth } from "@/app/api/auth";
import { isModelNotavailableInServer } from "@/app/utils/model";
import { prettyObject } from "@/app/utils/format";
import { NextRequest, NextResponse } from "next/server";

const serverConfig = getServerSideConfig();

export async function handle(
  req: NextRequest,
  { params }: { params: { path: string[] } },
) {
  console.log("[Goose Route] params ", params);

  if (req.method === "OPTIONS") {
    return NextResponse.json({ body: "OK" }, { status: 200 });
  }

  const authResult = auth(req, ModelProvider.Goose);
  if (authResult.error) {
    return NextResponse.json(authResult, {
      status: 401,
    });
  }

  try {
    return await request(req);
  } catch (e) {
    console.error("[Goose] ", e);
    return NextResponse.json(prettyObject(e));
  }
}

async function request(req: NextRequest) {
  const controller = new AbortController();
  const path = `${req.nextUrl.pathname}`.replaceAll(ApiPath.Goose, "");
  let baseUrl = serverConfig.gooseUrl || GOOSE_BASE_URL;

  if (!baseUrl.startsWith("http")) {
    baseUrl = `http://${baseUrl}`;
  }

  if (baseUrl.endsWith("/")) {
    baseUrl = baseUrl.slice(0, -1);
  }

  const timeoutId = setTimeout(() => controller.abort(), 10 * 60 * 1000);

  const fetchUrl = `${baseUrl}${path}`;
  const authHeader =
    req.headers.get("Authorization") ||
    (serverConfig.gooseApiKey
      ? `Bearer ${serverConfig.gooseApiKey}`
      : undefined);
  const fetchOptions: RequestInit = {
    headers: {
      "Content-Type": "application/json",
      ...(authHeader ? { Authorization: authHeader } : {}),
    },
    method: req.method,
    body: req.body,
    redirect: "manual",
    // @ts-ignore
    duplex: "half",
    signal: controller.signal,
  };

  if (serverConfig.customModels && req.body) {
    try {
      const clonedBody = await req.text();
      fetchOptions.body = clonedBody;
      const jsonBody = JSON.parse(clonedBody) as { model?: string };
      if (
        isModelNotavailableInServer(
          serverConfig.customModels,
          jsonBody?.model as string,
          ServiceProvider.Goose as string,
        )
      ) {
        return NextResponse.json(
          {
            error: true,
            message: `you are not allowed to use ${jsonBody?.model} model`,
          },
          { status: 403 },
        );
      }
    } catch (e) {
      console.error("[Goose] filter", e);
    }
  }

  try {
    const res = await fetch(fetchUrl, fetchOptions);
    const newHeaders = new Headers(res.headers);
    newHeaders.delete("www-authenticate");
    newHeaders.set("X-Accel-Buffering", "no");
    return new Response(res.body, {
      status: res.status,
      statusText: res.statusText,
      headers: newHeaders,
    });
  } finally {
    clearTimeout(timeoutId);
  }
}
