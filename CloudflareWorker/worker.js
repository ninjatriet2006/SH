// Danh sách model THẬT của CODE_GENERATOR do user cung cấp từ docs.
// VERIFY live bằng key thật:
// - coder model (qwen3-coder-plus) qua CHAT path = 400 UNSUPPORTED_MODEL,
//   qua CODE path (POST /api/features) = SUCCESS.
// - CODE path với ?isStreaming=true trả raw text (không phải SSE events như chat),
//   worker xử lý cả hai dạng.
// Không có endpoint list model (404 cả khi có API-KEY) nên dùng list docs + học runtime.
const CODE_MODELS_FROM_DOCS = [
  // Alibaba
  "qwen3.7-plus", "qwen3.7-max", "qwen3.7-flash",
  "qwen3.6-plus", "qwen3.6-max-preview", "qwen3.6-flash",
  "qwen3-coder-plus", "qwen3-coder-flash",
  // aws-bedrock
  "us.anthropic.claude-sonnet-5", "us.anthropic.claude-sonnet-4-6",
  "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
  "us.anthropic.claude-opus-5", "us.anthropic.claude-opus-4-8",
  "us.anthropic.claude-opus-4-7", "us.anthropic.claude-opus-4-6-v1",
  "us.anthropic.claude-opus-4-5-20251101-v1:0",
  "us.anthropic.claude-haiku-4-5-20251001-v1:0",
  "us.anthropic.claude-fable-5-1", "us.anthropic.claude-fable-5",
  // DeepSeek
  "deepseek-v4-pro", "deepseek-v4-flash", "deepseek-reasoner",
  "deepseek-flash", "deepseek-chat",
  // GoogleAI
  "gemini-3.8-flash", "gemini-3.7-flash", "gemini-3.6-flash",
  "gemini-3.5-flash", "gemini-3.1-pro-preview",
  "gemini-3.1-flash-lite-preview", "gemini-3-flash-preview",
  // Mistral
  "mistral-small-2603", "mistral-medium-3-5", "mistral-large-2512",
  // OpenAI
  "gpt-5.3-codex", "gpt-6-astra",
  "gpt-5.6-terra", "gpt-5.6-sol", "gpt-5.6-luna",
  "gpt-5.5-pro", "gpt-5.5",
  "gpt-5.4-nano", "gpt-5.4-mini", "gpt-5.4", "gpt-5", "gpt-4o", "o3",
  // openrouter
  "moonshotai/kimi-k3", "moonshotai/kimi-k2.7-code", "moonshotai/kimi-k2.6",
  "meta/muse-spark-1.3", "meta/muse-spark-1.2", "meta/muse-spark-1.1",
  // xAI
  "grok-code-fast-1", "grok-4.6", "grok-4.5", "grok-4.3",
  // zai
  "glm-5.3", "glm-5.2", "glm-5.1", "glm-5",
];
const seenModels = new Set(CODE_MODELS_FROM_DOCS);
// Model mặc định cho code (đã verify SUCCESS trên CODE path).
const DEFAULT_CODE_MODEL = "qwen3-coder-plus";

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
    const cors = { "Access-Control-Allow-Origin": "*" };

    // 2. Header forward + Bearer -> API-KEY (1min.ai dùng API-KEY, không dùng Bearer)
    const headersToForward = new Headers(request.headers);
    const blacklist = [
      "cf-connecting-ip", "cf-ray", "cf-visitor",
      "x-forwarded-for", "x-forwarded-proto", "x-real-ip",
      "content-length", "host", "origin", "referer",
      "sec-ch-ua", "sec-ch-ua-mobile", "sec-ch-ua-platform",
    ];
    for (const h of blacklist) headersToForward.delete(h);
    headersToForward.set("Accept-Encoding", "gzip, deflate, br");
    headersToForward.set("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36");
    const authHeader = request.headers.get("Authorization");
    if (authHeader && authHeader.startsWith("Bearer ")) {
      headersToForward.set("API-KEY", authHeader.slice(7).trim());
      headersToForward.delete("Authorization");
    }
    headersToForward.set("Content-Type", "application/json");

    // 3. GET /v1/models — trả list CODE_GENERATOR (worker đã chuyển hẳn sang code).
    if (request.method === "GET" && url.pathname === "/v1/models") {
      const now = Math.floor(Date.now() / 1000);
      const ids = [...new Set([...CODE_MODELS_FROM_DOCS, ...seenModels])];
      return new Response(
        JSON.stringify({
          object: "list",
          data: ids.map((id) => ({ id, object: "model", created: now, owned_by: "1min.ai" })),
        }),
        { status: 200, headers: { "Content-Type": "application/json", ...cors } }
      );
    }

    // 4. Passthrough cho API native 1min.ai
    if (url.pathname.startsWith("/api/")) {
      const targetUrl = "https://api.1min.ai" + url.pathname + url.search;
      const body = ["GET", "HEAD"].includes(request.method) ? null : request.body;
      const resp = await fetch(new Request(targetUrl, { method: request.method, headers: headersToForward, body, redirect: "manual" }));
      const h = new Headers(resp.headers);
      h.delete("Content-Encoding"); h.delete("Content-Length");
      h.set("Access-Control-Allow-Origin", "*");
      return new Response(resp.body, { status: resp.status, statusText: resp.statusText, headers: h });
    }

    // 5. Dịch OpenAI/Anthropic -> 1min.ai
    const isChatLike = request.method === "POST" &&
      ["/v1/chat/completions", "/v1/completions", "/v1/messages", "/v1/responses"].includes(url.pathname);
    if (!isChatLike) {
      return new Response(JSON.stringify({ error: { message: "Not found: " + url.pathname, type: "not_found" } }),
        { status: 404, headers: { "Content-Type": "application/json", ...cors } });
    }

    let bodyJson;
    try {
      bodyJson = await request.json();
    } catch {
      return new Response(JSON.stringify({ error: { message: "Invalid JSON body", type: "invalid_request" } }),
        { status: 400, headers: { "Content-Type": "application/json", ...cors } });
    }

    const isStream = bodyJson.stream === true;
    const requestedModel = bodyJson.model || DEFAULT_CODE_MODEL;

    // --- trích messages ---
    // OpenAI: messages[{role, content: str | [{type:text|image_url}]}]
    // Anthropic: {system, messages} — content blocks [{type:text|image}]
    // Responses: {input, instructions}
    let rawMessages = [];
    let toolsPresent = false;
    if (url.pathname === "/v1/chat/completions") {
      rawMessages = bodyJson.messages || [];
      if (bodyJson.tools || bodyJson.functions || bodyJson.tool_choice) toolsPresent = true;
    } else if (url.pathname === "/v1/completions") {
      const p = typeof bodyJson.prompt === "string" ? bodyJson.prompt : String(bodyJson.prompt ?? "");
      rawMessages = [{ role: "user", content: p }];
    } else if (url.pathname === "/v1/messages") {
      if (bodyJson.system) rawMessages.push({ role: "system", content: bodyJson.system });
      rawMessages = rawMessages.concat(bodyJson.messages || []);
      if (bodyJson.tools || bodyJson.tool_choice) toolsPresent = true;
    } else if (url.pathname === "/v1/responses") {
      if (bodyJson.instructions) rawMessages.push({ role: "system", content: bodyJson.instructions });
      const input = bodyJson.input;
      if (typeof input === "string") rawMessages.push({ role: "user", content: input });
      else if (Array.isArray(input)) rawMessages = rawMessages.concat(input);
      if (bodyJson.tools) toolsPresent = true;
    }

    const promptInfo = buildCodePrompt(rawMessages, toolsPresent);
    const promptText = promptInfo.text;
    if (!promptText.trim()) {
      return new Response(JSON.stringify({ error: { message: "Empty prompt after translation", type: "invalid_request" } }),
        { status: 400, headers: { "Content-Type": "application/json", ...cors } });
    }

    // CODE_GENERATOR (POST /api/features): promptObject PHẲNG, khác hẳn chat.
    // Docs: {prompt* , webSearch?, numOfSite? (1-10, khi webSearch), maxWord? (100-10000, khi webSearch)}.
    // Không có: settings/history/withMemories/brandVoice/attachments. conversationId
    // optional nhưng PHẢI là uuid từ POST /api/conversations {type:CODE_GENERATOR}
    // (tên feature không phải conversationId) — worker chỉ forward khi đúng uuid.
    // Mặc định tối ưu code: webSearch OFF (đỡ credit/latency), one-shot stateless.
    const extra = bodyJson.extra_body ?? {};
    const md = (bodyJson.metadata && typeof bodyJson.metadata === "object") ? bodyJson.metadata : {};
    const wantWebSearch =
      extra.webSearch ?? extra.web_search ?? md.webSearch ?? md.web_search ?? false;
    const numOfSite = clampNum(extra.numOfSite ?? md.numOfSite, 1, 10, 3);
    const maxWord = clampNum(extra.maxWord ?? md.maxWord, 100, 10000, 1000);
    const rawConvId = bodyJson.conversationId ?? md.conversationId ?? null;
    const clientConversationId = isUuid(rawConvId) ? rawConvId : null;
    const oneMinBody = {
      type: "CODE_GENERATOR",
      model: requestedModel,
      ...(clientConversationId ? { conversationId: clientConversationId } : {}),
      promptObject: {
        prompt: promptText,
        webSearch: wantWebSearch === true,
        ...(wantWebSearch === true ? { numOfSite, maxWord } : {}),
      },
    };

    const targetUrl = `https://api.1min.ai/api/features${isStream ? "?isStreaming=true" : ""}`;
    let upstream;
    try {
      upstream = await fetch(targetUrl, {
        method: "POST",
        headers: headersToForward,
        body: JSON.stringify(oneMinBody),
      });
    } catch (e) {
      return new Response(JSON.stringify({ error: { message: "Upstream fetch failed: " + e.message, type: "upstream_error" } }),
        { status: 502, headers: { "Content-Type": "application/json", ...cors } });
    }

    // --- non-stream: 1min {aiRecord:{aiRecordDetail:{resultObject[]},status}} -> OpenAI ---
    // Dùng dữ liệu THẬT từ upstream: uuid, createdAt, model, credit, temporaryUrl.
    // Lưu ý: lỗi như UNSUPPORTED_MODEL trả envelope {errorCode,message} (không có aiRecord).
    if (!isStream) {
      const text = await upstream.text();
      let rec = null;
      let textContent = "";
      try {
        const j = JSON.parse(text);
        if (!j.aiRecord && (j.errorCode || j.error || j.message)) {
          const msg = j.message || j.error?.message || "1min.ai request failed";
          return new Response(JSON.stringify({ error: { message: msg, type: "upstream_error", code: j.errorCode || "UPSTREAM_ERROR" } }),
            { status: upstream.status >= 400 ? upstream.status : 502, headers: { "Content-Type": "application/json", ...cors } });
        }
        rec = j.aiRecord ?? null;
        const r = extractText(j);
        if (r.ok) { textContent = r.text; rec = r.rec ?? rec; }
        else {
          return new Response(JSON.stringify({ error: { message: r.message, type: "upstream_error", code: r.code } }),
            { status: 502, headers: { "Content-Type": "application/json", ...cors } });
        }
      } catch {
        textContent = text; // fallback raw
      }
      const realId = rec?.uuid ? `chatcmpl-${rec.uuid}` : `chatcmpl-${requestedModel}-${Date.now()}`;
      const realCreated = toUnix(rec?.createdAt) ?? Math.floor(Date.now() / 1000);
      const realModel = rec?.model || requestedModel;
      if (realModel) seenModels.add(realModel);
      const meta = oneMinMeta(rec, requestedModel);
      // Anthropic caller (/v1/messages) muốn shape Anthropic
      if (url.pathname === "/v1/messages") {
        return new Response(JSON.stringify({
          id: realId, type: "message", role: "assistant", model: realModel,
          content: [{ type: "text", text: textContent }],
          stop_reason: "end_turn",
          usage: estimateUsage(textContent),
          _one_min: meta,
        }), { status: upstream.status, headers: { "Content-Type": "application/json", ...cors } });
      }
      return new Response(JSON.stringify({
        id: realId, object: "chat.completion", created: realCreated, model: realModel,
        choices: [{ index: 0, message: { role: "assistant", content: textContent }, finish_reason: "stop" }],
        usage: estimateUsage(textContent),
        _one_min: meta,
      }), { status: upstream.status, headers: { "Content-Type": "application/json", ...cors } });
    }

    // --- stream: CODE path trả RAW TEXT (verify live), CHAT path trả SSE events ---
    // Worker xử lý cả hai: nếu upstream không phải text/event-stream thì đọc raw
    // và phát lại thành 1 chunk OpenAI duy nhất. Nếu là SSE thì dịch event.
    // id/created/model lấy THẬT từ event result (aiRecord.uuid/createdAt/model).
    const upstreamCtype = upstream.headers.get("Content-Type") || "";
    if (!upstreamCtype.includes("text/event-stream")) {
      const raw = await upstream.text();
      let textContent = raw;
      let rec = null;
      try {
        const j = JSON.parse(raw);
        if (!j.aiRecord && (j.errorCode || j.error || j.message)) {
          const msg = j.message || j.error?.message || "1min.ai request failed";
          return new Response(JSON.stringify({ error: { message: msg, type: "upstream_error", code: j.errorCode || "UPSTREAM_ERROR" } }),
            { status: upstream.status >= 400 ? upstream.status : 502, headers: { "Content-Type": "application/json", ...cors } });
        }
        const r = extractText(j);
        if (r.ok) { textContent = r.text; rec = r.rec ?? j.aiRecord ?? null; }
      } catch { /* raw không phải JSON — giữ nguyên */ }
      const realId = rec?.uuid ? `chatcmpl-${rec.uuid}` : `chatcmpl-${requestedModel}-${Date.now()}`;
      const realCreated = toUnix(rec?.createdAt) ?? Math.floor(Date.now() / 1000);
      const realModel = rec?.model || requestedModel;
      if (realModel) seenModels.add(realModel);
      const sseBody =
        `data: ${JSON.stringify({ id: realId, object: "chat.completion.chunk", created: realCreated, model: realModel, choices: [{ index: 0, delta: { role: "assistant" }, finish_reason: null }] })}\n\n` +
        `data: ${JSON.stringify({ id: realId, object: "chat.completion.chunk", created: realCreated, model: realModel, choices: [{ index: 0, delta: { content: textContent }, finish_reason: null }] })}\n\n` +
        `data: ${JSON.stringify({ id: realId, object: "chat.completion.chunk", created: realCreated, model: realModel, choices: [{ index: 0, delta: {}, finish_reason: "stop" }] })}\n\n` +
        `data: [DONE]\n\n`;
      return new Response(sseBody, {
        status: 200,
        headers: { "Content-Type": "text/event-stream", "Cache-Control": "no-cache, no-transform", "Connection": "keep-alive", ...cors },
      });
    }
    const openAIStream = new ReadableStream({
      async start(controller) {
        const enc = new TextEncoder();
        let chatId = `chatcmpl-${requestedModel}-${Date.now()}`;
        let created = Math.floor(Date.now() / 1000);
        let realModel = requestedModel;
        let fullText = "";
        let gotContent = false;
        let lastMeta = null;
        const send = (obj) => controller.enqueue(enc.encode(`data: ${JSON.stringify(obj)}\n\n`));
        const headerChunk = (content) => ({
          id: chatId, object: "chat.completion.chunk", created, model: realModel,
          choices: [{ index: 0, delta: { role: "assistant", content }, finish_reason: null }],
        });
        // role đầu tiên để opencode/AI-SDK nhận diện
        send({ id: chatId, object: "chat.completion.chunk", created, model: realModel, choices: [{ index: 0, delta: { role: "assistant" }, finish_reason: null }] });

        const reader = upstream.body.getReader();
        const dec = new TextDecoder();
        let buf = "";
        const emitContent = (t) => { if (t) { gotContent = true; fullText += t; send(headerChunk(t)); } };
        try {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            buf += dec.decode(value, { stream: true });
            let idx;
            while ((idx = buf.indexOf("\n\n")) !== -1) {
              const block = buf.slice(0, idx);
              buf = buf.slice(idx + 2);
              const { event, data } = parseSSEBlock(block);
              if (!data) {
                // CODE path có thể trả raw text không theo khung SSE — phát nguyên block.
                if (block.trim() && block.trim() !== "[DONE]") emitContent(block);
                continue;
              }
              if (event === "content") {
                try {
                  const d = JSON.parse(data);
                  if (typeof d.content === "string") emitContent(d.content);
                  else if (typeof d.text === "string") emitContent(d.text);
                  else if (typeof d === "string") emitContent(d);
                } catch { emitContent(data); } // raw text fallback
              } else if (event === "result") {
                try {
                  const d = JSON.parse(data);
                  const inner = d.aiRecord ?? d;
                  if (inner?.uuid) chatId = `chatcmpl-${inner.uuid}`;
                  if (inner?.createdAt) { const t = toUnix(inner.createdAt); if (t) created = t; }
                  if (inner?.model) { realModel = inner.model; seenModels.add(realModel); }
                  lastMeta = oneMinMeta(inner, requestedModel);
                  const r = extractText({ aiRecord: inner });
                  if (r.ok && r.text && !gotContent) emitContent(r.text); // fallback thật
                  else if (r.ok && r.text) fullText = fullText || r.text;
                } catch { /* ignore */ }
              } else if (event === "done") {
                // sẽ đóng bên dưới
              } else if (event === "error") {
                send({ id: chatId, object: "chat.completion.chunk", created, model: realModel, choices: [{ index: 0, delta: { content: "\n[1min.ai error: " + data.slice(0, 300) + "]" }, finish_reason: null }] });
              } else if (!event && data) {
                // block chỉ có data: (tương thích raw stream)
                if (data === "[DONE]") continue;
                try {
                  const d = JSON.parse(data);
                  const c = d.choices?.[0]?.delta?.content ?? d.content ?? d.text;
                  if (typeof c === "string") emitContent(c);
                } catch { emitContent(data); }
              }
            }
          }
        } catch (e) {
          send(headerChunk("\n[stream error: " + e.message + "]"));
        }
        send({ id: chatId, object: "chat.completion.chunk", created, model: realModel, choices: [{ index: 0, delta: { _one_min: lastMeta, _full_text_length: fullText.length }, finish_reason: "stop" }] });
        controller.enqueue(enc.encode("data: [DONE]\n\n"));
        controller.close();
      },
    });

    return new Response(openAIStream, {
      status: 200,
      headers: {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache, no-transform",
        "Connection": "keep-alive",
        ...cors,
      },
    });
  },
};

