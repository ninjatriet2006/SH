// workerjustwork.js — Passthrough thuần cho api.justwoker.icu (chuẩn OpenAI CŨ).
// justworker KHÔNG phải kiểu 1min (không API-KEY, không /api/*, không aiRecord):
// nó là gateway OpenAI-standard cũ, auth Bearer sk-... giữ nguyên.
// Lỗi 500 "not implemented" là do client mới (opencode/AI-SDK) gửi các param mà
// gateway cũ không implement (stream_options, response_format, reasoning*, ...).
// Fix: passthrough + gọt (denylist) các param mới, giữ nguyên messages/tools/stream.
// KHÔNG parse/đụng vào body stream — forward byte thô nên không bao giờ treo do parser.

const UPSTREAM = "https://api.justwoker.icu";

// Param mới mà gateway chuẩn cũ thường trả "not implemented" — gọt bỏ.
const STRIP_KEYS = [
  "stream_options",
  "response_format",
  "reasoning_effort",
  "reasoning",
  "modalities",
  "audio",
  "prediction",
  "store",
  "metadata",
  "service_tier",
  "prompt_cache_key",
  "safety_identifier",
  "top_logprobs",
  "logprobs",
  "seed",
  "functions",
  "function_call",
  "parallel_tool_calls",
];

export default {
  async fetch(request) {
    const url = new URL(request.url);

    // 1. CORS
    if (request.method === "OPTIONS") {
      return new Response(null, {
        status: 204,
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
          "Access-Control-Allow-Headers": "*",
          "Access-Control-Max-Age": "86400",
        },
      });
    }

    // Chỉ phục vụ /v1
    if (!url.pathname.startsWith("/v1")) {
      return new Response("Not Found", { status: 404 });
    }

    // 2. Header forward + blacklist (giữ nguyên Authorization Bearer)
    const headersToForward = new Headers(request.headers);
    const blacklist = [
      "cf-connecting-ip", "cf-ray", "cf-visitor",
      "x-forwarded-for", "x-forwarded-proto", "x-real-ip",
      "content-length",
      "host", "origin", "referer",
      "sec-ch-ua", "sec-ch-ua-mobile", "sec-ch-ua-platform",
    ];
    for (const header of blacklist) headersToForward.delete(header);
    headersToForward.set("Accept-Encoding", "gzip, deflate, br");
    headersToForward.set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36");
    headersToForward.set("Sec-Ch-Ua", '"Chromium";v="122", "Not(A:Brand";v="24", "Google Chrome";v="122"');
    headersToForward.set("Sec-Ch-Ua-Mobile", "?0");
    headersToForward.set("Sec-Ch-Ua-Platform", '"Linux"');
    headersToForward.delete("originator");
    headersToForward.delete("version");

    // 3. Gọt param mới khỏi JSON body (giữ messages/tools/stream nguyên).
    // Không đụng vào stream bytes — forward thô nên không treo.
    let forwardBody = ["GET", "HEAD"].includes(request.method) ? null : request.body;
    const ctype = request.headers.get("Content-Type") || "";
    if (request.method === "POST" && ctype.includes("application/json")) {
      try {
        const bodyJson = await request.json();
        let stripped = false;
        for (const k of STRIP_KEYS) {
          if (bodyJson[k] !== undefined) {
            delete bodyJson[k];
            stripped = true;
          }
        }
        if (stripped) forwardBody = JSON.stringify(bodyJson);
        else forwardBody = JSON.stringify(bodyJson);
      } catch {
        forwardBody = request.body;
      }
    }

    const targetUrl = UPSTREAM + url.pathname + url.search;

    try {
      const proxyRequest = new Request(targetUrl, {
        method: request.method,
        headers: headersToForward,
        body: forwardBody,
        redirect: "manual",
      });

      const response = await fetch(proxyRequest);
      const responseHeaders = new Headers(response.headers);
      responseHeaders.delete("Content-Encoding");
      responseHeaders.delete("Content-Length");
      responseHeaders.set("Cache-Control", "no-cache, no-transform");
      responseHeaders.set("Access-Control-Allow-Origin", "*");

      return new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers: responseHeaders,
      });
    } catch (error) {
      return new Response(JSON.stringify({ error: error.message }), {
        status: 502,
        headers: { "Content-Type": "application/json" },
      });
    }
  },
};
