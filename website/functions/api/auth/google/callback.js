import { createAuthSession, htmlPage, json, requireEnv, sessionCookie } from "../../_utils.js";

async function exchangeGoogleCode(env, code) {
  const body = new URLSearchParams({
    code,
    client_id: env.GOOGLE_CLIENT_ID,
    client_secret: env.GOOGLE_CLIENT_SECRET,
    redirect_uri: env.GOOGLE_AUTH_REDIRECT_URI,
    grant_type: "authorization_code",
  });

  const response = await fetch("https://oauth2.googleapis.com/token", {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body,
  });

  const data = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(data.error_description || data.error || "Google token exchange failed");
  }
  return data;
}

async function fetchGoogleProfile(accessToken) {
  const response = await fetch("https://openidconnect.googleapis.com/v1/userinfo", {
    headers: { authorization: `Bearer ${accessToken}` },
  });
  const profile = await response.json().catch(() => ({}));
  if (!response.ok || !profile.sub || !profile.email) {
    throw new Error(profile.error_description || "Unable to read Google profile");
  }
  return profile;
}

async function upsertUser(env, profile) {
  const email = String(profile.email || "").trim().toLowerCase();
  const name = String(profile.name || "").trim();
  const picture = String(profile.picture || "").trim();
  const existingByEmail = await env.DB.prepare("SELECT id FROM users WHERE email = ?")
    .bind(email)
    .first();

  if (existingByEmail) {
    await env.DB.prepare(
      "UPDATE users SET google_sub = ?, last_login_at = datetime('now') WHERE id = ?",
    )
      .bind(profile.sub, existingByEmail.id)
      .run();
    return env.DB.prepare("SELECT * FROM users WHERE id = ?").bind(existingByEmail.id).first();
  }

  await env.DB.prepare(
    `INSERT INTO users (google_sub, email, name, picture, last_login_at)
     VALUES (?, ?, ?, ?, datetime('now'))
     ON CONFLICT(google_sub) DO UPDATE SET
       email = excluded.email,
       name = excluded.name,
       picture = excluded.picture,
       last_login_at = datetime('now')`,
  )
    .bind(profile.sub, email, name || null, picture || null)
    .run();

  return env.DB.prepare("SELECT * FROM users WHERE google_sub = ?").bind(profile.sub).first();
}

