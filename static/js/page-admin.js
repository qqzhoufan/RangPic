/* admin: overview, users, image moderation, site settings */
(function () {
  "use strict";
  const { onReady, GET, PATCH, DEL, PUT, icon, bytes, num, escapeHtml, ago, datetime, toast, confirmDialog } = window.R;

  onReady(function (site) {
    if (!window.R.requireAuth(site)) return;
    if (site.you.role !== "admin") {
      document.querySelector("main").innerHTML = `<div class="empty">${icon("lock")}<h3>权限不足</h3><p>需要管理员账号才能访问。</p></div>`;
      return;
    }

    document.querySelectorAll(".tabs button").forEach((b) => {
      b.addEventListener("click", () => {
        document.querySelectorAll(".tabs button").forEach((x) => x.classList.toggle("active", x === b));
        ["overview", "users", "images", "site"].forEach((t) =>
          document.getElementById("tab-" + t).classList.toggle("hide", t !== b.dataset.tab));
        if (b.dataset.tab === "users") loadUsers();
        if (b.dataset.tab === "images") loadImages();
        if (b.dataset.tab === "site") loadSite();
      });
    });

    async function loadOverview() {
      try {
        const s = await GET("/api/admin/stats");
        document.getElementById("a-stats").innerHTML = [
          ["用户", num(s.users), "users"],
          ["图片", num(s.images), "image"],
          ["存储占用", bytes(s.storage_bytes), "drive"],
          ["总浏览", num(s.views), "eye"],
          ["随机图池", num(s.public_pool), "shuffle"],
          ["24h 上传", num(s.uploads_24h), "upload"],
        ].map(([l, v, ico]) =>
          `<div class="stat"><div class="num">${v}</div><div class="lbl">${ico ? icon(ico, 14) + " " : ""}${l}</div></div>`).join("");
        document.getElementById("a-source").innerHTML = (s.by_source || []).map((x) => `
          <div class="feature-note"><div class="ico">${icon(x.source === "upload" ? "upload" : "link", 18)}</div>
            <div><h3>${{ upload: "本地上传", remote: "远程链接", local: "本地素材" }[x.source] || x.source}</h3>
            <p>${num(x.count)} 张 · ${bytes(x.bytes)}</p></div></div>`).join("");
      } catch (e) { toast(e.message, "error"); }
    }

    async function loadUsers() {
      try {
        const users = await GET("/api/admin/users");
        document.querySelector("#users-table tbody").innerHTML = users.map((u) => `
          <tr data-id="${u.id}">
            <td><div class="row"><span class="avatar">${escapeHtml(u.username.slice(0, 1).toUpperCase())}</span><strong>${escapeHtml(u.username)}</strong>${u.role === "admin" ? '<span class="badge role">管理员</span>' : ""}</div></td>
            <td><select class="select u-role" style="padding:6px 8px">
              <option value="user" ${u.role === "user" ? "selected" : ""}>用户</option>
              <option value="admin" ${u.role === "admin" ? "selected" : ""}>管理员</option></select></td>
            <td class="mono subtle" style="white-space:nowrap">${bytes(u.used_bytes)} / <input class="input u-quota" type="number" min="0" value="${u.quota_bytes > 0 ? Math.round(u.quota_bytes / 1048576) : 0}" title="单位 MB，0 表示不限" style="width:90px;padding:5px 8px;display:inline"> MB</td>
            <td><label class="checkrow" style="margin:0"><input type="checkbox" class="u-banned" ${u.banned ? "checked" : ""}> <span>${u.banned ? "已封禁" : "正常"}</span></label></td>
            <td class="subtle">${datetime(u.created_at)}</td>
            <td class="row">
              <button class="btn btn-outline btn-sm u-save">保存</button>
              <button class="iconbtn u-del" title="删除">${icon("trash", 16)}</button>
            </td>
          </tr>`).join("");
      } catch (e) { toast(e.message, "error"); }
    }

    document.querySelector("#users-table tbody").addEventListener("click", async (e) => {
      const tr = e.target.closest("tr"); if (!tr) return;
      const id = tr.dataset.id;
      if (e.target.closest(".u-save")) {
        const body = {
          role: tr.querySelector(".u-role").value,
          quota_bytes: Math.round((parseInt(tr.querySelector(".u-quota").value, 10) || 0) * 1048576),
          banned: tr.querySelector(".u-banned").checked,
        };
        try { await PATCH("/api/admin/users/" + id, body); toast("已保存"); loadUsers(); }
        catch (er) { toast(er.message, "error"); }
      } else if (e.target.closest(".u-del")) {
        if (await confirmDialog("删除该用户？其上传的图片将保留但失去归属。", { danger: true, confirmLabel: "删除" })) {
          try { await DEL("/api/admin/users/" + id); toast("已删除"); loadUsers(); }
          catch (er) { toast(er.message, "error"); }
        }
      }
    });
    // live-update banned label
    document.querySelector("#users-table tbody").addEventListener("change", (e) => {
      if (e.target.classList.contains("u-banned")) {
        e.target.nextElementSibling.textContent = e.target.checked ? "已封禁" : "正常";
      }
    });

    const imgState = { page: 1, limit: 48 };
    async function loadImages() {
      try {
        const res = await GET("/api/admin/images?limit=" + imgState.limit + "&page=" + imgState.page);
        const grid = document.getElementById("a-grid");
        grid.innerHTML = res.items.map((it) => `
          <div class="img-card" data-id="${it.id}">
            <img src="${it.thumb || it.url}" alt="" loading="lazy">
            <div class="top-overlay"><span class="badge ${it.privacy}">${it.privacy}</span></div>
            <div class="overlay"><div class="name">${escapeHtml(it.original_name || "image")}</div>
              <div class="meta"><span>${it.owner_name ? escapeHtml(it.owner_name) : "无归属"}</span><span>${ago(it.created_at)}</span></div></div>
          </div>`).join("");
        const pages = Math.ceil(res.total / imgState.limit) || 1;
        const pager = document.getElementById("a-pager");
        if (pages <= 1) { pager.innerHTML = ""; return; }
        let h = "";
        for (let p = 1; p <= pages; p++) h += `<button class="${p === imgState.page ? "active" : ""}" data-p="${p}">${p}</button>`;
        pager.innerHTML = h;
      } catch (e) { toast(e.message, "error"); }
    }
    document.getElementById("a-pager").addEventListener("click", (e) => {
      const b = e.target.closest("button[data-p]"); if (!b) return;
      imgState.page = +b.dataset.p; loadImages();
    });
    document.getElementById("a-grid").addEventListener("click", async (e) => {
      const c = e.target.closest(".img-card"); if (!c) return;
      if (await confirmDialog("删除这张图片？文件与记录都会移除。", { danger: true, confirmLabel: "删除" })) {
        try { await DEL("/api/admin/images/" + c.dataset.id); toast("已删除"); loadImages(); }
        catch (er) { toast(er.message, "error"); }
      }
    });

    async function loadSite() {
      try {
        const s = await GET("/api/admin/settings");
        document.getElementById("s-url").value = s.site_url || "";
        document.getElementById("s-name").value = s.site_name || "";
        document.getElementById("s-desc").value = s.site_description || "";
        document.getElementById("s-blurb").value = s.index_blurb || "";
        const qb = parseInt(s.default_quota_bytes, 10);
        document.getElementById("s-quota").value = (isNaN(qb) || qb <= 0) ? 0 : Math.round(qb / 1048576);
        document.getElementById("s-reg").checked = s.registration_open === "true";
      } catch (e) { toast(e.message, "error"); }
    }
    document.getElementById("s-save").addEventListener("click", async () => {
      const body = {
        site_url: document.getElementById("s-url").value.trim(),
        site_name: document.getElementById("s-name").value,
        site_description: document.getElementById("s-desc").value,
        index_blurb: document.getElementById("s-blurb").value,
        default_quota_bytes: String(Math.round((parseInt(document.getElementById("s-quota").value, 10) || 0) * 1048576)),
        registration_open: document.getElementById("s-reg").checked ? "true" : "false",
      };
      try { await PUT("/api/admin/settings", body); toast("已保存"); }
      catch (e) { toast(e.message, "error"); }
    });

    loadOverview();
  });
})();
