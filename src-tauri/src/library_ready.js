/**
 * Minimal injected library-readiness signal. Reports DOM readiness before all
 * page resources load, while rejecting unrelated origins and account-sign-in
 * pages.
 */

// Start library automation at DOM readiness, without waiting for ads/images.
(() => {
  if (
    location.protocol !== "https:" ||
    !["www.audible.com", "audible.com"].includes(location.hostname) ||
    !location.pathname.startsWith("/library")
  )
    return;
  const ready = () => {
    location.href = "audible-launch://result?code=ready";
  };
  if (document.readyState === "loading")
    document.addEventListener("DOMContentLoaded", ready, { once: true });
  else ready();
})();
