import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const nextConfig: NextConfig = {
  poweredByHeader: false,
  // No page may be framed (clickjacking). shortcut: a full Content-Security-Policy waits until the Google and Firebase
  // sign-in origins are fixed at deployment; add it then.
  async headers() {
    return [
      {
        source: "/:path*",
        headers: [
          { key: "X-Frame-Options", value: "DENY" },
          { key: "Content-Security-Policy", value: "frame-ancestors 'none'" },
          { key: "X-Content-Type-Options", value: "nosniff" },
          { key: "Referrer-Policy", value: "same-origin" },
        ],
      },
    ];
  },
};

export default createNextIntlPlugin()(nextConfig);
