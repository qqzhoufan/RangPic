/* settings: profile + API tokens */
(function () {
  "use strict";
  const { onReady, GET, POST, DEL, icon, bytes, escapeHtml, ago, datetime, toast, modal } = window.R;

  onReady(function (site) {
    if (!window.R.requireAuth(site)) return;
    document.getElementById("tok-ico").innerHTML = icon("key", 17);

    // tabs
    document.querySelectorAll(".tabs button").forEach((b) => {
      b.addEventListener("click", () => {
        document.querySelectorAll(".tabs button").forEach((x) => x.classList.toggle("active", x === b));
        const profile = b.dataset.tab === "profile";
        document.getElementById("tab-profile").classList.toggle("hide", !profile);
        document.getElementById("tab-tokens").classList.toggle("hide", profile);
      });
    });

    // profile info
    const you = site.you;
    document.getElementById("profile-info").innerHTML = [
      ["用户名", escapeHtml(you.username), "user"],
      ["角色", you.role === "admin" ? "管理员" : "用户", "shield"],
      ["已用空间", bytes(you.used_bytes), "drive"],
      ["配额", bytes(you.quota_bytes), "shield"],
    ].map(([l, v, ico]) =>
      `<div class="stat"><div class="lbl">${l}</div><div class="num" style="font-size:1.2rem;font-family:var(--font-sans)">${ico ? icon(ico, 16) : ""} ${v}</div></div>`
    ).join("");

    // password
    document.getElementById("pw-form").addEventListener("submit", async (e) => {
      e.preventDefault();
      const fd = new FormData(e.target);
      if (fd.get("new") !== fd.get("new2")) { toast("两次新密码不一致", "error"); return; }
      try {
        await POST("/api/account/password", { current: fd.get("current"), new: fd.get("new") });
        toast("密码已更新，请重新登录");
        setTimeout(() => (location.href = "/login"), 1200);
      } catch (er) { toast(er.message, "error"); }
    });

    // tokens
    async function loadTokens() {
      try {
        const rows = await GET("/api/account/tokens");
        const tbody = document.querySelector("#tok-table tbody");
        if (!rows.length) {
          tbody.innerHTML = `<tr><td colspan="5" class="subtle center" style="padding:24px">还没有 Token</td></tr>`;
          return;
        }
        tbody.innerHTML = rows.map((t) => `
          <tr>
            <td><strong>${escapeHtml(t.name)}</strong></td>
            <td class="mono subtle">${escapeHtml(t.prefix)}…</td>
            <td class="subtle">${t.last_used_at ? ago(t.last_used_at) : "—"}</td>
            <td class="subtle">${datetime(t.created_at)}</td>
            <td><button class="btn btn-outline btn-sm revoke-btn" data-id="${t.id}">吊销</button></td>
          </tr>`).join("");
      } catch (e) { toast(e.message, "error"); }
    }

    document.getElementById("tok-form").addEventListener("submit", async (e) => {
      e.preventDefault();
      const name = new FormData(e.target).get("name");
      try {
        const res = await POST("/api/account/tokens", { name: name || undefined });
        e.target.reset();
        modal({
          title: "Token 已生成",
          body: `<p class="muted" style="margin-bottom:10px">请立即复制保存，关闭后将无法再次查看完整 Token。</p>
                 <div class="copy-row"><code class="code grow" id="newtok">${escapeHtml(res.token)}</code><button class="btn btn-primary" id="cpytok">${icon("copy", 16)} 复制</button></div>`,
          actions: [{ label: "完成", kind: "primary", onClick: ({ close }) => close() }],
        });
        document.getElementById("cpytok").addEventListener("click", () => window.R.copy(res.token));
        loadTokens();
      } catch (er) { toast(er.message, "error"); }
    });

    document.querySelector("#tok-table tbody").addEventListener("click", async (e) => {
      const b = e.target.closest(".revoke-btn"); if (!b) return;
      if (!await window.R.confirmDialog("确定吊销这个 Token？使用它的客户端将立即失效。", { danger: true, confirmLabel: "吊销" })) return;
      try { await DEL("/api/account/tokens/" + b.dataset.id); toast("已吊销"); loadTokens(); }
      catch (er) { toast(er.message, "error"); }
    });

    loadTokens();
  });
})();
