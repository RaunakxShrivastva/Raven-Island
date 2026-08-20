import { getCookie, json, sha256Hex } from "../_utils.js";

export async function onRequestPost({ request, env }) {
  try {
    const token = getCookie(request, "raven_session");
    if (token) {
      const tokenHash = await sha256Hex(token);
      await env.DB.prepare("DELETE FROM auth_sessions WHERE token_hash = ?")
        .bind(tokenHash)
        .run();
    }

    return new Response(JSON.stringify({ ok: true }), {
      status: 200,
      headers: {
        "content-type": "application/json; charset=utf-8",
        "cache-control": "no-store",
        "set-cookie": "raven_session=; Path=/; HttpOnly; Secure; SameSite=Lax; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
      },
    });
  } catch (error) {
    return json({ error: error.message || "Unable to log out" }, 500);
  }
}
