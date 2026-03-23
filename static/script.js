// --- IP parsing & validation ---

function parseIPv4(s) {
  const parts = s.split(".");
  if (parts.length !== 4) return null;
  const octets = [];
  for (const p of parts) {
    if (!/^\d{1,3}$/.test(p)) return null;
    const n = parseInt(p, 10);
    if (n > 255 || p !== String(n)) return null;
    octets.push(n);
  }
  return octets;
}

function isGlobalIPv4(o) {
  const [a, b, c] = o;
  if (a === 0) return false;
  if (a === 10) return false;
  if (a === 100 && (b & 192) === 64) return false;
  if (a === 127) return false;
  if (a === 169 && b === 254) return false;
  if (a === 172 && (b & 240) === 16) return false;
  if (a === 192 && b === 0 && c === 0) return false;
  if (a === 192 && b === 0 && c === 2) return false;
  if (a === 192 && b === 88 && c === 99) return false;
  if (a === 192 && b === 168) return false;
  if (a === 198 && (b === 18 || b === 19)) return false;
  if (a === 198 && b === 51 && c === 100) return false;
  if (a === 203 && b === 0 && c === 113) return false;
  if (a >= 224 && a <= 239) return false;
  if (a >= 240) return false;
  return true;
}

function expandIPv6(s) {
  s = s.replace(/^\[|\]$/g, "");
  const halves = s.split("::");
  if (halves.length > 2) return null;

  const parseHalf = (h) => {
    if (h === "") return [];
    const parts = h.split(":");
    const last = parts.at(-1);
    if (last.includes(".")) {
      const v4 = parseIPv4(last);
      if (!v4) return null;
      parts.pop();
      parts.push(((v4[0] << 8) | v4[1]).toString(16));
      parts.push(((v4[2] << 8) | v4[3]).toString(16));
    }
    return parts;
  };

  const left = parseHalf(halves[0]);
  if (left === null) return null;
  let right = halves.length === 2 ? parseHalf(halves[1]) : [];
  if (right === null) return null;
  if (halves.length === 1 && left.length !== 8) return null;

  let groups = left;
  if (halves.length === 2) {
    const fill = 8 - left.length - right.length;
    if (fill < 0) return null;
    groups = [...left, ...Array(fill).fill("0"), ...right];
  }
  if (groups.length !== 8) return null;

  const seg = [];
  for (const g of groups) {
    if (!/^[0-9a-fA-F]{1,4}$/.test(g)) return null;
    seg.push(parseInt(g, 16));
  }
  return seg;
}

function isGlobalIPv6(seg) {
  if (seg[0] === 0 && seg[1] === 0 && seg[2] === 0 && seg[3] === 0 &&
      seg[4] === 0 && seg[5] === 0 && seg[6] === 0 && seg[7] === 0) return false;
  if (seg[0] === 0 && seg[1] === 0 && seg[2] === 0 && seg[3] === 0 &&
      seg[4] === 0 && seg[5] === 0 && seg[6] === 0 && seg[7] === 1) return false;
  if (seg[0] === 0 && seg[1] === 0 && seg[2] === 0 && seg[3] === 0 &&
      seg[4] === 0 && seg[5] === 0xFFFF) {
    return isGlobalIPv4([seg[6] >> 8, seg[6] & 255, seg[7] >> 8, seg[7] & 255]);
  }
  if (seg[0] === 0 && seg[1] === 0 && seg[2] === 0 && seg[3] === 0 &&
      seg[4] === 0 && seg[5] === 0) return false;
  const f = seg[0];
  if ((f & 0xFFC0) === 0xFE80) return false;
  if ((f & 0xFFC0) === 0xFEC0) return false;
  if ((f & 0xFE00) === 0xFC00) return false;
  if ((f & 0xFF00) === 0xFF00) return false;
  if (f === 0x0100 && seg[1] === 0 && seg[2] === 0 && seg[3] === 0) return false;
  if (f === 0x2001 && seg[1] === 0x0DB8) return false;
  if (f === 0x2001 && seg[1] === 0x0000) return false;
  if (f === 0x2002) {
    return isGlobalIPv4([seg[1] >> 8, seg[1] & 255, seg[2] >> 8, seg[2] & 255]);
  }
  return true;
}

