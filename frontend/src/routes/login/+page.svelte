<script lang="ts">
  import { goto } from "$app/navigation";
  import { toasts } from "$lib/stores/toasts";

  let username = $state("");
  let password = $state("");
  let loading = $state(false);
  let loadingText = $state("Đăng nhập");
  let loginError = $state("");
  let showPassword = $state(false);

  async function handleSubmit() {
    loading = true;
    loginError = "";
    loadingText = "Đang đăng nhập...";
    try {
      const res = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ username, password }),
      });
      if (!res.ok) {
        if (res.status === 401 || res.status === 403) throw new Error("Sai tài khoản hoặc mật khẩu.");
        throw new Error("Đăng nhập thất bại.");
      }

      loadingText = "Đang đồng bộ tài khoản...";
      const started = Date.now();
      const me = await fetch("/api/auth/me", { credentials: "include" });
      if (!me.ok) throw new Error("Chưa đọc được thông tin tài khoản");
      await me.json();
      const wait = Math.max(0, 850 - (Date.now() - started));
      if (wait) await new Promise((resolve) => setTimeout(resolve, wait));
      window.location.replace("/discover");
    } catch (err) {
      const message = err instanceof Error ? err.message : "Đăng nhập thất bại.";
      loginError = message;
      toasts.error(message);
      loadingText = "Đăng nhập";
      loading = false;
    }
  }
</script>

<div class="auth-screen">
  <section class="auth-card">
    <div class="brand brand-image" aria-label="FHub Movie Hub">
      <img src="/images/logo5.png" alt="FHub" />
      <span>ĐĂNG NHẬP</span>
    </div>
    <form onsubmit={(event) => { event.preventDefault(); handleSubmit(); }}>
      <label><span class="material-icons">person</span><input bind:value={username} placeholder="Tài khoản" autocomplete="username" /></label>
      <label><span class="material-icons">lock</span><input bind:value={password} type={showPassword ? "text" : "password"} placeholder="Mật khẩu" autocomplete="current-password" /><button type="button" aria-label={showPassword ? "Ẩn mật khẩu" : "Hiện mật khẩu"} onclick={() => showPassword = !showPassword}><span class="material-icons">{showPassword ? "visibility_off" : "visibility"}</span></button></label>
      {#if loginError}<p class="login-error">{loginError}</p>{/if}
      <button class="submit" type="submit" disabled={loading}>{loading ? loadingText : "Đăng nhập"}</button>
    </form>
  </section>
</div>

<style>
  .auth-screen{min-height:100dvh;display:grid;place-items:center;padding:1.2rem;background:radial-gradient(circle at 50% -8%,rgba(244,181,68,.18),transparent 32%),linear-gradient(180deg,#050505,#0d0906 52%,#050505)}.auth-card{width:min(100%,420px);display:grid;gap:1.1rem;padding:2rem;border-radius:34px;background:linear-gradient(180deg,rgba(25,22,18,.92),rgba(8,8,8,.9));border:1px solid rgba(244,181,68,.18);box-shadow:0 30px 90px rgba(0,0,0,.58);backdrop-filter:blur(20px)}.brand{display:grid;justify-items:center;text-align:center;margin-bottom:.24rem}.brand-image{gap:.34rem}.brand-image img{width:min(350px,100%);height:auto;object-fit:contain;filter:drop-shadow(0 0 30px rgba(244,181,68,.28))}.brand-image span{color:rgba(255,255,255,.72);font-size:.86rem;font-weight:950;letter-spacing:.38em;text-align:center}form{display:grid;gap:.85rem}.login-error{margin:0;padding:.72rem .85rem;border-radius:14px;border:1px solid rgba(248,113,113,.28);background:rgba(127,29,29,.18);color:#fecaca;font-weight:850;font-size:.92rem;text-align:center;line-height:1.35}label{display:flex;align-items:center;gap:.7rem;min-height:52px;padding:0 .85rem;border-radius:16px;background:rgba(5,5,5,.72);border:1px solid rgba(244,181,68,.14)}.material-icons{color:#f4b544}input{flex:1;min-width:0;border:0;outline:0;background:transparent;color:#fff8eb}label button{border:0;background:transparent;padding:0}.submit{min-height:52px;border:0;border-radius:16px;background:linear-gradient(180deg,#ffd37a,#d99129);color:#130b04;font-weight:950}@media(max-width:520px){.auth-card{padding:1.2rem;border-radius:28px}.brand-image img{width:min(310px,100%)}.brand-image span{font-size:.74rem;letter-spacing:.28em}}
</style>