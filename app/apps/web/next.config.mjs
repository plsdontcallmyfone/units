/** API_URL is the backend the site's server proxies (06: the browser talks only to /api). */
const nextConfig = {
  transpilePackages: ['@hookwars/shared'],
  reactStrictMode: true,
  poweredByHeader: false,
};
export default nextConfig;
