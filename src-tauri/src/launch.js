/**
 * Injected title-selection helper for the signed-in official library. Finds the
 * requested book's own Listen now control, searches/highlights when necessary
 * and reports bounded launch progress back to the host.
 */

// Click only the selected title's real Listen now control on Audible's library.
// Does not read account fields, cookies, link tokens, or construct player URLs.
// The native host wraps this declaration as a function expression before calling it.
function launchSelectedTitle(asin, title = "", allowSearch = true) {
  if (
    location.protocol !== "https:" ||
    !["audible.com", "www.audible.com"].includes(location.hostname) ||
    !location.pathname.startsWith("/library") ||
    !/^[A-Z0-9]{10}$/.test(asin)
  )
    return;
  const route = location.pathname + location.search;
  if (
    window.__audibleLaunchJob?.asin === asin &&
    window.__audibleLaunchJob.route === route &&
    !window.__audibleLaunchJob.stopped
  )
    return;
  window.__audibleLaunchCancel?.();
  const job = { asin, route, stopped: false };
  window.__audibleLaunchJob = job;
  window.__audibleLaunchDebug = {
    matchedControls: 0,
    clickAttempts: 0,
    titleFound: false,
    popupRedirects: 0,
  };
  let stopped = false,
    timer,
    scheduledAt = Infinity,
    observer,
    focused = false,
    banner,
    bannerText,
    dismissed = false,
    clicks = 0,
    controlClicks = 0,
    lastControl = null,
    nextClickAt = 0,
    searchRequested = !allowSearch,
    started = Date.now();
  const marked = [];
  let originalOpen,
    redirectOpen,
    popupRedirects = 0;
  const stop = () => {
    stopped = true;
    job.stopped = true;
    clearTimeout(timer);
    observer?.disconnect();
    if (redirectOpen && window.open === redirectOpen)
      window.open = originalOpen;
  };
  const schedule = (delay) => {
    if (stopped) return;
    const due = Date.now() + delay;
    if (timer !== undefined && scheduledAt <= due) return;
    clearTimeout(timer);
    scheduledAt = due;
    timer = setTimeout(() => {
      timer = undefined;
      scheduledAt = Infinity;
      attempt();
    }, delay);
  };
  window.__audibleLaunchCancel = () => {
    stop();
    for (const undo of marked) undo();
    banner?.remove();
  };
  const notice = (message) => {
    if (dismissed || !document.createElement || !document.body) return;
    if (!banner) {
      banner = document.createElement("div");
      for (const [key, value] of Object.entries({
        position: "fixed",
        bottom: "16px",
        left: "16px",
        display: "flex",
        gap: "12px",
        "align-items": "center",
        "max-width": "min(520px, calc(100vw - 64px))",
        padding: "12px 16px",
        background: "#172230",
        color: "#fff",
        border: "2px solid #ffb34d",
        "border-radius": "10px",
        "box-shadow": "0 4px 20px #0005",
        "z-index": "2147483647",
        font: "14px system-ui",
        "pointer-events": "auto",
        "white-space": "normal",
        "overflow-wrap": "anywhere",
        "box-sizing": "border-box",
      }))
        banner.style.setProperty(key, value, "important");
      for (const event of [
        "click",
        "pointerdown",
        "pointerup",
        "mousedown",
        "mouseup",
        "touchstart",
      ])
        banner.addEventListener(event, (e) => e.stopPropagation());
      bannerText = document.createElement("span");
      bannerText.setAttribute("role", "status");
      banner.appendChild(bannerText);
      const close = document.createElement("button");
      close.type = "button";
      close.textContent = "×";
      close.setAttribute("aria-label", "Dismiss playback notice");
      close.style.setProperty("color", "#fff");
      close.style.setProperty("background", "#27313f");
      close.style.setProperty("padding", "4px 10px");
      close.style.setProperty("cursor", "pointer");
      close.addEventListener("click", (e) => {
        e.preventDefault();
        e.stopPropagation();
        dismissed = true;
        banner.remove();
      });
      banner.appendChild(close);
      document.body.appendChild(banner);
    }
    bannerText.textContent = message;
  };
  const highlight = (target) => {
    if (focused || !visible(target)) return;
    focused = true;
    if (target.style) {
      const outline = target.style.getPropertyValue("outline"),
        priority = target.style.getPropertyPriority("outline"),
        offset = target.style.getPropertyValue("outline-offset");
      target.style.setProperty("outline", "3px solid #f5a623", "important");
      target.style.setProperty("outline-offset", "5px");
      marked.push(() => {
        if (outline) target.style.setProperty("outline", outline, priority);
        else target.style.removeProperty("outline");
        if (offset) target.style.setProperty("outline-offset", offset);
        else target.style.removeProperty("outline-offset");
      });
    }
    target.scrollIntoView?.({
      behavior: "instant",
      block: "center",
      inline: "nearest",
    });
    notice(
      title
        ? "Selected book: " + title + " · Finding its Listen now control…"
        : "Selected book highlighted · Finding its Listen now control…",
    );
  };
  const finish = (code) => {
    stop();
    location.href = "audible-launch://result?asin=" + asin + "&code=" + code;
  };
  // Synthetic Listen now clicks can reach window.open without producing a native
  // popup event. Reuse only the actual official player URL for the selected book;
  // do not invent a launch URL or intercept unrelated windows/sign-in flows.
  if (typeof window.open === "function") {
    originalOpen = window.open;
    redirectOpen = function (...args) {
      try {
        const target = new URL(String(args[0] ?? ""), location.href);
        if (
          !stopped &&
          clicks > 0 &&
          target.protocol === "https:" &&
          ["audible.com", "www.audible.com"].includes(target.hostname) &&
          !target.username &&
          !target.password &&
          (!target.port || target.port === "443") &&
          target.pathname === "/webplayer" &&
          target.searchParams.get("asin") === asin &&
          target.searchParams.get("isSample") !== "true"
        ) {
          popupRedirects++;
          window.__audibleLaunchDebug.popupRedirects = popupRedirects;
          stop();
          location.href = target.href;
          return window;
        }
      } catch {}
      return originalOpen.apply(this, args);
    };
    window.open = redirectOpen;
  }
  const identity = (e) => {
    for (const key of ["data-asin", "data-item-asin"]) {
      const data = e.getAttribute(key);
      if (data && /^[A-Z0-9]{10}$/.test(data)) return data;
    }
    const rowId = e
      .getAttribute("id")
      ?.match(
        /^(?:adbl-library-content-row|library-content-row)-([A-Z0-9]{10})$/,
      );
    if (rowId) return rowId[1];
    if (e.tagName !== "A") return null;
    try {
      const u = new URL(e.getAttribute("href"), location.href);
      if (
        u.protocol !== "https:" ||
        !["www.audible.com", "audible.com"].includes(u.hostname)
      )
        return null;
      return u.pathname === "/webplayer"
        ? u.searchParams.get("asin")
        : (u.pathname.match(/\/pd\/(?:[^/]+\/)?([A-Z0-9]{10})(?:\/|$)/)?.[1] ??
            null);
    } catch {
      return null;
    }
  };
  const visible = (e) =>
    e.getClientRects().length > 0 && e.getAttribute("aria-hidden") !== "true";
  // HTMLLIElement.value is numeric; only textual fields are button labels.
  const isListen = (e) =>
    visible(e) &&
    !e.disabled &&
    e.getAttribute("aria-disabled") !== "true" &&
    [e.getAttribute("aria-label"), e.textContent, e.value].some(
      (value) =>
        typeof value === "string" &&
        /^listen now(?:\s|$)/i.test(value.replace(/\s+/g, " ").trim()),
    );
  const disabled = (e) =>
    !e ||
    !visible(e) ||
    e.disabled ||
    e.getAttribute("aria-disabled") === "true" ||
    e.classList?.contains("bc-button-disabled");
  const controlSelector =
    'button,a,[role="button"],input[type="button"],input[type="submit"]';
  const controls = (scope) => {
    const result = new Set();
    const labels = [
      scope,
      ...scope.querySelectorAll(
        controlSelector +
          ",.bc-button,.bc-button-text,[onclick],[data-action],span",
      ),
    ];
    for (const label of labels) {
      if (!isListen(label)) continue;
      const wrapper = label.closest?.(".bc-button");
      if (wrapper) {
        if (disabled(wrapper)) continue;
        const native = [...wrapper.querySelectorAll(controlSelector)].filter(
          (e) => !disabled(e),
        );
        const target = native.find(isListen) ?? native[0] ?? wrapper;
        if (!disabled(target)) result.add(target);
      } else {
        const target =
          label.closest?.(controlSelector + ",[onclick],[data-action]") ??
          (["A", "BUTTON", "INPUT"].includes(label.tagName) ||
          label.getAttribute("role") === "button"
            ? label
            : null);
        if (target && !disabled(target)) result.add(target);
      }
    }
    return [...result];
  };
  const identities =
    '[data-asin],[data-item-asin],[id^="adbl-library-content-row-"],[id^="library-content-row-"],a[href]';
  const rowSelector =
    'li,article,[role="listitem"],div[data-asin],div[data-item-asin],[id^="adbl-library-content-row-"],[id^="library-content-row-"],.adbl-library-content-row';
  const belongsToSelected = (control, row) => {
    for (
      let scope = control;
      scope && scope !== row;
      scope = scope.parentElement
    ) {
      const id = identity(scope);
      if (id && id !== asin) return false;
      if (scope.matches?.(rowSelector)) {
        const ids = new Set(
          [
            identity(scope),
            ...[...scope.querySelectorAll(identities)].map(identity),
          ].filter(Boolean),
        );
        if (ids.size && !ids.has(asin)) return false;
      }
    }
    return true;
  };
  // Work back from the exact Audible control supplied in the live HTML. Generic
  // containers need not have a known class, role, or a single product link.
  const exactControls = () => {
    const result = new Set();
    for (const wrapper of document.querySelectorAll(
      ".adbl-library-listen-now-button",
    )) {
      if (disabled(wrapper)) continue;
      const buttons = [...wrapper.querySelectorAll("button")].filter(
        (b) => !disabled(b) && isListen(b),
      );
      if (buttons.length !== 1) continue;
      for (
        let scope = wrapper.parentElement, depth = 0;
        scope && depth < 24;
        scope = scope.parentElement, depth++
      ) {
        const explicit = identity(scope);
        if (explicit && explicit !== asin) break;
        const ids = new Set(
          [
            identity(scope),
            ...[...scope.querySelectorAll(identities)].map(identity),
          ].filter(Boolean),
        );
        if (!ids.size) continue;
        const wrappers = [
          ...scope.querySelectorAll(".adbl-library-listen-now-button"),
        ].filter((e) => !disabled(e));
        // One real Listen now button per book. Never match an entire library group.
        if (wrappers.length !== 1 || !ids.has(asin)) break;
        const titles = [...scope.querySelectorAll("a[href]")].filter((a) => {
          try {
            return (
              !!a.textContent?.trim() &&
              new URL(
                a.getAttribute("href"),
                location.href,
              ).pathname.startsWith("/pd/") &&
              identity(a)
            );
          } catch {
            return false;
          }
        });
        // A related-title link inside another book must not authorize its button.
        if (titles.length && identity(titles[0]) !== asin) break;
        if (!titles.length && explicit !== asin) continue;
        if (belongsToSelected(buttons[0], scope)) {
          result.add(buttons[0]);
          highlight(scope);
        }
        break;
      }
    }
    return result;
  };
  const search = () => {
    if (
      searchRequested ||
      !title ||
      title.length > 4000 ||
      new URL(location.href).searchParams.get("searchTerm") === title
    )
      return false;
    const fields = [
      ...document.querySelectorAll('input[name="searchTerm"]'),
    ].filter(visible);
    if (fields.length !== 1) return false;
    const field = fields[0],
      form = field.form;
    if (!form || typeof form.requestSubmit !== "function") return false;
    const action = new URL(form.action || location.href, location.href);
    if (
      action.protocol !== "https:" ||
      !["www.audible.com", "audible.com"].includes(action.hostname) ||
      !action.pathname.startsWith("/library")
    )
      return false;
    field.value = title;
    field.dispatchEvent(new Event("input", { bubbles: true }));
    field.dispatchEvent(new Event("change", { bubbles: true }));
    searchRequested = true;
    started = Date.now();
    lastControl = null;
    controlClicks = 0;
    nextClickAt = 0;
    notice("Searching your Audible library for " + title + "…");
    location.href = "audible-launch://result?asin=" + asin + "&code=searching";
    schedule(100);
    form.requestSubmit();
    return true;
  };
  const attemptOnce = () => {
    if (stopped) return;
    if (!location.pathname.startsWith("/library")) {
      stop();
      return;
    }
    job.route = location.pathname + location.search;
    if (Date.now() - started > 20000) {
      notice(
        "Audible has not opened playback yet · Your selected book remains highlighted; use Listen now.",
      );
      finish("highlighted");
      return;
    }
    const matches = new Set();
    let found = false;
    const exact = exactControls();
    if (!exact.size)
      for (const anchor of document.querySelectorAll(identities)) {
        if (identity(anchor) !== asin || !visible(anchor)) continue;
        found = true;
        let target = anchor;
        let row = anchor.closest?.(rowSelector),
          matchedRow = false;
        for (
          let depth = 0;
          row && depth < 8;
          depth++, row = row.parentElement?.closest?.(rowSelector)
        ) {
          if (identity(row) && identity(row) !== asin) break;
          const candidates = controls(row).filter((c) =>
            belongsToSelected(c, row),
          );
          if (candidates.length) {
            for (const c of candidates) matches.add(c);
            highlight(row);
            matchedRow = true;
            break;
          }
          // A title can be inside a nested list item while Listen now is its sibling.
          // Do not stop at the first semantic container if it has no playback control.
          if (!focused) target = row;
        }
        if (matchedRow) continue;
        for (
          let scope = anchor, depth = 0;
          scope && depth < 8;
          scope = scope.parentElement, depth++
        ) {
          const ids = new Set(
            [
              identity(scope),
              ...[...scope.querySelectorAll(identities)].map(identity),
            ].filter(Boolean),
          );
          // Never climb into a row/container which also identifies a different book.
          if ([...ids].some((id) => id !== asin)) break;
          if (
            !["A", "BUTTON", "INPUT", "BODY", "HTML", "MAIN"].includes(
              scope.tagName,
            ) &&
            identity(scope) === asin
          )
            target = scope;
          const candidates = controls(scope);
          if (candidates.length) {
            if (!["BODY", "HTML", "MAIN"].includes(scope.tagName))
              target = scope;
            for (const c of candidates) matches.add(c);
            break;
          }
        }
        highlight(target);
      }
    if (exact.size) {
      matches.clear();
      for (const control of exact) matches.add(control);
      found = true;
    }
    window.__audibleLaunchDebug = {
      matchedControls: matches.size,
      exactControls: exact.size,
      clickAttempts: clicks,
      titleFound: found,
      popupRedirects,
    };
    if (matches.size === 1) {
      const control = [...matches][0];
      if (control !== lastControl) {
        lastControl = control;
        controlClicks = 0;
        nextClickAt = 0;
      }
      if (Date.now() < nextClickAt) {
        schedule(Math.min(200, nextClickAt - Date.now()));
        return;
      }
      if (controlClicks >= 8) {
        schedule(200);
        return;
      }
      const delay = [250, 600, 1200, 2400, 4000][Math.min(controlClicks, 4)];
      clicks++;
      controlClicks++;
      nextClickAt = Date.now() + delay;
      window.__audibleLaunchDebug.clickAttempts = clicks;
      notice(
        title ? "Starting " + title + "…" : "Starting your selected book…",
      );
      location.href = "audible-launch://result?asin=" + asin + "&code=opening";
      control.click();
      schedule(delay);
      return;
    }
    if (matches.size > 1) {
      notice(
        "More than one playback control matched · Choose Listen now beside the highlighted book.",
      );
      finish("ambiguous");
      return;
    }
    if (
      !found &&
      Date.now() - started >= 150 &&
      document.readyState !== "loading" &&
      search()
    )
      return;
    if (Date.now() - started > 15000) {
      if (found) {
        notice(
          title
            ? "Selected book: " +
                title +
                " · Automatic launch could not find an enabled control; use Listen now."
            : "Your selected book is highlighted · Use Listen now.",
        );
        finish("highlighted");
      } else {
        notice(
          title
            ? "Not found on this page: " +
                title +
                " · Search your Audible library for this title."
            : "The selected book was not found on this library page.",
        );
        finish("manual");
      }
      return;
    }
    schedule(100);
  };
  const attempt = () => {
    try {
      attemptOnce();
    } catch (error) {
      window.__audibleLaunchDebug = {
        ...window.__audibleLaunchDebug,
        failed: true,
        errorKind: ["TypeError", "ReferenceError", "SyntaxError"].includes(
          error?.name,
        )
          ? error.name
          : "Error",
      };
      notice(
        "Automatic launch could not complete · Use Listen now beside the highlighted book.",
      );
      finish("failed");
    }
  };
  if (typeof MutationObserver !== "undefined" && document.documentElement) {
    observer = new MutationObserver((records) => {
      if (
        records.some(
          (record) =>
            !(
              banner &&
              (record.target === banner ||
                banner.contains(record.target) ||
                (record.addedNodes?.length &&
                  [...record.addedNodes].every((node) => node === banner)))
            ),
        )
      )
        schedule(40);
    });
    observer.observe(document.documentElement, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: [
        "disabled",
        "aria-disabled",
        "data-asin",
        "data-item-asin",
      ],
    });
  }
  attempt();
}
