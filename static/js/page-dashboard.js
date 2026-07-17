/* dashboard */
(function () {
  "use strict";
  const { onReady, GET, icon, bytes, num, escapeHtml, ago } = window.R;

  onReady(async function (site) {
    if (!window.R.requireAuth(site)) return;
    document.getElementById("up-ico").innerHTML = icon("upload", 17);
    const greet = document.getElementById("greet");
    greet.textContent = `你好，${site.you.username}`;

    try {
      const s = await GET("/api/my/stats");
      const stats = document.getElementById("stats");
      stats.innerHTML = [
        ["图片数量", num(s.count), "image", "已上传的图片总数"],
        ["已用空间", bytes(s.used_bytes), "drive", "原始文件占用"],
        ["总浏览", num((s.by_privacy || []).reduce((a, b) => a + 0, 0)), "eye", "本账号统计"],
        ["配额", bytes(s.quota_bytes), "shield", "可使用的存储上限"],
      ].map(([lbl, val, ico, sub]) =>
        `<div class="stat"><div class="num">${val}</div><div class="lbl">${lbl}</div><div class="sub">${sub}</div></div>`
      ).join("");

      const pct = s.quota_bytes > 0 ? Math.min(100, (s.used_bytes / s.quota_bytes) * 100) : 0;
      const meter = document.getElementById("quota-meter");
      meter.classList.remove("warn", "danger");
      if (pct > 90) meter.classList.add("danger");
      else if (pct > 75) meter.classList.add("warn");
      meter.querySelector("i").style.width = pct + "%";
      document.getElementById("quota-text").textContent = `${bytes(s.used_bytes)} / ${bytes(s.quota_bytes)}（${pct.toFixed(0)}%）`;
    } catch {}

    try {
      const res = await GET("/api/my/images?limit=8");
      const el = document.getElementById("recent");
      if (!res.items.length) {
        el.outerHTML = `<div class="empty">${icon("image")}<h3>还没有图片</h3><p>上传你的第一张图片吧</p><a class="btn btn-primary" href="/upload" style="margin-top:12px">去上传</a></div>`;
        return;
      }
      el.innerHTML = res.items.map((it) => `
        <a class="img-card card-hover" href="${it.share || it.links.share}">
          <img src="${it.thumb || it.url}" alt="${escapeHtml(it.original_name)}" loading="lazy">
          <div class="overlay"><div class="name">${escapeHtml(it.original_name || "image")}</div>
            <div class="meta"><span>${ago(it.created_at)}</span><span>${icon("eye",12)} ${num(it.views)}</span></div></div>
        </a>`).join("");
    } catch {}
  });
})();
