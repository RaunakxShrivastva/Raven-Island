import { json, normalizeDeviceId, normalizeLicenseKey, readJson, signEntitlement } from "../_utils.js";

export async function onRequestPost({ request, env }) {
  try {
    const input = await readJson(request);
    const licenseKey = normalizeLicenseKey(input.licenseKey);
    const deviceId = normalizeDeviceId(input.deviceId);

    if (!licenseKey || !deviceId) {
      return json({ status: "invalid" }, 400);
    }

    const license = await env.DB.prepare("SELECT * FROM licenses WHERE license_key = ?")
      .bind(licenseKey)
      .first();

    if (!license || license.status !== "active") {
      return json({ status: "invalid" }, 404);
    }

    if (license.device_id !== deviceId) {
      return json({ status: "device_mismatch" }, 409);
    }

    await env.DB.prepare("UPDATE licenses SET last_checked_at = datetime('now') WHERE license_key = ?")
      .bind(licenseKey)
      .run();

    return json(await signEntitlement(env, {
      status: "paid_active",
      licenseKey,
      deviceId,
      checkedAt: new Date().toISOString(),
      offlineGraceDays: 7,
    }));
  } catch (error) {
    return json({ status: "error", error: error.message || "Unable to check license" }, 500);
  }
}
