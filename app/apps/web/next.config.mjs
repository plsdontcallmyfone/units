// Changed by Hookwars: security headers and unoptimized images (app audit A-1, A-5).
/** API_URL is the backend the site's server proxies (06: the browser talks only to /api). */

/**
 * Security headers (audit A-5). The browser talks only to this site (`/api` is proxied server
 * side), so `connect-src` is `'self'`; wallets inject into the page from their extension and need
 * no network origin here. Next's app router bootstraps with inline scripts, hence `'unsafe-inline'`
 * for scripts and styles until nonces are wired. Nothing may frame the site (wallet approvals must
 * not be clickjacked).
 */
const csp = [
  "default-src 'self'",
  // Next's dev runtime evaluates code; production does not.
  `script-src 'self' 'unsafe-inline'${process.env.NODE_ENV === 'development' ? " 'unsafe-eval'" : ''}`,
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data: blob:",
  "font-src 'self' data:",
  "connect-src 'self'",
  // The landing page frames the archived site from this origin (/plnty); nothing else may frame us.
  "frame-ancestors 'self'",
  "base-uri 'self'",
  "form-action 'self'",
  "object-src 'none'",
].join('; ');

export const securityHeaders = [
  { key: 'Content-Security-Policy', value: csp },
  { key: 'X-Frame-Options', value: 'SAMEORIGIN' },
  { key: 'X-Content-Type-Options', value: 'nosniff' },
  { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
  { key: 'Permissions-Policy', value: 'camera=(), microphone=(), geolocation=()' },
];

const nextConfig = {
  transpilePackages: ['@hookwars/shared'],
  reactStrictMode: true,
  poweredByHeader: false,
  // The site serves no optimized images; turning the optimizer off closes /_next/image (audit A-1).
  images: { unoptimized: true },
  async headers() {
    return [{ source: '/:path*', headers: securityHeaders }];
  },
};
export default nextConfig;
