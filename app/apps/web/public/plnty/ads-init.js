/**
 * ads-init.js — the Google tag bootstrap, as a same-origin file.
 *
 * ⚠ THIS IS A STATIC FILE ON PURPOSE, AND THE REASON IS THE CSP.
 *
 * `apps/web/vercel.json` runs `script-src 'self' ... 'sha256-...' 'sha256-...'`.
 * An inline <script> in index.html would need a THIRD sha256 hash, and the boot
 * watchdog's own comment says what that costs: "ANY byte change to the script
 * body below (including whitespace) breaks the hash and the CSP blocks the
 * watchdog". A same-origin FILE is covered by `'self'` and has no hash to break,
 * so this can be edited freely.
 *
 * ⚠ IT MUST LOAD SYNCHRONOUSLY AND BEFORE gtag/js. Consent Mode defaults are
 * only honoured if they are in the dataLayer before the tag initialises. Pushed
 * afterwards they are an UPDATE, and the first beacon has already gone out under
 * the wrong assumption. index.html therefore loads this WITHOUT `async` and puts
 * the googletagmanager script directly after it.
 *
 * ── WHY THIS EXISTS AT ALL, WHEN A MODULE ALREADY DID IT ───────────────────
 *
 * It did, and it worked: verified on production 2026-09-09, the tag loaded and
 * the gclid reached `ad.doubleclick.net/ccm/s/collect`. But Google Ads emailed
 * "We haven't found a Google tag on your website", because their DETECTOR
 * fetches the HTML and looks for the snippet. It does not run a React bundle, so
 * a tag injected from a module is invisible to it however well it works for a
 * real visitor. The snippet has to be IN THE HTML to be found.
 *
 * ⚠ KEEP THIS IN STEP WITH THE MARKETING SITE. `apps/marketing/src/layouts/
 * BaseLayout.astro` carries the same bootstrap inline (its CSP allows
 * 'unsafe-inline', so it needs no file). The two must declare the SAME consent
 * defaults, or a visitor's treatment would change as they cross between the
 * marketing site and the app, which is the same domain to them.
 *
 * The reporting side of this (conversions, the click id) lives in
 * `src/lib/google-ads.ts`, which reads what this file sets up.
 */
(function () {
  window.dataLayer = window.dataLayer || [];
  function gtag() { window.dataLayer.push(arguments); }
  window.gtag = gtag;

  /* Consent Mode v2, region-scoped.
     There is no cookie banner in this product, and the privacy policy says
     consent will be requested "where required by law". So the EEA, the UK and
     Switzerland default to denied and everywhere else to granted, which is
     Google's own pattern for a site without a banner. EEA conversions are
     modelled rather than observed as a result. Reasoning in full:
     src/lib/google-ads.ts. */
  gtag('consent', 'default', {
    ad_storage: 'denied',
    ad_user_data: 'denied',
    ad_personalization: 'denied',
    analytics_storage: 'denied',
    region: ['AT', 'BE', 'BG', 'HR', 'CY', 'CZ', 'DK', 'EE', 'FI', 'FR', 'DE',
      'GR', 'HU', 'IS', 'IE', 'IT', 'LV', 'LI', 'LT', 'LU', 'MT', 'NL', 'NO',
      'PL', 'PT', 'RO', 'SK', 'SI', 'ES', 'SE', 'GB', 'CH'],
  });
  gtag('consent', 'default', {
    ad_storage: 'granted',
    ad_user_data: 'granted',
    ad_personalization: 'granted',
    analytics_storage: 'granted',
  });

  gtag('js', new Date());

  /* `allow_ad_personalization_signals: false` turns REMARKETING collection off.
     The untuned tag fires `google.com/rmkt/collect` on first paint, and the
     privacy policy says Plnty "does not currently process Personal Data for
     targeted advertising". Conversion counting does not need it. Turning it on
     means updating that policy in the same change. */
  gtag('config', 'AW-18440458478', { allow_ad_personalization_signals: false });

  /* A marker the module half checks, so it never configures the tag twice. */
  window.__plntyAdsLoaded = true;
})();
