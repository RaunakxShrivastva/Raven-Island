import { constantTimeEqual, createAuthSession, json, passwordHash, readJson, sessionCookie } from "../_utils.js";

function cleanEmail(value) {
  return String(value || "").trim().toLowerCase().slice(0, 180);
}

export async function onRequestPost({ request, env }) {
  try {
    const input = await readJson(request);
    const email = cleanEmail(input.email);
    const password = String(input.password || "");

    if (!email || !password) {
      return json({ error: "Enter your email and password." }, 400);
    }

    const user = await env.DB.prepare("SELECT id, email, name, username, picture, password_hash, password_salt, google_sub FROM users WHERE email = ?")
      .bind(email)
      .first();
    if (!user || !user.password_hash || !user.password_salt) {
      return json({ error: "Invalid email or password." }, 401);
    }

    const hash = await passwordHash(password, user.password_salt);
    if (!constantTimeEqual(hash, user.password_hash)) {
      return json({ error: "Invalid email or password." }, 401);
    }

    await env.DB.prepare("UPDATE users SET last_login_at = datetime('now') WHERE id = ?")
      .bind(user.id)
      .run();

    const session = await createAuthSession(env, user.id, "web");
    return new Response(JSON.stringify({ authenticated: true, user: { email: user.email, name: user.name, username: user.username, picture: user.picture } }), {
      status: 200,
      headers: {
        "content-type": "application/json; charset=utf-8",
        "cache-control": "no-store",
        "set-cookie": sessionCookie(session.token, session.expiresAt),
      },
    });
  } catch (error) {
    return json({ error: error.message || "Unable to log in" }, 500);
  }
}
