import { constantTimeEqual, getSessionUser, hmacHex, json, makeLicenseKey, readJson, sendLicenseEmail } from "./_utils.js";

async function resolvePurchaseUser(env, request, email, orderId) {
  const sessionUser = await getSessionUser(env, request);
  if (sessionUser?.id) return sessionUser;

  const normalizedEmail = String(email || "").trim().toLowerCase();
  if (normalizedEmail) {
    const user = await env.DB.prepare("SELECT id, email, name, picture FROM users WHERE email = ?")
      .bind(normalizedEmail)
      .first();
    if (user) return user;
  }

  const payment = await env.DB.prepare("SELECT customer_email FROM payments WHERE razorpay_order_id = ?")
    .bind(orderId)
    .first();
  if (payment?.customer_email) {
    const user = await env.DB.prepare("SELECT id, email, name, picture FROM users WHERE email = ?")
      .bind(String(payment.customer_email).trim().toLowerCase())
      .first();
    if (user) return user;
  }

  return null;
}

async function attachPurchase(env, userId, orderId, paymentId) {
  if (!userId) return false;
  await env.DB.prepare(
    `INSERT INTO purchases (user_id, razorpay_payment_id, razorpay_order_id, status, plan)
     VALUES (?, ?, ?, 'active', 'lifetime')
     ON CONFLICT(razorpay_payment_id) DO UPDATE SET
       user_id = excluded.user_id,
       status = 'active',
       plan = 'lifetime'`,
  )
    .bind(userId, paymentId, orderId)
    .run();
  return true;
}

export async function onRequestPost({ request, env }) {
  try {
    const input = await readJson(request);
    const orderId = String(input.razorpay_order_id || "");
    const paymentId = String(input.razorpay_payment_id || "");
    const signature = String(input.razorpay_signature || "");
    const email = String(input.email || "").trim().slice(0, 180);
    const purchaseUser = await resolvePurchaseUser(env, request, email, orderId);

    if (!orderId || !paymentId || !signature) {
      return json({ error: "Missing Razorpay payment details" }, 400);
    }

    const expected = await hmacHex(env.RAZORPAY_KEY_SECRET, `${orderId}|${paymentId}`);
    if (!constantTimeEqual(expected, signature)) {
      return json({ error: "Payment signature verification failed" }, 400);
    }

    const existing = await env.DB.prepare(
      "SELECT license_key FROM licenses WHERE razorpay_payment_id = ?",
    )
      .bind(paymentId)
      .first();

    if (existing?.license_key) {
      const purchaseAttached = await attachPurchase(env, purchaseUser?.id, orderId, paymentId);
      const emailResult = await sendLicenseEmail(env, { email, licenseKey: existing.license_key });
      return json({ licenseKey: existing.license_key, reused: true, emailSent: emailResult.sent, purchaseAttached });
    }

    let licenseKey = makeLicenseKey();
    for (let i = 0; i < 4; i += 1) {
      const row = await env.DB.prepare("SELECT id FROM licenses WHERE license_key = ?")
        .bind(licenseKey)
        .first();
      if (!row) break;
      licenseKey = makeLicenseKey();
    }

    await env.DB.batch([
      env.DB.prepare(
        "UPDATE payments SET status = ?, razorpay_payment_id = ?, customer_email = COALESCE(?, customer_email), paid_at = datetime('now') WHERE razorpay_order_id = ?",
      ).bind("paid", paymentId, email || null, orderId),
      env.DB.prepare(
        "INSERT INTO licenses (license_key, razorpay_order_id, razorpay_payment_id, customer_email, status) VALUES (?, ?, ?, ?, ?)",
      ).bind(licenseKey, orderId, paymentId, email || null, "active"),
    ]);
    const purchaseAttached = await attachPurchase(env, purchaseUser?.id, orderId, paymentId);

    const emailResult = await sendLicenseEmail(env, { email, licenseKey });
    return json({ licenseKey, emailSent: emailResult.sent, purchaseAttached });
  } catch (error) {
    return json({ error: error.message || "Unable to verify payment" }, 500);
  }
}