function escapeHtml(value) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function appSuccessPage({ appUrl, email }) {
  const escapedEmail = escapeHtml(email);
  const escapedAppUrl = escapeHtml(appUrl);

  return new Response(`<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Successfully Authenticated | Raven Notch</title>
  <link rel="icon" type="image/png" href="/assets/app_logo.png?v=1">
  <link rel="shortcut icon" type="image/png" href="/assets/app_logo.png?v=1">
  <style>
    * {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }

    body {
      min-height: 100vh;
      overflow: hidden;
      display: flex;
      align-items: center;
      justify-content: center;
      background: #ffffff;
      color: #202124;
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
      position: relative;
    }

    #particle-svg {
      position: absolute;
      inset: 0;
      width: 100%;
      height: 100%;
      z-index: 1;
      pointer-events: none;
      transform-origin: 50% 50%;
      animation: rotate-spiral 80s linear infinite;
    }

    .content {
      position: relative;
      z-index: 2;
      width: min(760px, calc(100vw - 40px));
      padding: 32px 24px;
      text-align: center;
      transform: translateY(10px);
      opacity: 0;
      animation: content-in 700ms cubic-bezier(0.16, 1, 0.3, 1) forwards;
    }

    .logo-row {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 14px;
      margin-bottom: 24px;
    }

    .logo {
      width: 42px;
      height: 42px;
      object-fit: contain;
    }

    .brand {
      font-size: clamp(30px, 4vw, 42px);
      font-weight: 560;
      letter-spacing: -1.2px;
      color: #202124;
    }

    .mark {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 28px;
      height: 28px;
      margin-left: 2px;
      border-radius: 50%;
      background: #34a853;
      color: #ffffff;
      font-size: 17px;
      font-weight: 800;
      vertical-align: 3px;
      box-shadow: 0 8px 24px rgba(52, 168, 83, 0.22);
    }

    h1 {
      margin-bottom: 24px;
      color: #202124;
      font-size: clamp(28px, 3.4vw, 36px);
      font-weight: 400;
      letter-spacing: -0.7px;
      line-height: 1.2;
    }

    .subline {
      margin-bottom: 20px;
      color: #5f6368;
      font-size: 14px;
      line-height: 1.7;
    }

    .details {
      min-height: 24px;
      margin-bottom: 26px;
      color: #188038;
      font-size: 15px;
      line-height: 1.6;
    }

    .links {
      display: flex;
      justify-content: center;
      gap: 16px;
      font-size: 14px;
    }

    a {
      color: #1a73e8;
      text-decoration: none;
    }

    a:hover {
      text-decoration: underline;
    }

    @keyframes rotate-spiral {
      from { transform: rotate(0deg); }
      to { transform: rotate(360deg); }
    }

    @keyframes content-in {
      to {
        transform: translateY(0);
        opacity: 1;
      }
    }

    @media (prefers-reduced-motion: reduce) {
      #particle-svg,
      .content {
        animation: none;
      }

      .content {
        transform: none;
        opacity: 1;
      }
    }

    @media (max-width: 560px) {
      .logo-row {
        gap: 10px;
      }

      .logo {
        width: 34px;
        height: 34px;
      }

      .links {
        gap: 12px;
      }
    }
  </style>
</head>
<body>
  <svg id="particle-svg" aria-hidden="true"></svg>

  <main class="content">
    <div class="logo-row">
      <img class="logo" src="/assets/app_logo.png?v=1" alt="Raven Notch">
      <div class="brand">Raven Notch <span class="mark">✓</span></div>
    </div>
    <h1>You have successfully authenticated.</h1>
    <p class="subline">
      You should be redirected back to Raven Notch.
      <a href="${escapedAppUrl}">Click here</a> if not working.
    </p>
    <p class="details">Signed in as ${escapedEmail}.</p>
    <nav class="links" aria-label="Helpful links">
      <a href="/">Website</a>
      <a href="javascript:window.close()">Close tab</a>
    </nav>
  </main>

  <script>
    (function () {
      const appUrl = ${JSON.stringify(appUrl)};
      const svg = document.getElementById("particle-svg");
      const colors = ["#4285F4", "#EA4335", "#FBBC05", "#34A853", "#7E57C2", "#FF7A00"];

      function drawSpiral() {
        const width = window.innerWidth || 1440;
        const height = window.innerHeight || 820;
        const centerX = width * 0.5;
        const centerY = height * 0.5;
        const maxRadius = Math.sqrt(width * width + height * height) * 0.62;
        const count = Math.min(620, Math.max(360, Math.floor((width * height) / 2600)));

        svg.setAttribute("viewBox", "0 0 " + width + " " + height);
        svg.setAttribute("width", width);
        svg.setAttribute("height", height);
        svg.innerHTML = "";

        for (let i = 0; i < count; i += 1) {
          const t = i / count;
          const angle = 0.42 + i * 0.155;
          const radius = 22 + Math.pow(t, 0.92) * maxRadius;
          const noiseX = (Math.random() - 0.5) * 20;
          const noiseY = (Math.random() - 0.5) * 20;
          const x = centerX + radius * Math.cos(angle) + noiseX;
          const y = centerY + radius * Math.sin(angle) + noiseY;
          const length = 2.5 + Math.random() * 7.5;
          const dx = -Math.sin(angle) * length;
          const dy = Math.cos(angle) * length;
          const line = document.createElementNS("http://www.w3.org/2000/svg", "line");

          line.setAttribute("x1", x.toFixed(1));
          line.setAttribute("y1", y.toFixed(1));
          line.setAttribute("x2", (x + dx).toFixed(1));
          line.setAttribute("y2", (y + dy).toFixed(1));
          line.setAttribute("stroke", colors[i % colors.length]);
          line.setAttribute("stroke-width", (0.9 + Math.random() * 1.7).toFixed(1));
          line.setAttribute("stroke-linecap", "round");

          let opacity = 0.86;
          if (radius < 130) {
            opacity = radius / 130;
          } else if (radius > maxRadius * 0.78) {
            opacity = Math.max(0.1, 1 - (radius - maxRadius * 0.78) / (maxRadius * 0.28));
          }

          line.setAttribute("opacity", opacity.toFixed(2));
          svg.appendChild(line);
        }
      }

      drawSpiral();
      window.addEventListener("resize", drawSpiral);
      window.setTimeout(function () {
        window.location.href = appUrl;
      }, 650);
    })();
  </script>
</body>
</html>`, {
    headers: {
      "content-type": "text/html; charset=utf-8",
      "cache-control": "no-store",
    },
  });
}

export async function onRequestGet({ request, env }) {
  try {
    requireEnv(env, ["GOOGLE_CLIENT_ID", "GOOGLE_CLIENT_SECRET", "GOOGLE_AUTH_REDIRECT_URI"]);

    const url = new URL(request.url);
    const code = url.searchParams.get("code");
    const state = url.searchParams.get("state");

    if (!code || !state) {
      return json({ error: "Missing Google auth code or state" }, 400);
    }

    const authState = await env.DB.prepare("SELECT * FROM auth_states WHERE state = ?")
      .bind(state)
      .first();
    if (!authState || new Date(authState.expires_at).getTime() < Date.now()) {
      return json({ error: "Google login session expired. Please try again." }, 400);
    }

    await env.DB.prepare("DELETE FROM auth_states WHERE state = ?").bind(state).run();

    const googleTokens = await exchangeGoogleCode(env, code);
    const profile = await fetchGoogleProfile(googleTokens.access_token);
    const user = await upsertUser(env, profile);
    const session = await createAuthSession(env, user.id, authState.source);

    if (authState.source === "app") {
      const appUrl = `ravennotch://auth?token=${encodeURIComponent(session.token)}`;
      return appSuccessPage({ appUrl, email: user.email });
    }

    return new Response(null, {
      status: 302,
      headers: {
        "location": "/?login=success",
        "set-cookie": sessionCookie(session.token, session.expiresAt),
        "cache-control": "no-store",
      },
    });
  } catch (error) {
    return htmlPage(
      "Raven Notch Login Failed",
      `<h1>Login failed</h1><p>${error.message || "Unable to complete Google login."}</p><p><a href="/">Back to Raven Notch</a></p>`,
    );
  }
}
