// workerchat.js — Worker DỊCH THUẬT chạy trên CHAT (UNIFY_CHAT_WITH_AI), không dùng CONTENT_TRANSLATOR.
// Lý do: test live cùng 1 câu EN->VI (gpt-4o-mini) cho thấy chat rẻ tương đương
// translator (delta usedCredit ~170 vs ~127 — cùng bậc, đều rẻ), mà chat hơn ở:
// real SSE streaming, multi-turn history/conversationId, 106 model, không bắt 6 field.
// Prompt dịch chuẩn đặt ở system, giữ nguyên multi-turn để câu sau hưởng context câu trước.
// VERIFY live: chat-translate EN->VI SUCCESS, chất lượng tương đương translator.

const CHAT_MODELS_FROM_DOCS = [
  // Alibaba
  "qwen3.7-plus", "qwen3.7-max", "qwen3.7-flash",
  "qwen3.6-plus", "qwen3.6-max-preview", "qwen3.6-flash",
  "qwen3-vl-plus", "qwen3-vl-flash", "qwen3-vl-8b-thinking",
  "qwen3-max", "qwen3-8b",
  "qwen-vl-plus", "qwen-vl-max", "qwen-plus", "qwen-max", "qwen-flash",
  // aws-bedrock
  "us.anthropic.claude-sonnet-5", "us.anthropic.claude-sonnet-4-6",
  "us.anthropic.claude-sonnet-4-5-20250929-v1:0",
  "us.anthropic.claude-opus-5", "us.anthropic.claude-opus-4-8",
  "us.anthropic.claude-opus-4-7", "us.anthropic.claude-opus-4-6-v1",
  "us.anthropic.claude-opus-4-5-20251101-v1:0",
  "us.anthropic.claude-haiku-4-5-20251001-v1:0",
  "us.anthropic.claude-fable-5-1", "us.anthropic.claude-fable-5",
  // Cohere
  "command-r-08-2024",
  // DeepSeek
  "deepseek-v4-pro", "deepseek-v4-flash", "deepseek-reasoner",
  "deepseek-flash", "deepseek-chat",
  // GoogleAI
  "gemini-3.8-flash", "gemini-3.7-flash", "gemini-3.6-flash",
  "gemini-3.5-flash", "gemini-3.1-pro-preview",
  "gemini-3.1-flash-lite-preview", "gemini-3-flash-preview",
  "gemini-2.5-pro", "gemini-2.5-flash",
  // Mistral
  "magistral-small-latest", "magistral-medium-latest",
  "ministral-14b-latest", "open-mistral-nemo",
  "mistral-small-latest", "mistral-small-2603",
  "mistral-medium-latest", "mistral-medium-3-5",
  "mistral-large-latest", "mistral-large-2512",
  // OpenAI
  "gpt-5.3-codex", "o3-mini", "gpt-6-astra",
  "gpt-5.6-terra", "gpt-5.6-sol", "gpt-5.6-luna",
  "gpt-5.5-pro", "gpt-5.5",
  "gpt-5.4-pro", "gpt-5.4-nano", "gpt-5.4-mini", "gpt-5.4",
  "gpt-5.2-pro", "gpt-5.2", "gpt-5.1",
  "gpt-5-nano", "gpt-5-mini", "gpt-5",
  "gpt-4o-mini", "gpt-4o",
  "gpt-4.1-nano", "gpt-4.1-mini", "gpt-4.1",
  "gpt-4-turbo", "gpt-3.5-turbo", "o3-pro", "o3",
  // openrouter
  "moonshotai/kimi-k3", "moonshotai/kimi-k2.7-code", "moonshotai/kimi-k2.6",
  "meta/muse-spark-1.3", "meta/muse-spark-1.2", "meta/muse-spark-1.1",
  // Perplexity
  "sonar-reasoning-pro", "sonar-pro", "sonar-deep-research", "sonar",
  // xAI
  "grok-4.6", "grok-4.5", "grok-4.3",
  "grok-4-fast-reasoning", "grok-4-fast-non-reasoning", "grok-4-0709",
  "grok-3-mini", "grok-3",
  // zai
  "glm-5.3", "glm-5.2", "glm-5.1", "glm-5",
  // Extra
  "meta/meta-llama-3-70b-instruct",
  "meta/llama-4-scout-instruct", "meta/llama-4-maverick-instruct",
  "openai/gpt-oss-20b", "openai/gpt-oss-120b",
];
const seenModels = new Set(CHAT_MODELS_FROM_DOCS);
const DEFAULT_CHAT_MODEL = "gpt-4o-mini";

