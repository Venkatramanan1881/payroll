import { getRequestConfig } from "next-intl/server";

// Every string comes from messages/<locale>.json (TSD §12, D-42).
// shortcut: English only until the Tamil and Hindi files exist; then pick the user's language from /v1/me.
export default getRequestConfig(async () => ({
  locale: "en",
  messages: (await import("../../messages/en.json")).default,
}));
