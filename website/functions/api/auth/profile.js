import { getSessionUser, json, passwordHash, randomToken, readJson } from "../_utils.js";

function cleanText(value, max = 120) {
  return String(value || "").trim().slice(0, max);
}

function cleanUsername(value) {
  return cleanText(value, 24).toLowerCase();
}

function isValidUsername(value) {
  return !value || /^[a-z0-9._-]{3,24}$/.test(value);
}

function cleanPicture(value) {
  const text = cleanText(value, 260000);
  if (!text) return "";
  try {
    const url = new URL(text);
    return ["http:", "https:", "data:"].includes(url.protocol) ? text : "";
  } catch {
    return "";
  }
}

function isMissingProfileColumns(error) {
  const message = String(error?.message || "");
  return message.includes("no such column: username")
    || message.includes("no such column: password_hash")
    || message.includes("no such column: password_salt");
}

export async function onRequestPost({ request, env }) {
  try {
    const user = await getSessionUser(env, request);
    if (!user) return json({ error: "Please log in first." }, 401);

    const input = await readJson(request);
    const name = cleanText(input.name, 80);
    const username = cleanUsername(input.username);
    const picture = cleanPicture(input.picture);
    const password = String(input.password || "");

    if (!name) return json({ error: "Enter your name." }, 400);
    if (!isValidUsername(username)) {
      return json({ error: "Username must be 3-24 characters: lowercase letters, numbers, dot, underscore, or hyphen." }, 400);
    }
    if (password && password.length < 6) {
      return json({ error: "Password must be at least 6 characters." }, 400);
    }

    if (username) {
      const taken = await env.DB.prepare("SELECT id FROM users WHERE username = ? AND id != ?")
        .bind(username, user.id)
        .first();
      if (taken) return json({ error: "Username already taken. Try a different one." }, 409);
    }

    if (password) {
      const salt = randomToken(18);
      const hash = await passwordHash(password, salt);
      await env.DB.prepare(
        "UPDATE users SET name = ?, username = ?, picture = COALESCE(NULLIF(?, ''), picture), password_hash = ?, password_salt = ? WHERE id = ?",
      )
        .bind(name, username || null, picture, hash, salt, user.id)
        .run();
    } else {
      await env.DB.prepare(
        "UPDATE users SET name = ?, username = ?, picture = COALESCE(NULLIF(?, ''), picture) WHERE id = ?",
      )
        .bind(name, username || null, picture, user.id)
        .run();
    }

    const updated = await env.DB.prepare("SELECT email, name, username, picture FROM users WHERE id = ?")
      .bind(user.id)
      .first();

    return json({ authenticated: true, user: updated });
  } catch (error) {
    if (isMissingProfileColumns(error)) {
      return json({ error: "Profile database columns are missing. Run the latest D1 users-table migration first." }, 500);
    }
    return json({ error: error.message || "Unable to update profile" }, 500);
  }
}
