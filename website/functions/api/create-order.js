import { createRazorpayOrder, getSessionUser, json, readJson } from "./_utils.js";

export async function onRequestPost({ request, env }) {
  try {
    const input = await readJson(request);
    const user = await getSessionUser(env, request);
    const email = String(user?.email || input.email || "").trim().slice(0, 180);

    // Geolocation currency detection
    const country = request.cf?.country || request.headers.get("CF-IPCountry") || "IN";
    const isIndia = country === "IN";
    const currency = isIndia ? "INR" : "USD";
    const amount = isIndia ? 9900 : 199; // ₹99 = 9900 paise, $1.99 = 199 cents

    const order = await createRazorpayOrder(env, { email, amount, currency });

    await env.DB.prepare(
      "INSERT INTO payments (razorpay_order_id, amount, currency, status, customer_email) VALUES (?, ?, ?, ?, ?)",
    )
      .bind(order.id, amount, currency, "created", email || null)
      .run();

    return json({
      keyId: env.RAZORPAY_KEY_ID,
      orderId: order.id,
      amount: amount,
      currency: currency,
      name: "Raven Notch",
      description: "Lifetime license",
    });
  } catch (error) {
    return json({ error: error.message || "Unable to create order" }, 500);
  }
}