const DEFAULTS = { originalLanguage: "auto", targetLanguage: "vi" };

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

    // 2. Bearer -> API-KEY
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

    // 3. GET /v1/models — 106 model chat + học runtime
    if (request.method === "GET" && url.pathname === "/v1/models") {
      const now = Math.floor(Date.now() / 1000);
      const ids = [...new Set([...CHAT_MODELS_FROM_DOCS, ...seenModels])];
      return new Response(
        JSON.stringify({
          object: "list",
          data: ids.map((id) => ({ id, object: "model", created: now, owned_by: "1min.ai" })),
        }),
        { status: 200, headers: { "Content-Type": "application/json", ...cors } }
      );
    }

    // 4. GET /languages — mã ISO 639-1 phổ biến (static)
    if (request.method === "GET" && url.pathname === "/languages") {
      return new Response(JSON.stringify({
        object: "list",
        data: [
          { code: "auto", name: "Auto detect" },
          { code: "vi", name: "Vietnamese" }, { code: "en", name: "English" },
          { code: "fr", name: "French" }, { code: "de", name: "German" },
          { code: "es", name: "Spanish" }, { code: "ja", name: "Japanese" },
          { code: "ko", name: "Korean" }, { code: "zh", name: "Chinese" },
          { code: "ru", name: "Russian" }, { code: "th", name: "Thai" },
          { code: "id", name: "Indonesian" }, { code: "ms", name: "Malay" },
          { code: "pt", name: "Portuguese" }, { code: "it", name: "Italian" },
          { code: "nl", name: "Dutch" }, { code: "hi", name: "Hindi" },
          { code: "ar", name: "Arabic" },
        ],
      }), { status: 200, headers: { "Content-Type": "application/json", ...cors } });
    }

    // 5. Passthrough native 1min.ai
    if (url.pathname.startsWith("/api/")) {
      const targetUrl = "https://api.1min.ai" + url.pathname + url.search;
      const body = ["GET", "HEAD"].includes(request.method) ? null : request.body;
      const resp = await fetch(new Request(targetUrl, { method: request.method, headers: headersToForward, body, redirect: "manual" }));
      const h = new Headers(resp.headers);
      h.delete("Content-Encoding"); h.delete("Content-Length");
      h.set("Access-Control-Allow-Origin", "*");
      return new Response(resp.body, { status: resp.status, statusText: resp.statusText, headers: h });
    }

    if (request.method !== "POST") {
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

    // 6. Input dịch từ 2 dạng:
    // (a) Native: POST /translate {text*, targetLanguage=vi, originalLanguage=auto, model?}
    // (b) OpenAI-compat: POST /v1/chat|completions|messages — giữ FULL transcript
    //     (multichat: câu sau hưởng context câu trước), ngôn ngữ qua metadata/extra_body.
    const isNative = url.pathname === "/translate";
    const isChatLike = ["/v1/chat/completions", "/v1/completions", "/v1/messages"].includes(url.pathname);
    if (!isNative && !isChatLike) {
      return new Response(JSON.stringify({ error: { message: "Not found: " + url.pathname + " (use /translate or /v1/chat/completions)", type: "not_found" } }),
        { status: 404, headers: { "Content-Type": "application/json", ...cors } });
    }

    const extra = bodyJson.extra_body ?? {};
    const md = (bodyJson.metadata && typeof bodyJson.metadata === "object") ? bodyJson.metadata : {};
    const model = bodyJson.model || DEFAULT_CHAT_MODEL;
    const pick = (...vals) => {
      for (const v of vals) if (typeof v === "string" && v.trim()) return v.trim();
      return null;
    };
    const originalLanguage = pick(bodyJson.originalLanguage, bodyJson.source, extra.originalLanguage, extra.source, md.originalLanguage, md.source, DEFAULTS.originalLanguage);
    const targetLanguage = pick(bodyJson.targetLanguage, bodyJson.target, extra.targetLanguage, extra.target, md.targetLanguage, md.target, DEFAULTS.targetLanguage);
    // Opt-in websearch (mặc định OFF — dịch không cần): metadata/extra_body {webSearch,numOfSite,maxWord}
    const wantWebSearch = extra.webSearch ?? extra.web_search ?? md.webSearch ?? md.web_search ?? false;
    const numOfSite = clampNum(extra.numOfSite ?? md.numOfSite, 1, 10, 3);
    const maxWord = clampNum(extra.maxWord ?? md.maxWord, 100, 10000, 1000);
    const rawConvId = bodyJson.conversationId ?? md.conversationId ?? null;
    const clientConversationId = isUuid(rawConvId) ? rawConvId : null;

    let transcript = "";
    let imageUrls = [];
    if (isNative) {
      const t = typeof bodyJson.text === "string" ? bodyJson.text : String(bodyJson.text ?? bodyJson.prompt ?? "");
      if (!t.trim()) {
        return new Response(JSON.stringify({ error: { message: "Empty text to translate", type: "invalid_request" } }),
          { status: 400, headers: { "Content-Type": "application/json", ...cors } });
      }
      transcript = t;
    } else if (url.pathname === "/v1/completions") {
      transcript = typeof bodyJson.prompt === "string" ? bodyJson.prompt : String(bodyJson.prompt ?? "");
    } else {
      let msgs = bodyJson.messages || [];
      if (url.pathname === "/v1/messages" && bodyJson.system) {
        msgs = [{ role: "system", content: bodyJson.system }].concat(msgs);
      }
      const built = buildTranscript(msgs);
      transcript = built.text; imageUrls = built.imageUrls;
    }
    if (!transcript.trim()) {
      return new Response(JSON.stringify({ error: { message: "Empty text to translate", type: "invalid_request" } }),
        { status: 400, headers: { "Content-Type": "application/json", ...cors } });
    }

    // 7. Prompt dịch thông dụng bọc ngoài transcript (giữ multi-turn context)
    const srcName = originalLanguage.toLowerCase() === "auto"
      ? "the source language (auto-detect it)"
      : langName(originalLanguage);
    const promptText =
      `You are a professional translator. Translate the user text below from ${srcName} to ${langName(targetLanguage)}. ` +
      `Preserve formatting, line breaks and code blocks. Output ONLY the translation, no explanations.\n\n` +
      `Text to translate:\n${transcript}`;

    const oneMinBody = {
      type: "UNIFY_CHAT_WITH_AI",
      model,
      promptObject: {
        prompt: promptText,
        ...(clientConversationId ? { conversationId: clientConversationId } : {}),
        settings: {
          ...(wantWebSearch === true
            ? { webSearchSettings: { webSearch: true, numOfSite, maxWord } }
            : {}),
          historySettings: { isMixed: false, historyMessageLimit: 10 },
        },
        ...(imageUrls.length > 0 ? { attachments: { images: imageUrls } } : {}),
      },
    };

    const isStream = bodyJson.stream === true && !isNative;
    let upstream;
    try {
      upstream = await fetch(`https://api.1min.ai/api/chat-with-ai${isStream ? "?isStreaming=true" : ""}`, {
        method: "POST",
        headers: headersToForward,
        body: JSON.stringify(oneMinBody),
      });
    } catch (e) {
      return new Response(JSON.stringify({ error: { message: "Upstream fetch failed: " + e.message, type: "upstream_error" } }),
        { status: 502, headers: { "Content-Type": "application/json", ...cors } });
    }

    // 8a. Non-stream (và native /translate)
    if (!isStream) {
      const raw = await upstream.text();
      let rec = null;
      let translated = "";
      try {
        const j = JSON.parse(raw);
        if (!j.aiRecord && (j.errorCode || j.error || j.message)) {
          const msg = j.message || j.error?.message || "1min.ai request failed";
          return new Response(JSON.stringify({ error: { message: msg, type: "upstream_error", code: j.errorCode || "UPSTREAM_ERROR" } }),
            { status: upstream.status >= 400 ? upstream.status : 502, headers: { "Content-Type": "application/json", ...cors } });
        }
        const r = extractText(j);
        if (!r.ok) {
          return new Response(JSON.stringify({ error: { message: r.message, type: "upstream_error", code: r.code } }),
            { status: 502, headers: { "Content-Type": "application/json", ...cors } });
        }
        translated = r.text; rec = r.rec ?? j.aiRecord ?? null;
      } catch {
        translated = raw;
      }
      const realModel = rec?.model || model;
      if (realModel) seenModels.add(realModel);
      const meta = oneMinMeta(rec, model, originalLanguage, targetLanguage);
      if (isNative) {
        return new Response(JSON.stringify({
          translated,
          source: originalLanguage, target: targetLanguage, model: realModel,
          uuid: rec?.uuid ?? null, status: rec?.status ?? "SUCCESS",
          credits: { limit: rec?.teamUser?.creditLimit ?? null, used: rec?.teamUser?.usedCredit ?? null },
        }), { status: upstream.status, headers: { "Content-Type": "application/json", ...cors } });
      }
      const realId = rec?.uuid ? `chatcmpl-${rec.uuid}` : `chatcmpl-${model}-${Date.now()}`;
      const realCreated = toUnix(rec?.createdAt) ?? Math.floor(Date.now() / 1000);
      if (url.pathname === "/v1/messages") {
        return new Response(JSON.stringify({
          id: realId, type: "message", role: "assistant", model: realModel,
          content: [{ type: "text", text: translated }],
          stop_reason: "end_turn", usage: estimateUsage(translated), _one_min: meta,
        }), { status: upstream.status, headers: { "Content-Type": "application/json", ...cors } });
      }
      return new Response(JSON.stringify({
        id: realId, object: "chat.completion", created: realCreated, model: realModel,
        choices: [{ index: 0, message: { role: "assistant", content: translated }, finish_reason: "stop" }],
        usage: estimateUsage(translated),
        _one_min: meta,
      }), { status: upstream.status, headers: { "Content-Type": "application/json", ...cors } });
    }

    // 8b. Stream — chat có SSE thật (event: content|result|done|error), dịch sang OpenAI SSE
    const openAIStream = new ReadableStream({
      async start(controller) {
        const enc = new TextEncoder();
        let chatId = `chatcmpl-${model}-${Date.now()}`;
        let created = Math.floor(Date.now() / 1000);
        let realModel = model;
        let fullText = "";
        let gotContent = false;
        let lastMeta = null;
        const send = (obj) => controller.enqueue(enc.encode(`data: ${JSON.stringify(obj)}\n\n`));
        const headerChunk = (content) => ({
          id: chatId, object: "chat.completion.chunk", created, model: realModel,
          choices: [{ index: 0, delta: { role: "assistant", content }, finish_reason: null }],
        });
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
                if (block.trim() && block.trim() !== "[DONE]") emitContent(block);
                continue;
              }
              if (event === "content") {
                try {
                  const d = JSON.parse(data);
                  if (typeof d.content === "string") emitContent(d.content);
                  else if (typeof d.text === "string") emitContent(d.text);
                  else if (typeof d === "string") emitContent(d);
                } catch { emitContent(data); }
              } else if (event === "result") {
                try {
                  const d = JSON.parse(data);
                  const inner = d.aiRecord ?? d;
                  if (inner?.uuid) chatId = `chatcmpl-${inner.uuid}`;
                  if (inner?.createdAt) { const t = toUnix(inner.createdAt); if (t) created = t; }
                  if (inner?.model) { realModel = inner.model; seenModels.add(realModel); }
                  lastMeta = oneMinMeta(inner, model, originalLanguage, targetLanguage);
                  const r = extractText({ aiRecord: inner });
                  if (r.ok && r.text && !gotContent) emitContent(r.text);
                  else if (r.ok && r.text) fullText = fullText || r.text;
                } catch { /* ignore */ }
              } else if (event === "error") {
                send({ id: chatId, object: "chat.completion.chunk", created, model: realModel, choices: [{ index: 0, delta: { content: "\n[1min.ai error: " + data.slice(0, 300) + "]" }, finish_reason: null }] });
              } else if (!event && data) {
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
      if (typeof p.text === "string") return p.text;
      return "";
    }).join("");
  }
  if (typeof content === "object" && typeof content.text === "string") return content.text;
  return "";
}

