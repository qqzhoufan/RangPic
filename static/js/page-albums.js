/* albums */
(function () {
  "use strict";
  const { onReady, GET, POST, PATCH, DEL, icon, escapeHtml, modal, confirmDialog, toast } = window.R;

  async function load() {
    const list = document.getElementById("list");
    try {
      const albums = await GET("/api/albums");
      if (!albums.length) {
        list.className = "";
        list.innerHTML = `<div class="empty">${icon("folder")}<h3>还没有相册</h3><p>新建一个相册，开始整理你的图片</p></div>`;
        return;
      }
      list.className = "split";
      list.innerHTML = albums.map((a) => `
        <div class="card card-hover card-pad" data-id="${a.id}">
          <div class="between">
            <a class="row" href="/gallery?album=${a.id}" style="color:var(--text);gap:10px">
              <span style="color:var(--accent)">${icon("folder", 22)}</span>
              <strong>${escapeHtml(a.name)}</strong>
            </a>
            <span class="badge ${a.privacy}">${a.privacy}</span>
          </div>
          <a href="/gallery?album=${a.id}" class="img-card" style="margin-top:12px;aspect-ratio:16/9;display:grid;place-items:center;background:var(--surface-2)">
            ${a.cover_id ? `<img src="/t/${a.cover_id}" alt="" style="width:100%;height:100%;object-fit:cover">` : `<span style="color:var(--text-subtle)">${icon("image", 28)}</span>`}
          </a>
          <div class="between" style="margin-top:12px;font-size:.85rem">
            <span class="muted">${a.image_count} 张图片</span>
            <div class="row">
              <a class="iconbtn" href="/random-image?album=${a.id}" target="_blank" title="从该文件夹随机一张">${icon("shuffle", 17)}</a>
              <a class="iconbtn" href="/gallery?album=${a.id}" title="查看图片">${icon("gallery", 17)}</a>
              <button class="iconbtn edit-btn" data-id="${a.id}" title="编辑">${icon("edit", 17)}</button>
              <button class="iconbtn del-btn" data-id="${a.id}" title="删除">${icon("trash", 17)}</button>
            </div>
          </div>
        </div>`).join("");
    } catch (e) { toast(e.message, "error"); }
  }

  function editDialog(album) {
    const creating = !album;
    const m = modal({
      title: creating ? "新建相册" : "编辑相册",
      body: `
        <div class="stack">
          <div class="field"><label>名称</label><input class="input" id="a-name" value="${escapeHtml(album ? album.name : "")}"></div>
          <div class="field"><label>描述</label><textarea class="textarea" id="a-desc">${escapeHtml(album ? album.description : "")}</textarea></div>
          <div class="field"><label>可见性</label>
            <div class="segmented" id="a-privacy">
              ${["public", "unlisted", "private"].map((v) =>
                `<button data-v="${v}" class="${(!album && v === "unlisted") || (album && album.privacy === v) ? "active" : ""}">${{ public: "公开", unlisted: "不公开", private: "私有" }[v]}</button>`
              ).join("")}
            </div></div>
        </div>`,
      actions: [
        { label: creating ? "创建" : "保存", kind: "primary", onClick: async ({ close }) => {
          const body = {
            name: m.root.querySelector("#a-name").value,
            description: m.root.querySelector("#a-desc").value,
            privacy: m.root.querySelector("#a-privacy .active").dataset.v,
          };
          if (!body.name.trim()) { toast("请填写相册名称", "error"); return; }
          try {
            if (creating) await POST("/api/albums", body);
            else await PATCH("/api/albums/" + album.id, body);
            toast(creating ? "已创建" : "已保存"); close(); load();
          } catch (e) { toast(e.message, "error"); }
        } },
        { label: "取消", kind: "outline", onClick: ({ close }) => close() },
      ],
    });
    m.root.querySelector("#a-privacy").addEventListener("click", (e) => {
      const b = e.target.closest("button"); if (!b) return;
      m.root.querySelectorAll("#a-privacy button").forEach((x) => x.classList.toggle("active", x === b));
    });
  }

  onReady(async function (site) {
    if (!window.R.requireAuth(site)) return;
    document.getElementById("new-ico").innerHTML = icon("plus", 17);
    document.getElementById("new-btn").addEventListener("click", () => editDialog(null));

    const list = document.getElementById("list");
    let albumsCache = [];
    try { albumsCache = await GET("/api/albums"); } catch {}

    list.addEventListener("click", async (e) => {
      const ed = e.target.closest(".edit-btn");
      const dl = e.target.closest(".del-btn");
      if (ed) {
        const a = albumsCache.find((x) => x.id === ed.dataset.id);
        if (a) editDialog(a);
      } else if (dl) {
        const a = albumsCache.find((x) => x.id === dl.dataset.id);
        if (a && await confirmDialog(`删除相册「${a.name}」？相册内的图片不会被删除（会变为未归类）。`, { danger: true, confirmLabel: "删除" })) {
          try { await DEL("/api/albums/" + a.id); toast("已删除"); load(); albumsCache = await GET("/api/albums"); } catch (er) { toast(er.message, "error"); }
        }
      }
    });

    load();
  });
})();
