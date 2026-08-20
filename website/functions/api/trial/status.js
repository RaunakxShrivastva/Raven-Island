import { json, normalizeDeviceId, readJson, signEntitlement } from "../_utils.js";

export async function onRequestPost({ request, env }) {
  try {
    const input = await readJson(request);
    const deviceId = normalizeDeviceId(input.deviceId);
    if (!deviceId) return json({ status: "invalid" }, 400);

    const trial = await env.DB.prepare("SELECT * FROM trials WHERE device_id = ?")
      .bind(deviceId)
      .first();

    if (!trial) return json({ status: "trial_missing" }, 404);

    await env.DB.prepare("UPDATE trials SET last_checked_at = datetime('now') WHERE device_id = ?")
      .bind(deviceId)
      .run();

    const active = new Date(trial.expires_at).getTime() > Date.now();
    return json(await signEntitlement(env, {
      status: active ? "trial_active" : "trial_expired",
      deviceId,
      startedAt: trial.started_at,
      expiresAt: trial.expires_at,
      checkedAt: new Date().toISOString(),
    }));
  } catch (error) {
    return json({ status: "error", error: error.message || "Unable to check trial" }, 500);
  }
}
