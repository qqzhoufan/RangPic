/* eslint-disable */
// RangPic shared runtime: icons, api, theme, nav/footer, toasts, formatting.
(function () {
  "use strict";

  // ---------------- icons (24x24, stroke = currentColor) ----------------
  const P = {
    image: ['<rect x="3" y="3" width="18" height="18" rx="2"/>', '<circle cx="8.5" cy="8.5" r="1.5"/>', '<path d="m21 15-5-5L5 21"/>'],
    upload: ['<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>', '<path d="M17 8l-5-5-5 5"/>', '<path d="M12 3v12"/>'],
    link: ['<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/>', '<path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>'],
    gallery: ['<rect x="3" y="3" width="18" height="18" rx="2"/>', '<circle cx="9" cy="9" r="2"/>', '<path d="m21 15-3.5-3.5a2 2 0 0 0-2.8 0L6 20"/>'],
    folder: ['<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>'],
    settings: ['<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>', '<circle cx="12" cy="12" r="3"/>'],
    user: ['<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/>', '<circle cx="12" cy="7" r="4"/>'],
    users: ['<path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/>', '<circle cx="9" cy="7" r="4"/>', '<path d="M22 21v-2a4 4 0 0 0-3-3.87"/>', '<path d="M16 3.13a4 4 0 0 1 0 7.75"/>'],
    shield: ['<path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>'],
    sun: ['<circle cx="12" cy="12" r="4"/>', '<path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41"/>'],
    moon: ['<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>'],
    logout: ['<path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/>', '<path d="M16 17l5-5-5-5"/>', '<path d="M21 12H9"/>'],
    trash: ['<path d="M3 6h18"/>', '<path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/>', '<path d="M10 11v6M14 11v6"/>'],
    edit: ['<path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/>', '<path d="M18.5 2.5a2.12 2.12 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/>'],
    copy: ['<rect x="8" y="8" width="14" height="14" rx="2"/>', '<path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>'],
    check: ['<path d="M20 6 9 17l-5-5"/>'],
    search: ['<circle cx="11" cy="11" r="8"/>', '<path d="m21 21-4.3-4.3"/>'],
    tag: ['<path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"/>', '<circle cx="7.5" cy="7.5" r=".5" fill="currentColor" stroke="none"/>'],
    globe: ['<circle cx="12" cy="12" r="10"/>', '<path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/>', '<path d="M2 12h20"/>'],
    lock: ['<rect x="3" y="11" width="18" height="11" rx="2"/>', '<path d="M7 11V7a5 5 0 0 1 10 0v4"/>'],
    eye: ['<path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7z"/>', '<circle cx="12" cy="12" r="3"/>'],
    calendar: ['<rect x="3" y="4" width="18" height="18" rx="2"/>', '<path d="M16 2v4M8 2v4M3 10h18"/>'],
    drive: ['<path d="M22 12H2"/>', '<path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>', '<path d="M6 16h.01M10 16h.01"/>'],
    plus: ['<path d="M12 5v14M5 12h14"/>'],
    x: ['<path d="M18 6 6 18M6 6l12 12"/>'],
    menu: ['<path d="M4 6h16M4 12h16M4 18h16"/>'],
    chevron: ['<path d="m6 9 6 6 6-6"/>'],
    arrow: ['<path d="M5 12h14"/>', '<path d="m12 5 7 7-7 7"/>'],
    terminal: ['<path d="m4 17 6-6-6-6"/>', '<path d="M12 19h8"/>'],
    spark: ['<path d="M12 2l1.6 6.4L20 10l-6.4 1.6L12 18l-1.6-6.4L4 10l6.4-1.6z"/>'],
    layers: ['<path d="M12 2 2 7l10 5 10-5-10-5z"/>', '<path d="m2 12 10 5 10-5"/>', '<path d="m2 17 10 5 10-5"/>'],
    download: ['<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>', '<path d="M7 10l5 5 5-5"/>', '<path d="M12 15V3"/>'],
    external: ['<path d="M15 3h6v6"/>', '<path d="M10 14 21 3"/>', '<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>'],
    refresh: ['<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/>', '<path d="M21 3v5h-5"/>', '<path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/>', '<path d="M8 16H3v5"/>'],
    more: ['<circle cx="12" cy="12" r="1.5" fill="currentColor" stroke="none"/><circle cx="5" cy="12" r="1.5" fill="currentColor" stroke="none"/><circle cx="19" cy="12" r="1.5" fill="currentColor" stroke="none"/>'],
    info: ['<circle cx="12" cy="12" r="10"/>', '<path d="M12 16v-4M12 8h.01"/>'],
    alert: ['<path d="m21.73 18-8-14a2 2 0 0 0-3.46 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/>', '<path d="M12 9v4M12 17h.01"/>'],
    key: ['<path d="m15.5 7.5 5 5a3.5 3.5 0 1 1-5 5l-2-2L7 22l-3-3 3.5-3.5-2-2a3.5 3.5 0 1 1 5-5z"/>'],
    clock: ['<circle cx="12" cy="12" r="10"/>', '<path d="M12 6v6l4 2"/>'],
    activity: ['<path d="M22 12h-4l-3 9L9 3l-3 9H2"/>'],
    grid: ['<rect x="3" y="3" width="7" height="7" rx="1"/>', '<rect x="14" y="3" width="7" height="7" rx="1"/>', '<rect x="14" y="14" width="7" height="7" rx="1"/>', '<rect x="3" y="14" width="7" height="7" rx="1"/>'],
    heart: ['<path d="M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z"/>'],
    shuffle: ['<path d="M18 14l4 4-4 4"/>', '<path d="M18 2l4 4-4 4"/>', '<path d="M2 18h1.973a4 4 0 0 0 3.3-1.7l5.454-8.6a4 4 0 0 1 3.3-1.7H22"/>', '<path d="M2 6h1.972a4 4 0 0 1 3.6 2.2"/>', '<path d="M22 18h-6.041a4 4 0 0 1-3.3-1.8l-.359-.45"/>'],
    mail: ['<rect x="2" y="4" width="20" height="16" rx="2"/>', '<path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/>'],
    fish: ['<path d="M3 12c0-3 4-6 9-6s9 3 9 6-4 6-9 6-9-3-9-6Z"/>', '<path d="M3 12 0 8 2.6 12 0 16Z"/>', '<circle cx="16" cy="10.8" r="1" fill="currentColor" stroke="none"/>'],
  };
  function icon(name, size) {
    const s = size || 20;
    const inner = (P[name] || P.image).join("");
    return `<svg width="${s}" height="${s}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${inner}</svg>`;
  }

  // ---------------- api ----------------
  async function api(method, url, body) {
    const opt = { method, credentials: "same-origin", headers: {} };
    if (body instanceof FormData) {
      opt.body = body;
    } else if (body !== undefined) {
      opt.headers["Content-Type"] = "application/json";
      opt.body = JSON.stringify(body);
    }
    let res, txt;
    try {
      res = await fetch(url, opt);
      txt = await res.text();
    } catch (e) {
      throw new Error("网络错误，请稍后再试");
    }
    const data = txt ? JSON.parse(txt) : null;
    if (!res.ok) {
      const e = new Error((data && data.error) || `HTTP ${res.status}`);
      e.status = res.status;
      e.data = data;
      throw e;
    }
    return data;
  }
  const GET = (u) => api("GET", u);
  const POST = (u, b) => api("POST", u, b);
  const PATCH = (u, b) => api("PATCH", u, b);
  const DEL = (u) => api("DELETE", u);

  // ---------------- theme ----------------
  function storedTheme() {
    return localStorage.getItem("rangpic-theme");
  }
  function systemDark() {
    return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
  }
  function currentTheme() {
    return storedTheme() || (systemDark() ? "dark" : "light");
  }
  function applyTheme(t) {
    document.documentElement.dataset.theme = t;
  }

  // ---------------- toasts ----------------
  function ensureToasts() {
    let el = document.getElementById("toasts");
    if (!el) {
      el = document.createElement("div");
      el.id = "toasts";
      el.className = "toasts";
      document.body.appendChild(el);
    }
    return el;
  }
  function toast(msg, kind, ms) {
    const el = ensureToasts();
    const t = document.createElement("div");
    t.className = "toast" + (kind ? " " + kind : "");
    const ico = kind === "error" ? "alert" : kind === "warn" ? "info" : "check";
    t.innerHTML = icon(ico, 17) + "<div></div>";
    t.querySelector("div").textContent = msg;
    el.appendChild(t);
    setTimeout(() => {
      t.style.opacity = "0";
      t.style.transform = "translateX(20px)";
      t.style.transition = "all .2s";
      setTimeout(() => t.remove(), 220);
    }, ms || 3200);
  }

  // ---------------- clipboard ----------------
  async function copy(text, label) {
    try {
      await navigator.clipboard.writeText(text);
      toast((label || "已复制") + "到剪贴板");
    } catch {
      // fallback
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      try { document.execCommand("copy"); toast((label || "已复制") + "到剪贴板"); }
      catch { toast("复制失败", "error"); }
      ta.remove();
    }
  }

  // ---------------- formatting ----------------
  function bytes(n) {
    if (n == null || isNaN(n)) return "—";
    if (n < 1024) return n + " B";
    const u = ["KB", "MB", "GB", "TB", "PB"];
    let v = n / 1024, i = 0;
    while (v >= 1024 && i < u.length - 1) { v /= 1024; i++; }
    return v.toFixed(v < 10 ? 1 : 0) + " " + u[i];
  }
  function num(n) {
    if (n == null) return "0";
    if (n < 1000) return String(n);
    if (n < 1e6) return (n / 1e3).toFixed(n < 1e4 ? 1 : 0) + "K";
    return (n / 1e6).toFixed(1) + "M";
  }
  function date(iso) {
    if (!iso) return "";
    const d = new Date(iso);
    return d.toLocaleDateString("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit" });
  }
  function datetime(iso) {
    if (!iso) return "";
    const d = new Date(iso);
    return d.toLocaleString("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
  }
  function ago(iso) {
    if (!iso) return "";
    const d = new Date(iso).getTime();
    const s = Math.max(1, Math.floor((Date.now() - d) / 1000));
    if (s < 60) return "刚刚";
    const m = Math.floor(s / 60);
    if (m < 60) return m + " 分钟前";
    const h = Math.floor(m / 60);
    if (h < 24) return h + " 小时前";
    const day = Math.floor(h / 24);
    if (day < 30) return day + " 天前";
    return date(iso);
  }
  function extOf(name) {
    const i = name.lastIndexOf(".");
    return i >= 0 ? name.slice(i + 1).toLowerCase() : "";
  }
  function escapeHtml(s) {
    return String(s == null ? "" : s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  }

  // ---------------- nav / footer ----------------
  function brandHtml(site) {
    const name = (site && site.site_name) || "咸鱼图床";
    return `<a class="brand" href="/">
      <span class="mark">${icon("fish", 16)}</span>
      <span>${escapeHtml(name)}</span>
    </a>`;
  }
  function navLink(href, label, iconKey) {
    const active = location.pathname === href ? " active" : "";
    return `<a href="${href}" class="${active.slice(1)}">${iconKey ? icon(iconKey, 17) : ""}<span>${label}</span></a>`;
  }
  function renderNav(site) {
    const mount = document.getElementById("nav-mount");
    if (!mount) return;
    const you = site && site.you;
    const links = [];
    links.push(navLink("/explore", "发现", "globe"));
    links.push(navLink("/gallery", "图库", "gallery"));
    if (you) {
      links.push(navLink("/upload", "上传", "upload"));
      links.push(navLink("/albums", "相册", "folder"));
      links.push(navLink("/dashboard", "控制台", "grid"));
      if (you.role === "admin") links.push(navLink("/admin", "管理", "shield"));
    }
    let end;
    if (you) {
      const initial = escapeHtml(you.username.slice(0, 1).toUpperCase());
      const roleBadge = you.role === "admin" ? '<span class="badge role" style="margin-left:6px">管理员</span>' : "";
      end = `
        <button class="iconbtn" id="theme-toggle" title="切换主题">${icon(currentTheme() === "dark" ? "sun" : "moon", 19)}</button>
        <div class="menu-wrap">
          <button class="chip" id="account-btn">
            <span class="avatar">${initial}</span>
            <span>${escapeHtml(you.username)}</span>
            ${icon("chevron", 15)}
          </button>
          <div class="menu" id="account-menu">
            <a href="/dashboard">${icon("grid", 16)}控制台</a>
            <a href="/gallery">${icon("gallery", 16)}我的图库</a>
            <a href="/settings">${icon("settings", 16)}账号设置</a>
            <div class="sep"></div>
            <button id="logout-btn">${icon("logout", 16)}退出登录</button>
          </div>
        </div>`;
    } else {
      end = `
        <button class="iconbtn" id="theme-toggle" title="切换主题">${icon(currentTheme() === "dark" ? "sun" : "moon", 19)}</button>
        <a class="btn btn-ghost btn-sm" href="/login">登录</a>
        ${site && site.registration_open ? '<a class="btn btn-primary btn-sm" href="/register">注册</a>' : ""}`;
    }
    mount.outerHTML = `
      <nav class="nav">
        <div class="container nav-inner">
          ${brandHtml(site)}
          <div class="nav-links">${links.join("")}</div>
          <div class="nav-end">${end}</div>
        </div>
      </nav>`;
    wireNav(site);
  }
  function wireNav(site) {
    const toggle = document.getElementById("theme-toggle");
    if (toggle) {
      toggle.addEventListener("click", () => {
        const next = currentTheme() === "dark" ? "light" : "dark";
        localStorage.setItem("rangpic-theme", next);
        applyTheme(next);
        toggle.innerHTML = icon(next === "dark" ? "sun" : "moon", 19);
        document.dispatchEvent(new CustomEvent("rangpic:theme", { detail: next }));
      });
    }
    const acct = document.getElementById("account-btn");
    const menu = document.getElementById("account-menu");
    if (acct && menu) {
      acct.addEventListener("click", (e) => { e.stopPropagation(); menu.classList.toggle("open"); });
      document.addEventListener("click", () => menu.classList.remove("open"));
    }
    const lo = document.getElementById("logout-btn");
    if (lo) lo.addEventListener("click", async () => {
      try { await POST("/api/auth/logout"); } catch {}
      location.href = "/";
    });
  }
  function renderFooter(site) {
    const mount = document.getElementById("footer-mount");
    if (!mount) return;
    const year = new Date().getFullYear();
    mount.outerHTML = `
      <footer class="footer">
        <div class="container between">
          <div class="row">
            <span class="brand" style="font-size:1.05rem">${icon("fish", 15)}<span class="serif">${escapeHtml((site && site.site_name) || "咸鱼图床")}</span></span>
            <span class="muted">· 自托管图床，随机图只是其中一个功能</span>
          </div>
          <div class="row">
            <a href="/explore">发现</a> · <a href="/api/random-image" target="_blank">随机图 API</a> · <a href="https://github.com/qqzhoufan/RangPic" target="_blank">GitHub</a>
            <span class="muted">v${escapeHtml((site && site.version) || "")}</span>
          </div>
        </div>
        <div class="container subtle" style="margin-top:10px;font-size:.8rem">© ${year} ${escapeHtml((site && site.site_name) || "咸鱼图床")}</div>
      </footer>`;
  }

  // ---------------- bootstrap ----------------
  const siteState = { ready: false };
  const pageCbs = [];
  async function bootstrap() {
    applyTheme(currentTheme());
    try {
      const s = await GET("/api/site");
      Object.assign(siteState, s);
    } catch { /* anonymous/offline */ }
    siteState.ready = true;
    document.title = `${(siteState.site_name || "咸鱼图床")}${pageTitleSuffix()}`;
    renderNav(siteState);
    renderFooter(siteState);
    for (const cb of pageCbs) {
      try { await cb(siteState); }
      catch (e) { console.error(e); toast("页面加载出错", "error"); }
    }
  }
  function pageTitleSuffix() {
    const main = document.querySelector("[data-title]");
    return main && main.dataset.title ? " · " + main.dataset.title : "";
  }
  function onReady(cb) {
    if (siteState.ready) Promise.resolve(cb(siteState));
    else pageCbs.push(cb);
  }

  // ---------------- guards ----------------
  function requireAuth(site, redirectTo) {
    if (!site.you) {
      const next = encodeURIComponent(location.pathname + location.search);
      location.href = (redirectTo || "/login") + "?next=" + next;
      return false;
    }
    return true;
  }

  // lightbox helper
  function openLightbox(src) {
    let lb = document.getElementById("lightbox");
    if (!lb) {
      lb = document.createElement("div");
      lb.id = "lightbox";
      lb.className = "lightbox";
      lb.innerHTML = `<button class="close-x">${icon("x", 22)}</button><img alt="">`;
      document.body.appendChild(lb);
      lb.addEventListener("click", (e) => { if (e.target === lb || e.target.closest(".close-x")) lb.classList.remove("open"); });
      document.addEventListener("keydown", (e) => { if (e.key === "Escape") lb.classList.remove("open"); });
    }
    lb.querySelector("img").src = src;
    lb.classList.add("open");
  }

  document.addEventListener("DOMContentLoaded", bootstrap);

  // ---------------- modal ----------------
  function modal(opts) {
    const back = document.createElement("div");
    back.className = "modal-backdrop";
    const actions = (opts.actions || [{ label: "关闭", kind: "outline", onClick: () => close() }])
      .map((a, i) => {
        const cls = a.kind === "primary" ? "btn-primary" : a.kind === "danger" ? "btn-danger" : "btn-outline";
        return `<button class="btn ${cls}" data-i="${i}">${escapeHtml(a.label)}</button>`;
      }).join("");
    back.innerHTML = `
      <div class="modal">
        <div class="modal-head"><strong>${escapeHtml(opts.title || "")}</strong>
          <button class="iconbtn" data-close>${icon("x", 18)}</button></div>
        <div class="modal-body">${opts.body || ""}</div>
        ${actions ? `<div class="modal-foot">${actions}</div>` : ""}
      </div>`;
    document.body.appendChild(back);
    requestAnimationFrame(() => back.classList.add("open"));
    function close(result) {
      back.classList.remove("open");
      setTimeout(() => back.remove(), 160);
      if (opts.onClose) opts.onClose(result);
    }
    back.addEventListener("click", (e) => {
      if (e.target === back) close();
      if (e.target.closest("[data-close]")) close();
      const ab = e.target.closest(".modal-foot button[data-i]");
      if (ab) {
        const a = opts.actions[+ab.dataset.i];
        if (a) a.onClick({ close, root: back });
      }
    });
    document.addEventListener("keydown", function esc(e) {
      if (e.key === "Escape") { close(); document.removeEventListener("keydown", esc); }
    });
    return { close, root: back };
  }

  function confirmDialog(message, opts) {
    return new Promise((resolve) => {
      const m = modal({
        title: (opts && opts.title) || "确认操作",
        body: `<p class="muted">${escapeHtml(message)}</p>`,
        actions: [
          { label: "取消", kind: "outline", onClick: ({ close }) => { close(); resolve(false); } },
          { label: (opts && opts.confirmLabel) || "确认", kind: (opts && opts.danger) ? "danger" : "primary",
            onClick: ({ close }) => { close(); resolve(true); } },
        ],
      });
    });
  }

  // expose
  window.R = {
    icon, api, GET, POST, PATCH, DEL,
    toast, copy, bytes, num, date, datetime, ago, extOf, escapeHtml,
    onReady, requireAuth, openLightbox, modal, confirmDialog,
    get site() { return siteState; },
  };
})();
