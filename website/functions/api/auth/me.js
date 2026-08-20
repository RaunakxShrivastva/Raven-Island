import { getSessionUser, json } from "../_utils.js";

export async function onRequestGet({ request, env }) {
  try {
    const user = await getSessionUser(env, request);
    if (!user) {
      return json({ authenticated: false });
    }

    const purchase = await env.DB.prepare(
      "SELECT status, plan FROM purchases WHERE user_id = ? AND status = 'active' ORDER BY id DESC LIMIT 1",
    )
      .bind(user.id)
      .first();
    let username = null;
    try {
      const profile = await env.DB.prepare("SELECT username FROM users WHERE id = ?")
        .bind(user.id)
        .first();
      username = profile?.username || null;
    } catch {}

    return json({
      authenticated: true,
      user: {
        email: user.email,
        name: user.name,
        username,
        picture: user.picture,
      },
      purchase: purchase || null,
    });
  } catch (error) {
    return json({ authenticated: false, error: error.message || "Unable to read session" }, 500);
  }
}