// ---- helpers ----

function textOf(content) {
  if (content == null) return "";
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content.map((p) => {
      if (typeof p === "string") return p;
      if (!p || typeof p !== "object") return "";
      if (typeof p.text === "string") return p.text;                    // OpenAI + Anthropic text block
      if (p.type === "text" && typeof p.text === "string") return p.text;
      if (p.type === "input_text" && typeof p.text === "string") return p.text;
      if (p.type === "output_text" && typeof p.text === "string") return p.text;
      return "";
    }).join("");
  }
  if (typeof content === "object" && typeof content.text === "string") return content.text;
  return "";
}

// CODE_GENERATOR chỉ nhận text thuần — không có attachments/images như chat.
// Ảnh (nếu client gửi) bị bỏ qua có ghi chú để không mất ngữ cảnh im lặng.
function buildCodePrompt(messages, toolsPresent) {
  const lines = [];
  let droppedImages = 0;
  for (const m of messages) {
    const role = (m.role || "user").toLowerCase();
    const content = m.content ?? m.text ?? "";
    const t = textOf(content).trim();
    if (Array.isArray(content)) {
      for (const p of content) {
        if (p && typeof p === "object" && (p.image_url || p.type === "image_url" || p.type === "image")) droppedImages++;
      }
    }
    if (!t) continue;
    if (role === "system" || role === "developer" || role === "instructions") lines.push(`[System] ${t}`);
    else if (role === "assistant") lines.push(`[Assistant] ${t}`);
    else if (role === "tool") lines.push(`[Tool] ${t}`);
    else lines.push(t); // user: giữ nguyên, bảo toàn code fence
  }
  if (droppedImages > 0) {
    lines.push(`[Note] ${droppedImages} image(s) omitted: CODE_GENERATOR does not accept image attachments.`);
  }
  if (toolsPresent) {
    lines.push("[Note] Client requested function/tool calling, which 1min.ai does not support. Answer with code in plain text only.");
  }
  return { text: lines.join("\n\n") };
}

