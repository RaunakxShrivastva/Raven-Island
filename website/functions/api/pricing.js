import { json } from "./_utils.js";

export async function onRequestGet({ request }) {
  const country = request.cf?.country || request.headers.get("CF-IPCountry") || "IN";
  const isIndia = country === "IN";
  const currency = isIndia ? "INR" : "USD";
  const priceText = isIndia ? "₹99" : "$1.99";

  return json({
    country,
    currency,
    priceText
  });
}