// --- Form validation ---

{
  const form = document.querySelector(".search-form");
  if (form) {
    const input = form.querySelector("input[name=ip]");
    const error = document.getElementById("ip-error");
    const msgInvalid = form.getAttribute("data-msg-invalid");
    const msgNonPublic = form.getAttribute("data-msg-non-public");

    const validate = (val) => {
      const s = val.trim();
      if (s === "") return null;
      const v4 = parseIPv4(s);
      if (v4) return isGlobalIPv4(v4) ? null : msgNonPublic;
      const v6 = expandIPv6(s);
      if (v6) return isGlobalIPv6(v6) ? null : msgNonPublic;
      return msgInvalid;
    };

    const showError = (msg) => {
      if (msg) {
        error.textContent = msg;
        error.hidden = false;
        input.setAttribute("aria-invalid", "true");
        input.setAttribute("aria-describedby", "ip-error");
      } else {
        error.textContent = "";
        error.hidden = true;
        input.removeAttribute("aria-invalid");
        input.removeAttribute("aria-describedby");
      }
    };

    form.addEventListener("submit", (e) => {
      const msg = validate(input.value);
      if (msg) {
        e.preventDefault();
        showError(msg);
        input.focus();
      }
    });

    input.addEventListener("input", () => {
      if (!error.hidden) showError(validate(input.value));
    });
  }
}

// --- IPv6 font scaling ---

function scaleIpv6(el) {
  if (!el) return;
  const ip = el.textContent.trim();
  if (!ip.includes(":")) return;
  const len = ip.length;
  // IPv6 addresses range from ~5 chars (e.g. "::1") to 39 chars (full form).
  // Scale font down for longer addresses. Short IPv6 (<= 16 chars) keeps default.
  // At 39 chars (max), scale to ~60% of default size.
  if (len <= 16) return;
  const scale = Math.max(1 - ((len - 16) / (39 - 16)) * 0.4, 0.6);
  const maxPx = Math.round(42 * scale);
  const vw = (6 * scale).toFixed(2);
  const minPx = Math.round(28 * scale);
  el.style.fontSize = `clamp(${minPx}px, ${vw}vw, ${maxPx}px)`;
}

// --- Copy IP to clipboard ---

function initCopyIp(wrap) {
  if (!wrap) return;
  const h1 = wrap.querySelector(".ip-display");
  const tooltip = wrap.querySelector(".ip-copied-tooltip");
  if (!h1 || !tooltip || !navigator.clipboard) return;
  let timer = null;
  wrap.style.cursor = "pointer";
  wrap.setAttribute("role", "button");
  wrap.setAttribute("tabindex", "0");
  const copyLabel = wrap.getAttribute("data-copy-label") || "Copy IP address to clipboard";
  wrap.setAttribute("aria-label", copyLabel);

  const copyIp = () => {
    const ip = h1.textContent.trim();
    navigator.clipboard.writeText(ip).then(() => {
      tooltip.hidden = false;
      clearTimeout(timer);
      timer = setTimeout(() => { tooltip.hidden = true; }, 1500);
    }).catch(() => {});
  };

  wrap.addEventListener("click", copyIp);
  wrap.addEventListener("keydown", (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      copyIp();
    }
  });
}

// --- Init primary IP display ---

{
  const wrap = document.querySelector(".ip-copy-wrap");
  initCopyIp(wrap);
  if (wrap) scaleIpv6(wrap.querySelector(".ip-display"));
}

// --- Alt IP detection (dual-stack) ---

{
  const root = document.getElementById("ip-info-root");
  if (root) {
    const section = document.getElementById("alt-ip-section");
    if (section) initAltIp(root, section);
  }
}

