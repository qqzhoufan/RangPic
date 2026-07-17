/* upload page: drag/drop/paste + URL import, real progress, copyable links */
(function () {
  "use strict";
  const { onReady, GET, POST, icon, toast, copy, bytes, escapeHtml } = window.R;

  const opts = { privacy: "public", tags: "", expires: "", album: "" };
  const ACCEPT = ["jpg", "jpeg", "png", "gif", "webp", "bmp", "ico"];

  function linkRows(links) {
    const rows = [
      ["直链", links.direct],
      ["Markdown", links.markdown],
      ["HTML", links.html],
      ["BBCode", links.bbcode],
    ];
    return rows.map(([k, v]) =>
      `<div class="copy-row"><span class="code" style="min-width:74px;text-align:center;font-size:.74rem;padding:9px 6px">${k}</span><code class="code grow">${escapeHtml(v)}</code><button class="btn btn-outline copy-row-btn" data-text="${escapeHtml(v)}">${icon("copy", 16)}</button></div>`
    ).join("");
  }

  function resultCard(r) {
    if (!r.ok) {
      return `<div class="card card-pad" style="border-left:3px solid var(--danger)">
        <div class="between"><strong>${escapeHtml(r.filename || "文件")}</strong><span class="badge private">失败</span></div>
        <p class="muted" style="margin-top:6px">${escapeHtml(r.error)}</p></div>`;
    }
    const it = r.image;
    const thumb = it.thumb || it.url;
    return `<div class="card card-pad">
      <div class="row" style="align-items:flex-start;gap:var(--s-4)">
        <a class="img-card" style="width:96px;flex-shrink:0" href="${it.share || it.links.share}"><img src="${thumb}" alt=""></a>
        <div class="grow">
          <div class="between">
            <strong>${escapeHtml(it.original_name || "image")}</strong>
            <span class="badge ${it.privacy}">${it.privacy}</span>
          </div>
          <div class="muted" style="font-size:.82rem;margin:4px 0 10px">${bytes(it.size_bytes)}${it.width ? ` · ${it.width}×${it.height}` : ""}</div>
          <div class="stack" style="gap:6px">${linkRows(it.links)}</div>
        </div>
      </div>
    </div>`;
  }

  function quotaMini(you) {
    const el = document.getElementById("quota-mini");
    if (!you) { el.innerHTML = ""; return; }
    const used = you.used_bytes, total = you.quota_bytes;
    const pct = total > 0 ? Math.min(100, (used / total) * 100) : 0;
    const cls = pct > 90 ? "danger" : pct > 75 ? "warn" : "";
    el.innerHTML = `
      <div class="between" style="font-size:.82rem"><span class="muted">存储配额</span><strong>${bytes(used)} / ${bytes(total)}</strong></div>
      <div class="meter ${cls}" style="margin-top:8px"><i style="width:${pct}%"></i></div>`;
  }

  function uploadFile(file, onProgress) {
    return new Promise((resolve, reject) => {
      const fd = new FormData();
      fd.append("file", file);
      fd.append("privacy", opts.privacy);
      if (opts.tags) fd.append("tags", opts.tags);
      if (opts.expires) fd.append("expires_hours", opts.expires);
      if (opts.album) fd.append("album_id", opts.album);
      const xhr = new XMLHttpRequest();
      xhr.open("POST", "/api/upload");
      xhr.upload.onprogress = (e) => { if (e.lengthComputable) onProgress(e.loaded / e.total); };
      xhr.onload = () => {
        let data; try { data = JSON.parse(xhr.responseText); } catch {}
        if (xhr.status >= 200 && xhr.status < 300) resolve(data);
        else reject(Object.assign(new Error((data && data.error) || "上传失败"), { parsed: data }));
      };
      xhr.onerror = () => reject(new Error("网络错误"));
      xhr.send(fd);
    });
  }

  function queueItem(file) {
    const id = "q" + Math.random().toString(36).slice(2);
    const item = document.createElement("div");
    item.className = "card card-pad";
    item.id = id;
    item.innerHTML = `
      <div class="between" style="margin-bottom:8px">
        <div class="row" style="gap:10px;min-width:0">
          <span style="color:var(--text-subtle)">${icon("image", 18)}</span>
          <span class="mono" style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${escapeHtml(file.name)}</span>
        </div>
        <span class="subtle" style="font-size:.8rem;flex-shrink:0">${bytes(file.size)}</span>
      </div>
      <div class="prog-track"><i></i></div>`;
    document.getElementById("queue").appendChild(item);
    return {
      set(p) { item.querySelector(".prog-track > i").style.width = Math.round(p * 100) + "%"; },
      done() { item.remove(); },
    };
  }

  async function handleFiles(files) {
    const list = Array.from(files).filter((f) => {
      const ext = (f.name.split(".").pop() || "").toLowerCase();
      return ACCEPT.includes(ext) || f.type.startsWith("image/");
    });
    if (!list.length) { toast("没有可上传的图片文件", "warn"); return; }
    const results = document.getElementById("results");
    for (const file of list) {
      const q = queueItem(file);
      try {
        const data = await uploadFile(file, q.set);
        q.done();
        (data.results || []).forEach((r) => {
          const wrap = document.createElement("div");
          wrap.innerHTML = resultCard(r);
          results.prepend(wrap.firstElementChild);
        });
        if (!data.results.some((r) => r.ok)) {
          toast(data.results[0]?.error || "上传失败", "error");
        } else {
          toast("上传成功");
        }
      } catch (err) {
        q.done();
        const wrap = document.createElement("div");
        wrap.innerHTML = resultCard({ ok: false, filename: file.name, error: err.message });
        results.prepend(wrap.firstElementChild);
      }
    }
    refreshQuota();
  }

  async function refreshQuota() {
    try {
      const s = await GET("/api/site");
      quotaMini(s.you);
    } catch {}
  }

  onReady(async function (site) {
    if (!window.R.requireAuth(site)) return;
    quotaMini(site.you);
    document.getElementById("dz-ico").innerHTML = icon("upload", 30);

    // tabs
    document.querySelectorAll(".tabs button").forEach((b) => {
      b.addEventListener("click", () => {
        document.querySelectorAll(".tabs button").forEach((x) => x.classList.toggle("active", x === b));
        const file = b.dataset.tab === "file";
        document.getElementById("tab-file").classList.toggle("hide", !file);
        document.getElementById("tab-url").classList.toggle("hide", file);
      });
    });

    // privacy segmented
    document.getElementById("privacy-seg").addEventListener("click", (e) => {
      const b = e.target.closest("button"); if (!b) return;
      opts.privacy = b.dataset.v;
      document.querySelectorAll("#privacy-seg button").forEach((x) => x.classList.toggle("active", x === b));
    });
    document.getElementById("opt-tags").addEventListener("input", (e) => opts.tags = e.target.value);
    document.getElementById("opt-expires").addEventListener("input", (e) => opts.expires = e.target.value);
    document.getElementById("opt-album").addEventListener("change", (e) => opts.album = e.target.value);

    // populate folder/album dropdown
    try {
      const albums = await GET("/api/albums");
      const sel = document.getElementById("opt-album");
      albums.forEach((a) => {
        const o = document.createElement("option");
        o.value = a.id;
        o.textContent = a.name + (a.privacy === "public" ? " · 公开" : "");
        sel.appendChild(o);
      });
      // pre-select if ?album= present (e.g. came from an album page)
      const pre = new URLSearchParams(location.search).get("album");
      if (pre) { sel.value = pre; opts.album = pre; }
    } catch {}

    // dropzone
    const dz = document.getElementById("dropzone");
    const input = document.getElementById("file-input");
    input.addEventListener("change", () => { if (input.files.length) handleFiles(input.files); input.value = ""; });
    ["dragenter", "dragover"].forEach((ev) => dz.addEventListener(ev, (e) => { e.preventDefault(); dz.classList.add("drag"); }));
    ["dragleave", "drop"].forEach((ev) => dz.addEventListener(ev, (e) => { e.preventDefault(); if (ev !== "drop" && dz.contains(e.relatedTarget)) return; dz.classList.remove("drag"); }));
    dz.addEventListener("drop", (e) => { if (e.dataTransfer.files.length) handleFiles(e.dataTransfer.files); });

    // paste
    window.addEventListener("paste", (e) => {
      const files = Array.from(e.clipboardData.files || []);
      if (files.length) handleFiles(files);
    });

    // url import
    document.getElementById("url-go").addEventListener("click", async () => {
      const url = document.getElementById("url-input").value.trim();
      if (!url) return;
      const btn = document.getElementById("url-go");
      btn.disabled = true; btn.innerHTML = '<span class="spinner"></span>';
      try {
        const data = await POST("/api/upload/url", { url, privacy: opts.privacy, tags: opts.tags, expires_hours: opts.expires, album_id: opts.album || null });
        const results = document.getElementById("results");
        const wrap = document.createElement("div");
        wrap.innerHTML = resultCard({ ok: true, image: data.image });
        results.prepend(wrap.firstElementChild);
        toast("导入成功");
        refreshQuota();
      } catch (err) {
        toast(err.message, "error");
      } finally {
        btn.disabled = false; btn.textContent = "导入";
      }
    });

    // copy result rows (delegated)
    document.getElementById("results").addEventListener("click", (e) => {
      const b = e.target.closest(".copy-row-btn");
      if (b) copy(b.dataset.text);
    });
  });
})();
