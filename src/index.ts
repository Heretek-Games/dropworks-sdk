/**
 * Dropworks SDK — a lightweight Steamworks-replacement surface for native games.
 *
 * This TypeScript package is the reference client. Language bindings under
 * `bindings/` wrap the same HTTP/WebSocket contract exposed by a Drop server:
 *   POST /api/v1/dropworks/session
 *   POST /api/v1/dropworks/achievement
 *   WS   /api/v1/dropworks/presence
 */
export interface DropworksSession {
  appId: string;
  userId: string;
  authToken: string;
}

export interface DropworksTransport {
  post(path: string, body: unknown): Promise<Response>;
}

export class DropworksClient {
  constructor(
    private readonly transport: DropworksTransport,
    private session: DropworksSession | null = null,
  ) {}

  async signIn(appId: string, authToken: string): Promise<DropworksSession> {
    const response = await this.transport.post("/api/v1/dropworks/session", { appId, authToken });
    if (!response.ok) throw new Error(`Dropworks sign-in failed: ${response.status}`);
    const payload = (await response.json()) as Partial<DropworksSession>;
    if (!payload.userId) throw new Error("Dropworks sign-in response missing userId");
    this.session = {
      appId,
      userId: payload.userId,
      authToken,
    };
    return this.session;
  }

  async unlockAchievement(achievementId: string): Promise<boolean> {
    if (!this.session) throw new Error("Dropworks client is not signed in");
    const response = await this.transport.post("/api/v1/dropworks/achievement", {
      appId: this.session.appId,
      userId: this.session.userId,
      achievementId,
    });
    return response.ok;
  }

  get currentSession(): DropworksSession | null {
    return this.session;
  }
}