// 1min trả HTTP 200 cả khi FAILURE / aiRecord:null — phải check status field
function extractText(j) {
  const rec = j.aiRecord ?? j;
  if (!rec || rec.aiRecord === null || j.aiRecord === null) {
    return { ok: false, code: "NOT_FOUND", message: "Unknown result id (aiRecord is null — wrong id or other team)" };
  }
  const status = rec.status;
  if (status === "FAILURE") {
    const err = rec.aiRecordDetail?.resultObject;
    const msg = (err && (err.message || err.details)) || "1min.ai processing failed";
    return { ok: false, code: err?.code || "PROCESSING_FAILED", message: `${msg} (traceId: ${err?.traceId || "n/a"})` };
  }
  if (status === "PROCESSING") {
    return { ok: false, code: "STILL_PROCESSING", message: "Result still PROCESSING — retry GET /api/results/:uuid later" };
  }
  const ro = rec.aiRecordDetail?.resultObject;
  if (Array.isArray(ro)) return { ok: true, text: ro.filter((x) => typeof x === "string").join("\n") || JSON.stringify(ro), rec };
  if (typeof ro === "string") return { ok: true, text: ro, rec };
  // fallback các envelope cũ
  const fb = j.response ?? j.message ?? j.result ?? j.text;
  if (typeof fb === "string") return { ok: true, text: fb, rec };
  return { ok: true, text: JSON.stringify(j), rec };
}

