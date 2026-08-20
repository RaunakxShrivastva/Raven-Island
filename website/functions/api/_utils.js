const PRODUCT_AMOUNT = 9900;
const PRODUCT_CURRENCY = "INR";
const PRODUCT_NAME = "Raven Notch Lifetime License";
const DEFAULT_LICENSE_FROM_EMAIL = "Raven Notch <onboarding@resend.dev>";
const SESSION_DAYS = 30;

export function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "content-type": "application/json; charset=utf-8",
      "cache-control": "no-store",
    },
  });
}

export async function readJson(request) {
  try {
    return await request.json();
  } catch {
    return {};
  }
}

export function requireEnv(env, keys) {
  for (const key of keys) {
    if (!env[key]) {
      throw new Error(`Missing environment variable: ${key}`);
    }
  }
}

export function normalizeLicenseKey(value) {
  return String(value || "").trim().toUpperCase();
}

export function normalizeDeviceId(value) {
  return String(value || "").trim().slice(0, 160);
}

export function makeReceipt() {
  const id = crypto.randomUUID().replace(/-/g, "").slice(0, 20);
  return `raven_${id}`;
}

export function makeLicenseKey() {
  const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  let raw = "";
  for (const byte of bytes) raw += alphabet[byte % alphabet.length];
  return `RAVEN-${raw.slice(0, 4)}-${raw.slice(4, 8)}-${raw.slice(8, 12)}-${raw.slice(12, 16)}`;
}

export function randomToken(bytes = 32) {
  const buffer = new Uint8Array(bytes);
  crypto.getRandomValues(buffer);
  return btoa(String.fromCharCode(...buffer))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/g, "");
}

export async function sha256Hex(message) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(message));
  return [...new Uint8Array(digest)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export async function passwordHash(password, salt) {
  return hmacHex(salt, password);
}

export function sessionCookie(token, expiresAt) {
  return [
    `raven_session=${token}`,
    "Path=/",
    "HttpOnly",
    "Secure",
    "SameSite=Lax",
    `Expires=${new Date(expiresAt).toUTCString()}`,
  ].join("; ");
}

export async function createAuthSession(env, userId, source = "web") {
  const token = randomToken(32);
  const tokenHash = await sha256Hex(token);
  const expiresAt = new Date(Date.now() + SESSION_DAYS * 24 * 60 * 60 * 1000).toISOString();

  await env.DB.prepare(
    "INSERT INTO auth_sessions (token_hash, user_id, source, expires_at, last_seen_at) VALUES (?, ?, ?, ?, datetime('now'))",
  )
    .bind(tokenHash, userId, source, expiresAt)
    .run();

  return { token, expiresAt };
}

export function getCookie(request, name) {
  const cookie = request.headers.get("cookie") || "";
  const prefix = `${name}=`;
  return cookie
    .split(";")
    .map((part) => part.trim())
    .find((part) => part.startsWith(prefix))
    ?.slice(prefix.length) || "";
}

export async function getSessionUser(env, request) {
  const token = getCookie(request, "raven_session");
  if (!token) return null;

  const tokenHash = await sha256Hex(token);
  const row = await env.DB.prepare(
    `SELECT users.id, users.email, users.name, users.picture, auth_sessions.expires_at
     FROM auth_sessions
     JOIN users ON users.id = auth_sessions.user_id
     WHERE auth_sessions.token_hash = ?`,
  )
    .bind(tokenHash)
    .first();

  if (!row || new Date(row.expires_at).getTime() < Date.now()) {
    return null;
  }

  await env.DB.prepare("UPDATE auth_sessions SET last_seen_at = datetime('now') WHERE token_hash = ?")
    .bind(tokenHash)
    .run();

  return {
    id: row.id,
    email: row.email,
    name: row.name,
    picture: row.picture,
  };
}

export function htmlPage(title, body) {
  return new Response(`<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${title}</title>
  <style>
    body{margin:0;min-height:100vh;display:grid;place-items:center;background:#070707;color:#f5f5f5;font-family:Inter,Arial,sans-serif}
    main{width:min(520px,calc(100vw - 36px));padding:34px;border:1px solid rgba(255,255,255,.12);border-radius:22px;background:rgba(18,18,20,.92);box-shadow:0 24px 80px rgba(0,0,0,.45)}
    h1{margin:0 0 10px;font-size:28px}
    p{margin:8px 0;color:rgba(255,255,255,.68);line-height:1.5}
    a{color:#34c759}
  </style>
</head>
<body><main>${body}</main></body>
</html>`, {
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-store",
    },
  });
}

