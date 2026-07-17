/* landing page */
(function () {
  "use strict";
  const { onReady, GET, icon, copy, escapeHtml, ago } = window.R;

  onReady(async function () {
    // feature icons
    const setIco = (id, key) => { const e = document.getElementById(id); if (e) e.innerHTML = icon(key, 18); };
    setIco("ico-shuffle", "shuffle");
    setIco("ico-upload", "upload");
    setIco("ico-layers", "layers");
    setIco("ico-key", "key");

    let currentTag = "";

    async function loadHero(tag, attempt = 0) {
      const img = document.getElementById("hero-img");
      try {
        const url = "/api/random-image" + (tag ? "?tags=" + encodeURIComponent(tag) : "");
        const data = await GET(url);
        const tags = document.getElementById("hero-tags");
        tags.innerHTML = (data.tags && data.tags.length)
          ? data.tags.slice(0, 4).map((t) => `<span class="tag">${escapeHtml(t)}</span>`).join("")
          : '<span class="subtle" style="font-size:.82rem">无标签</span>';
        const share = document.getElementById("hero-share");
        if (share) share.href = data.share || data.links.share;
        setLinks(data.links);
        // attach error handler before setting src: if the chosen image is a
        // dead remote entry, silently retry with another random pick.
        img.alt = "随机图片";
        img.onerror = () => {
          if (attempt < 5) {
            loadHero(tag, attempt + 1);
          } else {
            img.removeAttribute("src");
            img.alt = "图片加载失败";
          }
        };
        img.src = data.url;
      } catch {
        img.removeAttribute("src");
        img.alt = "暂无图片";
      }
    }

    function setLinks(links) {
      if (!links) return;
      document.getElementById("lnk-direct").textContent = links.direct;
      document.getElementById("lnk-md").textContent = links.markdown;
      document.getElementById("lnk-html").textContent = links.html;
    }

    document.getElementById("hero-refresh").addEventListener("click", () => loadHero(currentTag));

    // tag rail
    try {
      const tags = await GET("/api/tags");
      const rail = document.getElementById("tag-rail");
      rail.innerHTML = tags.slice(0, 18).map((t) =>
        `<button class="tag btn" data-tag="${escapeHtml(t)}">${icon("tag", 13)}${escapeHtml(t)}</button>`
      ).join("");
      rail.addEventListener("click", (e) => {
        const b = e.target.closest(".tag.btn");
        if (!b) return;
        currentTag = b.dataset.tag;
        rail.querySelectorAll(".tag.btn").forEach((x) => x.classList.toggle("active", x === b));
        loadHero(currentTag);
        document.getElementById("api-sample").textContent = "/random-image?tags=" + currentTag;
      });
    } catch {}

    // recent strip
    try {
      const res = await GET("/api/explore?limit=10");
      const el = document.getElementById("recent");
      el.innerHTML = (res.items || []).map((it) => `
        <a class="img-card card-hover" href="${it.share || it.links.share}">
          <img src="${it.thumb || it.url}" alt="${escapeHtml(it.original_name)}" loading="lazy">
          <div class="overlay">
            <div class="name">${escapeHtml(it.original_name || "image")}</div>
            <div class="meta"><span>${ago(it.created_at)}</span>${it.views ? `<span>${icon("eye",12)} ${it.views}</span>` : ""}</div>
          </div>
        </a>`).join("");
    } catch {}

    loadHero("");

    // copy buttons
    document.getElementById("copy-api").addEventListener("click", () =>
      copy(document.getElementById("api-sample").textContent));
    document.body.addEventListener("click", (e) => {
      const b = e.target.closest(".copy-btn");
      if (b) {
        const t = document.getElementById(b.dataset.target);
        if (t) copy(t.textContent);
      }
    });
  });
})();
