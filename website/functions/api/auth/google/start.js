import { json, randomToken, requireEnv } from "../../_utils.js";

const STATE_TTL_MINUTES = 10;

export async function onRequestGet({ request, env }) {
  try {
    requireEnv(env, ["GOOGLE_CLIENT_ID", "GOOGLE_AUTH_REDIRECT_URI"]);

    const url = new URL(request.url);
    const source = url.searchParams.get("source") === "app" ? "app" : "web";
    const state = randomToken(24);
    const expiresAt = new Date(Date.now() + STATE_TTL_MINUTES * 60 * 1000).toISOString();

    await env.DB.prepare(
      "INSERT INTO auth_states (state, source, expires_at) VALUES (?, ?, ?)",
    )
      .bind(state, source, expiresAt)
      .run();

    const google = new URL("https://accounts.google.com/o/oauth2/v2/auth");
    google.searchParams.set("client_id", env.GOOGLE_CLIENT_ID);
    google.searchParams.set("redirect_uri", env.GOOGLE_AUTH_REDIRECT_URI);
    google.searchParams.set("response_type", "code");
    google.searchParams.set("scope", "openid email profile");
    google.searchParams.set("state", state);
    google.searchParams.set("prompt", "select_account");

    return Response.redirect(google.toString(), 302);
  } catch (error) {
    return json({ error: error.message || "Unable to start Google login" }, 500);
  }
}
