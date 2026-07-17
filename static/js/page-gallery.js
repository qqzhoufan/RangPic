/* my gallery: filter, select/delete, edit modal */
(function () {
  "use strict";
  const { onReady, GET, PATCH, POST, icon, toast, copy, bytes, num, escapeHtml, ago, datetime, modal, confirmDialog } = window.R;

  const state = {
    page: 1,
    limit: 24,
    filters: { q: "", tag: "", privacy: "" },
    album: null,
    selectMode: false,
    selected: new Set(),
    byId: {},
  };

  function readAlbumFromUrl() {
    const a = new URLSearchParams(location.search).get("album");
    if (a) state.album = a;
  }

  async function load() {
    const grid = document.getElementById("grid");
    grid.innerHTML = Array.from({ length: 8 }).map(() => '<div class="skeleton" style="height:180px"></div>').join("");
    const f = state.filters;
    const p = new URLSearchParams({ page: state.page, limit: state.limit });
    if (f.q) p.set("q", f.q);
    if (f.tag) p.set("tag", f.tag);
    if (f.privacy) p.set("privacy", f.privacy);
    if (state.album) p.set("album", state.album);
    try {
      const res = await GET("/api/my/images?" + p);
      state.byId = {};
      res.items.forEach((it) => (state.byId[it.id] = it));
      document.getElementById("count-line").textContent = `共 ${res.total} 张图片`;
      if (!res.items.length) {
        grid.outerHTML = `<div class="empty" id="grid">${icon("gallery")}<h3>没有匹配的图片</h3><p>试试调整筛选条件，或上传新图片</p></div>`;
        document.getElementById("pager").innerHTML = "";
        return;
      }
      // restore grid element if replaced earlier
      let g = document.getElementById("grid");
      if (g.tagName !== "DIV" || !g.classList.contains("masonry")) {
        g = document.createElement("div"); g.id = "grid"; g.className = "masonry";
      }
      g.className = "masonry";
      g.innerHTML = res.items.map(cardHtml).join("");
      renderPager(res.total);
    } catch (e) {
      toast(e.message, "error");
    }
  }

  function cardHtml(it) {
    const sel = state.selected.has(it.id);
    return `<div class="img-card ${sel ? "selected" : ""}" data-id="${it.id}">
      <img src="${it.thumb || it.url}" alt="${escapeHtml(it.original_name)}" loading="lazy">
      <div class="top-overlay">
        <span class="badge ${it.privacy}">${it.privacy}</span>
        ${state.selectMode ? `<span class="sel">${sel ? icon("check", 15) : ""}</span>` : ""}
      </div>
      <div class="overlay">
        <div class="name">${escapeHtml(it.original_name || "image")}</div>
        <div class="meta">
          <span>${bytes(it.size_bytes)}</span>
          <span>${ago(it.created_at)}</span>
          <span>${icon("eye", 12)} ${num(it.views)}</span>
        </div>
      </div>
    </div>`;
  }

  function renderPager(total) {
    const pages = Math.ceil(total / state.limit) || 1;
    const el = document.getElementById("pager");
    if (pages <= 1) { el.innerHTML = ""; return; }
    const cur = state.page;
    const btn = (label, p, dis, act) =>
      `<button ${dis ? "disabled" : ""} class="${p === cur ? "active" : ""}" data-p="${p}">${label}</button>`;
    let html = btn("‹", Math.max(1, cur - 1), cur === 1);
    const range = [];
    for (let p = 1; p <= pages; p++) if (Math.abs(p - cur) <= 2 || p === 1 || p === pages) range.push(p);
    let prev = 0;
    range.forEach((p) => { if (p - prev > 1) html += `<button disabled>…</button>`; html += btn(p, p, false); prev = p; });
    html += btn("›", Math.min(pages, cur + 1), cur === pages);
    el.innerHTML = html;
  }

  async function openEdit(id) {
    const it = state.byId[id];
    if (!it) return;
    let albums = [];
    try { albums = await GET("/api/albums"); } catch {}
    const albumOpts = ['<option value="">（无）</option>']
      .concat(albums.map((a) => `<option value="${a.id}" ${a.id === it.album_id ? "selected" : ""}>${escapeHtml(a.name)}</option>`))
      .join("");
    const tagsStr = (it.tags || []).join(", ");
    const expNote = it.expires_at ? `当前将于 ${datetime(it.expires_at)} 过期` : "当前永久有效";
    const links = it.links;
    const m = modal({
      title: "编辑图片",
      body: `
        <div class="row" style="align-items:flex-start;gap:14px;margin-bottom:14px">
          <a href="${it.share || links.share}" target="_blank" class="img-card" style="width:88px;flex-shrink:0"><img src="${it.thumb || it.url}" alt=""></a>
          <div class="grow" style="min-width:0">
            <div class="mono" style="font-size:.8rem;word-break:break-all">${escapeHtml(it.original_name)}</div>
            <div class="muted" style="font-size:.8rem;margin-top:4px">${bytes(it.size_bytes)}${it.width ? ` · ${it.width}×${it.height}` : ""} · ${num(it.views)} 次浏览</div>
            <div class="muted" style="font-size:.8rem;margin-top:2px">${expNote}</div>
          </div>
        </div>
        <div class="stack">
          <div class="field"><label>可见性</label>
            <div class="segmented" id="m-privacy">
              <button data-v="public" class="${it.privacy === "public" ? "active" : ""}">公开</button>
              <button data-v="unlisted" class="${it.privacy === "unlisted" ? "active" : ""}">不公开</button>
              <button data-v="private" class="${it.privacy === "private" ? "active" : ""}">私有</button>
            </div>
          </div>
          <div class="field"><label>标签（逗号分隔）</label><input class="input" id="m-tags" value="${escapeHtml(tagsStr)}"></div>
          <div class="field"><label>相册</label><select class="select" id="m-album">${albumOpts}</select></div>
          <div class="field"><label>过期时间（小时，输入 0 表示永久）</label><input class="input" id="m-expires" type="number" min="0" placeholder="留空=不修改"></div>
          <div class="stack" style="gap:6px;margin-top:6px">
            ${[["直链", links.direct], ["Markdown", links.markdown], ["分享页", links.share]]
              .map(([k, v]) => `<div class="copy-row"><span class="code" style="min-width:64px;text-align:center;font-size:.74rem;padding:9px 6px">${k}</span><code class="code grow">${escapeHtml(v)}</code><button class="btn btn-outline copyln" data-t="${escapeHtml(v)}">${icon("copy", 16)}</button></div>`).join("")}
          </div>
        </div>`,
      actions: [
        { label: "删除", kind: "danger", onClick: async ({ close }) => {
          if (await confirmDialog("删除后无法恢复，确定删除这张图片吗？", { danger: true, confirmLabel: "删除" })) {
            try { await POST(`/api/my/images/${id}/delete`); toast("已删除"); close(); load(); }
            catch (e) { toast(e.message, "error"); }
          }
        } },
        { label: "保存", kind: "primary", onClick: async ({ close }) => {
          const privacy = m.root.querySelector("#m-privacy .active").dataset.v;
          const tags = m.root.querySelector("#m-tags").value;
          const albumId = m.root.querySelector("#m-album").value || null;
          const expRaw = m.root.querySelector("#m-expires").value;
          const body = { privacy, tags, album_id: albumId };
          if (expRaw !== "") body.expires_hours = expRaw;
          try { await PATCH(`/api/my/images/${id}`, body); toast("已保存"); close(); load(); }
          catch (e) { toast(e.message, "error"); }
        } },
        { label: "取消", kind: "outline", onClick: ({ close }) => close() },
      ],
    });
    m.root.querySelector("#m-privacy").addEventListener("click", (e) => {
      const b = e.target.closest("button"); if (!b) return;
      m.root.querySelectorAll("#m-privacy button").forEach((x) => x.classList.toggle("active", x === b));
    });
    m.root.addEventListener("click", (e) => {
      const c = e.target.closest(".copyln"); if (c) copy(c.dataset.t);
    });
  }

  onReady(function (site) {
    if (!window.R.requireAuth(site)) return;
    document.getElementById("sel-ico").innerHTML = icon("check", 16);
    readAlbumFromUrl();

    const grid = document.getElementById("grid");
    grid.addEventListener("click", (e) => {
      const card = e.target.closest(".img-card"); if (!card) return;
      const id = card.dataset.id;
      if (state.selectMode) {
        if (state.selected.has(id)) state.selected.delete(id); else state.selected.add(id);
        card.classList.toggle("selected");
        const sel = card.querySelector(".sel");
        if (sel) sel.innerHTML = state.selected.has(id) ? icon("check", 15) : "";
        document.getElementById("del-sel-btn").classList.toggle("hide", state.selected.size === 0);
      } else {
        openEdit(id);
      }
    });

    // filters
    const apply = () => { state.page = 1; load(); };
    document.getElementById("filter-btn").addEventListener("click", () => {
      state.filters.q = document.getElementById("f-search").value.trim();
      state.filters.tag = document.getElementById("f-tag").value.trim();
      state.filters.privacy = document.getElementById("f-privacy").value;
      apply();
    });
    document.getElementById("f-search").addEventListener("keydown", (e) => { if (e.key === "Enter") document.getElementById("filter-btn").click(); });
    document.getElementById("f-tag").addEventListener("keydown", (e) => { if (e.key === "Enter") document.getElementById("filter-btn").click(); });
    document.getElementById("reset-btn").addEventListener("click", () => {
      state.filters = { q: "", tag: "", privacy: "" }; state.album = null;
      document.getElementById("f-search").value = "";
      document.getElementById("f-tag").value = "";
      document.getElementById("f-privacy").value = "";
      history.replaceState(null, "", "/gallery");
      apply();
    });

    // pager
    document.getElementById("pager").addEventListener("click", (e) => {
      const b = e.target.closest("button[data-p]"); if (!b || b.disabled) return;
      state.page = +b.dataset.p; load(); window.scrollTo({ top: 0, behavior: "smooth" });
    });

    // select mode
    const selBtn = document.getElementById("select-btn");
    selBtn.addEventListener("click", () => {
      state.selectMode = !state.selectMode;
      state.selected.clear();
      selBtn.classList.toggle("btn-primary", state.selectMode);
      selBtn.classList.toggle("btn-outline", !state.selectMode);
      document.getElementById("del-sel-btn").classList.add("hide");
      load();
    });
    document.getElementById("del-sel-btn").addEventListener("click", async () => {
      if (state.selected.size === 0) return;
      if (!await confirmDialog(`确定删除选中的 ${state.selected.size} 张图片吗？此操作无法恢复。`, { danger: true, confirmLabel: "删除" })) return;
      const ids = Array.from(state.selected);
      let ok = 0;
      for (const id of ids) {
        try { await POST(`/api/my/images/${id}/delete`); ok++; } catch {}
      }
      toast(`已删除 ${ok}/${ids.length} 张`);
      state.selected.clear();
      document.getElementById("del-sel-btn").classList.add("hide");
      load();
    });

    load();
  });
})();
