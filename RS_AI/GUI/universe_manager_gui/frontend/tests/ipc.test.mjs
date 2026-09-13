import assert from "node:assert/strict";
import test, { after } from "node:test";
import { createServer } from "vite";

const server = await createServer({ server: { middlewareMode: true, hmr: false }, appType: "custom" });
const { JobClient } = await server.ssrLoadModule("/src/ipc.ts");
after(async () => server.close());

function event(state, requestId, seq = 0) {
  return {
    id: 1,
    event: "job.scan_apps",
    payload: {
      schema_version: 1,
      job_id: requestId,
      seq,
      state,
      payload: { request_id: requestId, completed: null, total: null, message: null, result: null },
    },
  };
}

test("JobClient listens before invoke and unlistens once after one terminal", async () => {
  const order = [];
  let handler;
  let unlistenCalls = 0;
  const bridge = {
    async listen(_topic, callback) {
      order.push("listen");
      handler = callback;
      return () => { unlistenCalls += 1; order.push("unlisten"); };
    },
    async invoke(_command, args) {
      order.push("invoke");
      const id = args.request.request_id;
      handler(event("progress", id, 0));
      handler(event("completed", id, 1));
      handler(event("failed", id, 2));
      return { schema_version: 1, request_id: id, data: [] };
    },
  };
  const seen = [];
  const client = new JobClient(bridge);
  await client.run("scan_apps", {}, (value) => seen.push(value.state));
  client.dispose();
  assert.deepEqual(order, ["listen", "invoke", "unlisten"]);
  assert.deepEqual(seen, ["progress", "completed"]);
  assert.equal(unlistenCalls, 1);
});

test("JobClient filters forged identity and non-monotonic sequence", async () => {
  let handler;
  const bridge = {
    async listen(_topic, callback) { handler = callback; return () => {}; },
    async invoke(_command, args) {
      const id = args.request.request_id;
      handler(event("progress", "forged", 0));
      const wrongPayload = event("progress", id, 1);
      wrongPayload.payload.payload.request_id = "forged";
      handler(wrongPayload);
      handler(event("progress", id, 2));
      handler(event("progress", id, 1));
      handler(event("completed", id, 3));
      return { schema_version: 1, request_id: id, data: [] };
    },
  };
  const seen = [];
  await new JobClient(bridge).run("scan_apps", {}, (value) => seen.push(value.seq));
  assert.deepEqual(seen, [2, 3]);
});
