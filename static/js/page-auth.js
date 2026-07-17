/* login + register */
(function () {
  "use strict";
  const { onReady, POST, icon, toast } = window.R;
  const isRegister = location.pathname === "/register";

  onReady(function (site) {
    const mk = document.getElementById("mk");
    if (mk) mk.innerHTML = icon("fish", 16);

    // registration closed guard
    if (isRegister && site.registration_open === false) {
      document.querySelector(".auth-card").innerHTML =
        '<div class="center stack"><div>' + icon("lock", 34) + '</div><h2 class="serif">暂未开放注册</h2><p class="muted">管理员已关闭公开注册，请联系管理员获取账号。</p><a class="btn btn-outline" href="/login">返回登录</a></div>';
      return;
    }

    const form = document.getElementById("auth-form");
    const btn = document.getElementById("submit-btn");
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const fd = new FormData(form);
      const username = fd.get("username").trim();
      const password = fd.get("password");
      if (isRegister) {
        if (password !== fd.get("password2")) {
          toast("两次输入的密码不一致", "error");
          return;
        }
      }
      btn.disabled = true;
      const orig = btn.textContent;
      btn.innerHTML = '<span class="spinner"></span>';
      try {
        const endpoint = isRegister ? "/api/auth/register" : "/api/auth/login";
        await POST(endpoint, { username, password });
        const next = new URLSearchParams(location.search).get("next");
        location.href = next || "/dashboard";
      } catch (err) {
        toast(err.message || "登录失败", "error");
        btn.disabled = false;
        btn.textContent = orig;
      }
    });
  });
})();
