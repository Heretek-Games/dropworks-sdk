import test from "node:test";
import assert from "node:assert/strict";
import { DropworksClient } from "../src/index.js";

function fakeTransport(responses: Record<string, unknown>) {
  return {
    post: async (path: string) => {
      const body = responses[path];
      return {
        ok: body !== undefined,
        status: body !== undefined ? 200 : 404,
        json: async () => body ?? {},
      } as Response;
    },
  };
}

test("DropworksClient signs in and stores the session", async () => {
  const client = new DropworksClient(
    fakeTransport({ "/api/v1/dropworks/session": { userId: "u1" } }),
  );
  const session = await client.signIn("app", "token");
  assert.equal(session.userId, "u1");
  assert.equal(client.currentSession?.appId, "app");
});

test("DropworksClient rejects achievement unlock before sign-in", async () => {
  const client = new DropworksClient(fakeTransport({}));
  await assert.rejects(() => client.unlockAchievement("a1"), /not signed in/);
});
