/* public explore gallery */
(function () {
  "use strict";
  const { onReady, GET, icon, escapeHtml, ago, num } = window.R;
  const state = { page: 1, limit: 30 };

  async function load() {
    const grid = document.getElementById("grid");
    grid.innerHTML = Array.from({ length: 10 }).map(() => '<div class="skeleton" style="height:200px"></div>').join("");
    try {
      const res = await GET("/api/explore?limit=" + state.limit + "&page=" + state.page);
      if (!res.items.length) {
        grid.outerHTML = `<div class="empty" id="grid">${icon("globe")}<h3>还没有公开图片</h3><p>成为第一个公开分享图片的人</p></div>`;
        document.getElementById("pager").innerHTML = "";
        return;
      }
      grid.className = "masonry";
      grid.innerHTML = res.items.map((it) => `
        <a class="img-card card-hover" href="${it.share || it.links.share}">
          <img src="${it.thumb || it.url}" alt="${escapeHtml(it.original_name)}" loading="lazy">
          <div class="top-overlay"><span class="badge public">公开</span></div>
          <div class="overlay">
            <div class="name">${escapeHtml(it.original_name || "image")}</div>
            <div class="meta"><span>${it.owner_name ? escapeHtml(it.owner_name) : "匿名"}</span><span>${ago(it.created_at)}</span><span>${icon("eye", 12)} ${num(it.views || 0)}</span></div>
          </div>
        </a>`).join("");
      renderPager(res.total);
    } catch (e) {
      window.R.toast(e.message, "error");
    }
  }

  function renderPager(total) {
    const pages = Math.ceil(total / state.limit) || 1;
    const el = document.getElementById("pager");
    if (pages <= 1) { el.innerHTML = ""; return; }
    const cur = state.page;
    let html = `<button ${cur === 1 ? "disabled" : ""} data-p="${cur - 1}">‹</button>`;
    for (let p = 1; p <= pages; p++) if (Math.abs(p - cur) <= 2 || p === 1 || p === pages)
      html += `<button class="${p === cur ? "active" : ""}" data-p="${p}">${p}</button>`;
    html += `<button ${cur === pages ? "disabled" : ""} data-p="${cur + 1}">›</button>`;
    el.innerHTML = html;
  }

  onReady(async function () {
    try {
      const tags = await GET("/api/tags");
      document.getElementById("tag-rail").innerHTML = tags.slice(0, 20).map((t) =>
        `<a class="tag btn" href="/random-image?tags=${encodeURIComponent(t)}" target="_blank">${icon("tag", 13)}${escapeHtml(t)}</a>`).join("");
    } catch {}
    document.getElementById("pager").addEventListener("click", (e) => {
      const b = e.target.closest("button[data-p]"); if (!b || b.disabled) return;
      state.page = +b.dataset.p; load(); window.scrollTo({ top: 0, behavior: "smooth" });
    });
    load();
  });
})();
