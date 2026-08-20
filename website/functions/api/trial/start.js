import { json, normalizeDeviceId, readJson, signEntitlement } from "../_utils.js";

const TRIAL_DAYS = 14;

export async function onRequestPost({ request, env }) {
  try {
    const input = await readJson(request);
    const deviceId = normalizeDeviceId(input.deviceId);
    if (!deviceId) return json({ status: "invalid", error: "Device id is required" }, 400);

    const existing = await env.DB.prepare("SELECT * FROM trials WHERE device_id = ?")
      .bind(deviceId)
      .first();

    if (existing) {
      const active = new Date(existing.expires_at).getTime() > Date.now();
      return json(await signEntitlement(env, {
        status: active ? "trial_active" : "trial_expired",
        deviceId,
        startedAt: existing.started_at,
        expiresAt: existing.expires_at,
        checkedAt: new Date().toISOString(),
      }));
    }

    const expiresAt = new Date(Date.now() + TRIAL_DAYS * 24 * 60 * 60 * 1000).toISOString();
    await env.DB.prepare(
      "INSERT INTO trials (device_id, expires_at, last_checked_at) VALUES (?, ?, datetime('now'))",
    )
      .bind(deviceId, expiresAt)
      .run();

    return json(await signEntitlement(env, {
      status: "trial_active",
      deviceId,
      startedAt: new Date().toISOString(),
      expiresAt,
      checkedAt: new Date().toISOString(),
    }));
  } catch (error) {
    return json({ status: "error", error: error.message || "Unable to start trial" }, 500);
  }
}