export async function hmacHex(secret, message) {
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const signature = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(message));
  return [...new Uint8Array(signature)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export function constantTimeEqual(a, b) {
  const left = String(a || "");
  const right = String(b || "");
  if (left.length !== right.length) return false;
  let diff = 0;
  for (let i = 0; i < left.length; i += 1) {
    diff |= left.charCodeAt(i) ^ right.charCodeAt(i);
  }
  return diff === 0;
}

export async function createRazorpayOrder(env, options = {}) {
  requireEnv(env, ["RAZORPAY_KEY_ID", "RAZORPAY_KEY_SECRET"]);
  const auth = btoa(`${env.RAZORPAY_KEY_ID}:${env.RAZORPAY_KEY_SECRET}`);
  const amount = options.amount || PRODUCT_AMOUNT;
  const currency = options.currency || PRODUCT_CURRENCY;
  const body = {
    amount: amount,
    currency: currency,
    receipt: makeReceipt(),
    notes: {
      product: PRODUCT_NAME,
      customer_email: options.email || "",
    },
  };

  const response = await fetch("https://api.razorpay.com/v1/orders", {
    method: "POST",
    headers: {
      authorization: `Basic ${auth}`,
      "content-type": "application/json",
    },
    body: JSON.stringify(body),
  });

  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data?.error?.description || "Razorpay order creation failed");
  }
  return data;
}

export async function fetchRazorpayPayment(env, paymentId) {
  requireEnv(env, ["RAZORPAY_KEY_ID", "RAZORPAY_KEY_SECRET"]);
  const auth = btoa(`${env.RAZORPAY_KEY_ID}:${env.RAZORPAY_KEY_SECRET}`);
  const response = await fetch(`https://api.razorpay.com/v1/payments/${encodeURIComponent(paymentId)}`, {
    headers: {
      authorization: `Basic ${auth}`,
    },
  });

  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data?.error?.description || "Unable to fetch Razorpay payment");
  }
  return data;
}

export async function sendLicenseEmail(env, { email, licenseKey }) {
  const to = String(email || "").trim();
  if (!to || !env.RESEND_API_KEY) {
    return { sent: false, skipped: true };
  }

  const from = env.LICENSE_FROM_EMAIL || DEFAULT_LICENSE_FROM_EMAIL;
  const response = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: {
      authorization: `Bearer ${env.RESEND_API_KEY}`,
      "content-type": "application/json",
    },
    body: JSON.stringify({
      from,
      to,
      subject: "Your Raven Notch license key",
      text: [
        "Thanks for purchasing Raven Notch.",
        "",
        `Your license key is: ${licenseKey}`,
        "",
        "Paste this key into Raven Notch when the app asks for activation.",
        "",
        "Keep this email safe.",
      ].join("\n"),
      html: `
        <div style="font-family:Arial,sans-serif;line-height:1.5;color:#111">
          <h2>Your Raven Notch license</h2>
          <p>Thanks for purchasing Raven Notch.</p>
          <p style="font-size:18px"><strong>${licenseKey}</strong></p>
          <p>Paste this key into Raven Notch when the app asks for activation.</p>
          <p>Keep this email safe.</p>
        </div>
      `,
    }),
  });

  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    return { sent: false, error: data?.message || "Email provider rejected the message" };
  }
  return { sent: true, id: data.id || null };
}

export async function signEntitlement(env, payload) {
  requireEnv(env, ["LICENSE_SIGNING_SECRET"]);
  const message = JSON.stringify(payload);
  const signature = await hmacHex(env.LICENSE_SIGNING_SECRET, message);
  return { ...payload, signature };
}

export { PRODUCT_AMOUNT, PRODUCT_CURRENCY, PRODUCT_NAME };
