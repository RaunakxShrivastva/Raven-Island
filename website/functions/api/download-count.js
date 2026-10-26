import { json, readJson, sha256Hex } from "./_utils.js";

async function getDownloadTotal(env) {
  const row = await env.DB.prepare("SELECT COUNT(*) AS total FROM download_visitors").first();
  return Number(row?.total || 0);
}

export async function onRequestGet({ env }) {
  try {
    return json({ total: await getDownloadTotal(env) });
  } catch {
    return json({ error: "Unable to load download count" }, 500);
  }
}

export async function onRequestPost({ request, env }) {
  try {
    const input = await readJson(request);
    const visitorId = String(input.visitorId || "").trim();
    if (!/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(visitorId)) {
      return json({ error: "Invalid visitor identifier" }, 400);
    }

    const visitorHash = await sha256Hex(visitorId);
    await env.DB.prepare("INSERT OR IGNORE INTO download_visitors (visitor_hash) VALUES (?)")
      .bind(visitorHash)
      .run();

    return json({ total: await getDownloadTotal(env) });
  } catch {
    return json({ error: "Unable to record download click" }, 500);
  }
}