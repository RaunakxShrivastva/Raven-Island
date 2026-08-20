import { fetchRazorpayPayment, json, makeLicenseKey, PRODUCT_AMOUNT, PRODUCT_CURRENCY, readJson, sendLicenseEmail } from "../_utils.js";

export async function onRequestPost({ request, env }) {
  try {
    const supportSecret = request.headers.get("x-app-api-secret") || "";
    if (!env.APP_API_SECRET || supportSecret !== env.APP_API_SECRET) {
      return json({ error: "Unauthorized" }, 401);
    }

    const input = await readJson(request);
    const paymentId = String(input.razorpay_payment_id || input.paymentId || "").trim();
    const orderId = String(input.razorpay_order_id || input.orderId || "").trim();
    const email = String(input.email || "").trim().slice(0, 180);

    if (!paymentId) {
      return json({ error: "Payment id is required" }, 400);
    }

    const existing = await env.DB.prepare(
      "SELECT license_key FROM licenses WHERE razorpay_payment_id = ?",
    )
      .bind(paymentId)
      .first();

    if (existing?.license_key) {
      const emailResult = await sendLicenseEmail(env, { email, licenseKey: existing.license_key });
      return json({ licenseKey: existing.license_key, reused: true, emailSent: emailResult.sent });
    }

    const payment = await fetchRazorpayPayment(env, paymentId);
    if (orderId && payment.order_id !== orderId) {
      return json({ error: "Payment id does not match this order id" }, 400);
    }
    if (!payment.order_id) {
      return json({ error: "This payment is not linked to a Razorpay order" }, 400);
    }
    if (!["captured", "authorized"].includes(payment.status)) {
      return json({ error: `Payment is not successful yet: ${payment.status || "unknown"}` }, 400);
    }
    if (Number(payment.amount) !== PRODUCT_AMOUNT || payment.currency !== PRODUCT_CURRENCY) {
      return json({ error: "Payment amount or currency does not match this product" }, 400);
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
        "INSERT OR IGNORE INTO payments (razorpay_order_id, razorpay_payment_id, amount, currency, status, customer_email, paid_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
      ).bind(payment.order_id, paymentId, PRODUCT_AMOUNT, PRODUCT_CURRENCY, payment.status, email || payment.email || null),
      env.DB.prepare(
        "UPDATE payments SET status = ?, razorpay_payment_id = ?, customer_email = COALESCE(?, customer_email), paid_at = COALESCE(paid_at, datetime('now')) WHERE razorpay_order_id = ?",
      ).bind(payment.status, paymentId, email || payment.email || null, payment.order_id),
      env.DB.prepare(
        "INSERT INTO licenses (license_key, razorpay_order_id, razorpay_payment_id, customer_email, status) VALUES (?, ?, ?, ?, ?)",
      ).bind(licenseKey, payment.order_id, paymentId, email || payment.email || null, "active"),
    ]);

    const emailResult = await sendLicenseEmail(env, { email: email || payment.email, licenseKey });
    return json({ licenseKey, emailSent: emailResult.sent });
  } catch (error) {
    return json({ error: error.message || "Unable to recover license" }, 500);
  }
}