function initAltIp(root, section) {
  const ipv4Domain = root.getAttribute("data-ipv4-domain");
  const copiedText = root.getAttribute("data-copied-text");
  let primaryIp = root.querySelector(".ip-display")?.textContent;
  if (!primaryIp) return;
  primaryIp = primaryIp.trim();

  const primaryIsIPv6 = primaryIp.includes(":");

  const langTag = document.documentElement.getAttribute("lang") || "en";
  const mmdbKeyMap = { en: "en", de: "de", es: "es", fr: "fr", ja: "ja", ru: "ru", "pt": "pt-BR", "zh-Hans": "zh-CN", "zh-Hant": "zh-CN" };
  const mmdbKey = mmdbKeyMap[langTag] || "en";
  const yesText = root.getAttribute("data-yes-text") || "Yes";
  const noText = root.getAttribute("data-no-text") || "No";
  const tableLabel = root.getAttribute("data-table-label") || "IP address information";

  const localizedName = (names) => names?.[mmdbKey] || names?.en || "";

  const existingLabels = readLabels();

  function readLabels() {
    const labels = {};
    const table = root.querySelector(".info-table");
    if (table) {
      for (const [attr, val] of Object.entries(table.dataset)) {
        if (attr.startsWith("l")) {
          const key = attr.slice(1).toLowerCase();
          if (key) labels[key] = val;
        }
      }
    }
    for (const row of root.querySelectorAll(".info-table tbody tr")) {
      const key = row.getAttribute("data-label");
      const th = row.querySelector("th");
      if (key && th) labels[key] = th.textContent.trim();
    }
    return labels;
  }

  const labelFor = (key) => existingLabels[key] || "";

  function boolCell(val) {
    const td = document.createElement("td");
    const icon = document.createElement("i");
    icon.className = val ? "icon-check bool-true" : "icon-minus bool-false";
    icon.setAttribute("aria-hidden", "true");
    td.appendChild(icon);
    const span = document.createElement("span");
    span.className = "sr-only";
    span.textContent = val ? yesText : noText;
    td.appendChild(span);
    return td;
  }

  function addRow(tbody, label, content) {
    const tr = document.createElement("tr");
    const th = document.createElement("th");
    th.scope = "row";
    th.textContent = label;
    tr.appendChild(th);
    if (content instanceof HTMLElement) {
      tr.appendChild(content);
    } else {
      const td = document.createElement("td");
      td.textContent = content;
      tr.appendChild(td);
    }
    tbody.appendChild(tr);
  }

  function buildBlock(ip, info) {
    const frag = document.createDocumentFragment();

    // IP display + copy
    const copyWrap = document.createElement("div");
    copyWrap.className = "ip-copy-wrap";
    copyWrap.setAttribute("data-copied", copiedText);
    copyWrap.setAttribute("data-copy-label", root.querySelector(".ip-copy-wrap")?.getAttribute("data-copy-label") || "Copy IP address to clipboard");
    const h2 = document.createElement("h2");
    h2.className = "ip-display";
    h2.textContent = ip;
    copyWrap.appendChild(h2);
    const tooltip = document.createElement("span");
    tooltip.className = "ip-copied-tooltip";
    tooltip.setAttribute("aria-live", "polite");
    tooltip.hidden = true;
    tooltip.textContent = copiedText;
    copyWrap.appendChild(tooltip);
    frag.appendChild(copyWrap);

    // Info table
    const table = document.createElement("table");
    table.className = "info-table";
    table.setAttribute("aria-label", tableLabel);
    const tbody = document.createElement("tbody");

    const asn = info.asn || {};
    const asnNum = asn.autonomous_system_number;
    const asnOrg = asn.autonomous_system_organization || "";
    const countryName = localizedName(info.country?.names);
    const cityName = localizedName(info.city?.names);
    const proxy = info.proxy || {};

    if (asnNum) {
      const td = document.createElement("td");
      const a = document.createElement("a");
      a.href = `https://bgp.tools/as/${asnNum}`;
      a.target = "_blank";
      a.rel = "noopener noreferrer";
      a.textContent = `AS${asnNum}`;
      const newTabHint = document.createElement("span");
      newTabHint.className = "sr-only";
      newTabHint.textContent = ` (opens in a new tab)`;
      a.appendChild(newTabHint);
      td.appendChild(a);
      addRow(tbody, labelFor("asn"), td);
    }
    if (asnOrg) addRow(tbody, labelFor("org"), asnOrg);
    if (countryName) addRow(tbody, labelFor("country"), countryName);
    if (cityName) addRow(tbody, labelFor("city"), cityName);
    addRow(tbody, labelFor("proxy"), boolCell(proxy.is_proxy));
    addRow(tbody, labelFor("vpn"), boolCell(proxy.is_vpn));
    addRow(tbody, labelFor("hosting"), boolCell(proxy.is_hosting));
    addRow(tbody, labelFor("tor"), boolCell(proxy.is_tor));

    table.appendChild(tbody);
    frag.appendChild(table);
    return frag;
  }

  function buildSkeleton() {
    let rows = "";
    for (let i = 0; i < 8; i++) {
      rows += `<div class="skeleton-row">
        <div class="skeleton-row-label"><div class="skeleton-bone"></div></div>
        <div class="skeleton-row-value"><div class="skeleton-bone"></div></div>
      </div>`;
    }
    return `<div class="skeleton-block">
      <div class="skeleton-bone skeleton-ip"></div>
    </div>
    <div class="skeleton-table">${rows}</div>`;
  }

  function showSkeleton() {
    section.setAttribute("aria-busy", "true");
    section.setAttribute("aria-label", "Loading additional IP information");
    section.setAttribute("role", "status");
    section.innerHTML = buildSkeleton();
    section.hidden = false;
    section.classList.add("alt-visible");
  }

  function clearLoadingState() {
    section.removeAttribute("aria-busy");
    section.removeAttribute("aria-label");
    section.removeAttribute("role");
  }

  function showAltIp(ip, info) {
    section.classList.remove("alt-visible");
    setTimeout(() => {
      clearLoadingState();
      section.replaceChildren(buildBlock(ip, info));
      initCopyIp(section.querySelector(".ip-copy-wrap"));
      scaleIpv6(section.querySelector(".ip-display"));
      section.hidden = false;
      // Force reflow before adding the visible class for transition
      void section.offsetHeight;
      section.classList.add("alt-visible");
    }, primaryIsIPv6 ? 300 : 0);
  }

  async function fetchJson(ip) {
    const r = await fetch(`/json?ip=${encodeURIComponent(ip)}`);
    if (!r.ok) throw new Error(r.status);
    return r.json();
  }

  function hideSkeleton() {
    section.classList.remove("alt-visible");
    setTimeout(() => {
      clearLoadingState();
      section.replaceChildren();
      section.hidden = true;
    }, 300);
  }

  async function detectAltIp() {
    if (primaryIsIPv6 && ipv4Domain) {
      showSkeleton();
      try {
        const r = await fetch(`https://${ipv4Domain}/`, { signal: AbortSignal.timeout(5000) });
        if (!r.ok) throw new Error(r.status);
        const ip = (await r.text()).trim();
        if (!ip || ip.includes(":")) { hideSkeleton(); return; }
        const info = await fetchJson(ip);
        showAltIp(ip, info);
      } catch {
        hideSkeleton();
      }
    } else if (!primaryIsIPv6) {
      try {
        const r = await fetch("/ip", { signal: AbortSignal.timeout(5000) });
        if (!r.ok) throw new Error(r.status);
        const ip = (await r.text()).trim();
        if (!ip || ip === primaryIp || !ip.includes(":")) return;
        const info = await fetchJson(ip);
        showAltIp(ip, info);
      } catch { /* network/timeout — silently ignore */ }
    }
  }

  if (!root.hasAttribute("data-is-query")) {
    detectAltIp();
  }
}