// Giữ FULL transcript multichat (câu sau hưởng context câu trước) + gom ảnh http(s).
function buildTranscript(messages) {
  const lines = [];
  const imageUrls = [];
  for (const m of messages) {
    const role = (m.role || "user").toLowerCase();
    const content = m.content ?? m.text ?? "";
    const t = textOf(content).trim();
    if (Array.isArray(content)) {
      for (const p of content) {
        const u = p?.image_url?.url ?? p?.image_url ?? null;
        if (typeof u === "string" && /^https?:\/\//.test(u)) imageUrls.push(u);
      }
    }
    if (!t) continue;
    if (role === "system" || role === "developer") lines.push(`[System] ${t}`);
    else if (role === "assistant") lines.push(`[Assistant] ${t}`);
    else if (role === "tool") lines.push(`[Tool] ${t}`);
    else lines.push(t);
  }
  return { text: lines.join("\n\n"), imageUrls: [...new Set(imageUrls)].slice(0, 5) };
}

function langName(code) {
  const map = {
    auto: "the source language", vi: "Vietnamese", en: "English", fr: "French",
    de: "German", es: "Spanish", ja: "Japanese", ko: "Korean", zh: "Chinese",
    ru: "Russian", th: "Thai", id: "Indonesian", ms: "Malay", pt: "Portuguese",
    it: "Italian", nl: "Dutch", hi: "Hindi", ar: "Arabic",
  };
  return map[String(code).toLowerCase()] || String(code);
}

function extractText(j) {
  const rec = j.aiRecord ?? j;
  if (!rec || rec.aiRecord === null || j.aiRecord === null) {
    return { ok: false, code: "NOT_FOUND", message: "Unknown result id (aiRecord is null)" };
  }
  if (rec.status === "FAILURE") {
    const err = rec.aiRecordDetail?.resultObject;
    const msg = (err && (err.message || err.details)) || "1min.ai processing failed";
    return { ok: false, code: err?.code || "PROCESSING_FAILED", message: `${msg} (traceId: ${err?.traceId || "n/a"})` };
  }
  if (rec.status === "PROCESSING") {
    return { ok: false, code: "STILL_PROCESSING", message: "Result still PROCESSING — retry later" };
  }
  const ro = rec.aiRecordDetail?.resultObject;
  if (Array.isArray(ro)) return { ok: true, text: ro.filter((x) => typeof x === "string").join("\n") || JSON.stringify(ro), rec };
  if (typeof ro === "string") return { ok: true, text: ro, rec };
  const fb = j.response ?? j.message ?? j.result ?? j.text;
  if (typeof fb === "string") return { ok: true, text: fb, rec };
  return { ok: true, text: JSON.stringify(j), rec };
}

function oneMinMeta(rec, requestedModel, src, tgt) {
  if (!rec) return { requestedModel, source: src, target: tgt, note: "no aiRecord (raw fallback)" };
  return {
    uuid: rec.uuid ?? null,
    status: rec.status ?? null,
    type: rec.type ?? "UNIFY_CHAT_WITH_AI",
    requestedModel,
    source: src, target: tgt,
    provider: rec.modelDetail?.provider ?? null,
    conversationId: rec.conversationId ?? null,
    creditLimit: rec.teamUser?.creditLimit ?? null,
    usedCredit: rec.teamUser?.usedCredit ?? null,
  };
}

function toUnix(iso) {
  if (!iso) return null;
  const t = Date.parse(iso);
  return Number.isFinite(t) ? Math.floor(t / 1000) : null;
}

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