// Giữ lại metadata THẬT của 1min (credit, provider, temporaryUrl) thay vì vứt bỏ.
function oneMinMeta(rec, requestedModel) {
  if (!rec) return { requestedModel, note: "no aiRecord (raw fallback)" };
  return {
    uuid: rec.uuid ?? null,
    status: rec.status ?? null,
    type: rec.type ?? "CODE_GENERATOR",
    requestedModel,
    provider: rec.modelDetail?.provider ?? null,
    conversationId: rec.conversationId ?? null,
    creditLimit: rec.teamUser?.creditLimit ?? null,
    usedCredit: rec.teamUser?.usedCredit ?? null,
    temporaryUrl: rec.temporaryUrl ?? null,
  };
}

function toUnix(iso) {
  if (!iso) return null;
  const t = Date.parse(iso);
  return Number.isFinite(t) ? Math.floor(t / 1000) : null;
}

// 1min không trả token usage — ước lượng minh bạch từ độ dài text (không giả số 0).
function estimateUsage(text) {
  const chars = text ? [...text].length : 0;
  const completion_tokens = Math.ceil(chars / 4);
  return { prompt_tokens: null, completion_tokens, total_tokens: completion_tokens, _estimated: true, _basis: "chars/4" };
}

function isUuid(v) {
  return typeof v === "string" &&
    /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(v.trim());
}

function parseSSEBlock(block) {
  let event = "";
  const datas = [];
  for (const line of block.split("\n")) {
    if (line.startsWith("event:")) event = line.slice(6).trim();
    else if (line.startsWith("data:")) datas.push(line.slice(5).trimStart());
  }
  return { event, data: datas.join("\n") };
}

function clampNum(v, min, max, fallback) {
  const n = typeof v === "string" && v.trim() !== "" ? Number(v) : v;
  if (typeof n !== "number" || !Number.isFinite(n)) return fallback;
  return Math.min(max, Math.max(min, Math.floor(n)));
}
