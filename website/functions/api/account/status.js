import { json, readJson, sha256Hex } from "../_utils.js";

function bearerToken(request) {
  const header = request.headers.get("authorization") || "";
  const match = header.match(/^Bearer\s+(.+)$/i);
  return match ? match[1].trim() : "";
}

function cleanDeviceId(value) {
  return String(value || "").trim().slice(0, 160);
}

async function getTokenUser(env, token) {
  if (!token) return null;
  const tokenHash = await sha256Hex(token);
  const row = await env.DB.prepare(
    `SELECT users.id, users.email, users.name, users.username, users.picture, auth_sessions.expires_at
     FROM auth_sessions
     JOIN users ON users.id = auth_sessions.user_id
     WHERE auth_sessions.token_hash = ?`,
  )
    .bind(tokenHash)
    .first();

  if (!row || new Date(row.expires_at).getTime() < Date.now()) return null;

  await env.DB.prepare("UPDATE auth_sessions SET last_seen_at = datetime('now') WHERE token_hash = ?")
    .bind(tokenHash)
    .run();

  return row;
}

async function activePurchase(env, userId) {
  return env.DB.prepare(
    "SELECT status, plan FROM purchases WHERE user_id = ? AND status = 'active' ORDER BY id DESC LIMIT 1",
  )
    .bind(userId)
    .first();
}

async function registerDevice(env, userId, deviceId) {
  if (!deviceId) return { ok: true };

  const existing = await env.DB.prepare(
    "SELECT device_id FROM account_devices WHERE user_id = ? AND status = 'active' AND device_id != ? ORDER BY activated_at DESC LIMIT 1",
  )
    .bind(userId, deviceId)
    .first();

  if (existing?.device_id) {
    return { ok: false, deviceId: existing.device_id };
  }

  await env.DB.prepare(
    `INSERT INTO account_devices (user_id, device_id, status, last_seen_at)
     VALUES (?, ?, 'active', datetime('now'))
     ON CONFLICT(user_id, device_id) DO UPDATE SET
       status = 'active',
       last_seen_at = datetime('now')`,
  )
    .bind(userId, deviceId)
    .run();

  return { ok: true };
}

export async function onRequestPost({ request, env }) {
  try {
    const token = bearerToken(request);
    const input = await readJson(request);
    const deviceId = cleanDeviceId(input.deviceId);
    const user = await getTokenUser(env, token);

    if (!user) {
      return json({ authenticated: false, status: "unauthenticated" }, 401);
    }

    const purchase = await activePurchase(env, user.id);
    if (!purchase) {
      return json({
        authenticated: true,
        status: "no_purchase",
        purchased: false,
        user: { email: user.email, name: user.name, username: user.username, picture: user.picture },
      });
    }

    const device = await registerDevice(env, user.id, deviceId);
    if (!device.ok) {
      return json({
        authenticated: true,
        status: "device_mismatch",
        purchased: true,
        message: "This Raven account is already active on another device.",
      }, 409);
    }

    return json({
      authenticated: true,
      status: "active",
      purchased: true,
      plan: purchase.plan,
      user: { email: user.email, name: user.name, username: user.username, picture: user.picture },
    });
  } catch (error) {
    return json({ authenticated: false, status: "error", error: error.message || "Unable to check account" }, 500);
  }
}
