import { json, normalizeDeviceId, normalizeLicenseKey, readJson, signEntitlement } from "../_utils.js";

export async function onRequestPost({ request, env }) {
  try {
    const input = await readJson(request);
    const licenseKey = normalizeLicenseKey(input.licenseKey);
    const deviceId = normalizeDeviceId(input.deviceId);

    if (!licenseKey || !deviceId) {
      return json({ status: "invalid", error: "License key and device id are required" }, 400);
    }

    const license = await env.DB.prepare("SELECT * FROM licenses WHERE license_key = ?")
      .bind(licenseKey)
      .first();

    if (!license || license.status !== "active") {
      return json({ status: "invalid" }, 404);
    }

    if (license.device_id && license.device_id !== deviceId) {
      return json({ status: "device_mismatch", message: "This license is already active on another device" }, 409);
    }

    if (!license.device_id) {
      await env.DB.prepare(
        "UPDATE licenses SET device_id = ?, activation_count = activation_count + 1, activated_at = datetime('now'), last_checked_at = datetime('now') WHERE license_key = ?",
      )
        .bind(deviceId, licenseKey)
        .run();
    } else {
      await env.DB.prepare("UPDATE licenses SET last_checked_at = datetime('now') WHERE license_key = ?")
        .bind(licenseKey)
        .run();
    }

    return json(await signEntitlement(env, {
      status: "paid_active",
      licenseKey,
      deviceId,
      checkedAt: new Date().toISOString(),
      offlineGraceDays: 7,
    }));
  } catch (error) {
    return json({ status: "error", error: error.message || "Unable to activate license" }, 500);
  }
}
