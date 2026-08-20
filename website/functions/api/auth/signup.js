import { createAuthSession, hmacHex, json, passwordHash, randomToken, readJson, sessionCookie } from "../_utils.js";

function cleanEmail(value) {
  return String(value || "").trim().toLowerCase().slice(0, 180);
}

function cleanText(value, max = 80) {
  return String(value || "").trim().slice(0, max);
}

function isValidUsername(value) {
  return /^[a-z0-9._-]{3,24}$/.test(value);
}

export async function onRequestPost({ request, env }) {
  try {
    const input = await readJson(request);
    const name = cleanText(input.name);
    const username = cleanText(input.username, 24).toLowerCase();
    const email = cleanEmail(input.email);
    const password = String(input.password || "");

    if (!name || !username || !email || password.length < 6) {
      return json({ error: "Enter name, username, email, and a password with at least 6 characters." }, 400);
    }
    if (!isValidUsername(username)) {
      return json({ error: "Username must be 3-24 characters: lowercase letters, numbers, dot, underscore, or hyphen." }, 400);
    }

    const existingEmail = await env.DB.prepare("SELECT id, password_hash, google_sub FROM users WHERE email = ?")
      .bind(email)
      .first();
    if (existingEmail) {
      if (!existingEmail.password_hash && existingEmail.google_sub && !existingEmail.google_sub.startsWith("email:")) {
        return json({ error: "This email is registered via Google Login. Please log in using 'Continue with Google', then set a password in your profile settings." }, 409);
      }
      return json({ error: "An account with this email already exists. Please log in." }, 409);
    }

    const existingUsername = await env.DB.prepare("SELECT id FROM users WHERE username = ?")
      .bind(username)
      .first();
    if (existingUsername) {
      return json({ error: "Username already taken. Try a different one." }, 409);
    }

    const salt = randomToken(18);
    const hash = await passwordHash(password, salt);
    const emailSub = `email:${await hmacHex("raven-email-user", email)}`;

    await env.DB.prepare(
      `INSERT INTO users (google_sub, email, username, name, password_hash, password_salt, last_login_at)
       VALUES (?, ?, ?, ?, ?, ?, datetime('now'))`,
    )
      .bind(emailSub, email, username, name, hash, salt)
      .run();

    const user = await env.DB.prepare("SELECT id, email, name, username, picture FROM users WHERE email = ?")
      .bind(email)
      .first();
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
    return json({ error: error.message || "Unable to create account" }, 500);
  }
}
