/* share page */
(function () {
  "use strict";
  const { onReady, GET, icon, bytes, num, escapeHtml, ago, datetime, copy, openLightbox } = window.R;

  function imageId() {
    const m = location.pathname.match(/\/([0-9a-fA-F-]{36})(?:\/view)?$/);
    return m ? m[1] : null;
  }

  onReady(async function () {
    const root = document.getElementById("share-root");
    const id = imageId();
    if (!id) {
      root.innerHTML = `<div class="empty">${icon("alert")}<h3>无效的链接</h3></div>`;
      return;
    }
    try {
      const it = await GET("/api/images/" + id);
      document.title = `${it.original_name || "图片"} — 咸鱼图床`;
      const tags = (it.tags && it.tags.length)
        ? it.tags.map((t) => `<a class="tag btn" href="/random-image?tags=${encodeURIComponent(t)}" target="_blank">${icon("tag", 13)}${escapeHtml(t)}</a>`).join(" ")
        : '<span class="subtle">无标签</span>';
      const expRow = it.expires_at
        ? `<div class="between" style="font-size:.86rem"><span class="muted">过期时间</span><span>${datetime(it.expires_at)}</span></div>` : "";
      const links = it.links;
      const linkRows = [["直链", links.direct], ["Markdown", links.markdown], ["HTML", links.html], ["BBCode", links.bbcode]]
        .map(([k, v]) => `<div class="copy-row"><span class="code" style="min-width:74px;text-align:center;font-size:.74rem;padding:9px 6px">${k}</span><code class="code grow">${escapeHtml(v)}</code><button class="btn btn-outline copyln" data-t="${escapeHtml(v)}">${icon("copy", 16)}</button></div>`)
        .join("");
      root.innerHTML = `
        <div class="row" style="margin-bottom:var(--s-4)">
          <a class="btn btn-ghost btn-sm" href="/explore">${icon("arrow", 16)} 返回发现</a>
          <span class="badge ${it.privacy}" style="margin-left:auto">${it.privacy}</span>
        </div>
        <div class="row" style="align-items:flex-start;flex-wrap:wrap;gap:var(--s-5)">
          <div class="share-stage" style="flex:1 1 520px;min-width:0;cursor:zoom-in" id="stage">
            <img src="${it.url}" alt="${escapeHtml(it.original_name)}">
          </div>
          <div style="flex:1 1 300px;min-width:280px" class="stack">
            <div class="card card-pad">
              <h2 class="serif" style="font-size:1.3rem;word-break:break-all">${escapeHtml(it.original_name || "未命名")}</h2>
              <div class="muted mono" style="font-size:.8rem;margin-top:4px">${escapeHtml(it.filename || it.id)}</div>
              <hr>
              <div class="stack" style="gap:10px;font-size:.88rem">
                <div class="between"><span class="muted">上传者</span>${it.owner_name ? `<a href="/random-image?user=${encodeURIComponent(it.owner_name)}" target="_blank" title="该用户的随机图">${escapeHtml(it.owner_name)}</a>` : "<span>匿名</span>"}</div>
                <div class="between"><span class="muted">尺寸</span><span>${it.width ? it.width + " × " + it.height : "—"}</span></div>
                <div class="between"><span class="muted">大小</span><span>${bytes(it.size_bytes)}</span></div>
                <div class="between"><span class="muted">格式</span><span class="mono">${escapeHtml(it.ext || it.content_type)}</span></div>
                <div class="between"><span class="muted">浏览</span><span>${icon("eye", 13)} ${num(it.views)}</span></div>
                <div class="between"><span class="muted">上传时间</span><span>${datetime(it.created_at)} · ${ago(it.created_at)}</span></div>
                ${expRow}
              </div>
            </div>
            <div class="card card-pad">
              <strong style="font-size:.9rem">标签</strong>
              <div class="row wrap" style="margin-top:10px;gap:6px">${tags}</div>
            </div>
            <div class="card card-pad">
              <strong style="font-size:.9rem;display:block;margin-bottom:12px">复制链接 / 嵌入代码</strong>
              <div class="stack" style="gap:6px">${linkRows}</div>
              <a class="btn btn-outline btn-block" style="margin-top:12px" href="${it.url}" download>${icon("download", 16)} 下载原图</a>
            </div>
          </div>
        </div>`;
      document.getElementById("stage").addEventListener("click", () => openLightbox(it.url));
      root.addEventListener("click", (e) => {
        const c = e.target.closest(".copyln"); if (c) copy(c.dataset.t);
      });
    } catch (e) {
      const msg = (e.status === 404) ? "图片不存在，或已被删除 / 设为私有。" : (e.message || "加载失败");
      root.innerHTML = `<div class="empty">${icon("image")}<h3>无法查看</h3><p>${escapeHtml(msg)}</p><a class="btn btn-outline" href="/" style="margin-top:12px">返回首页</a></div>`;
    }
  });
})();
