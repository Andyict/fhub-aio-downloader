<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";

  let step = $state<1 | 2>(1);
  let loading = $state(false);
  let status = $state("");
  let showAdminPassword = $state(false);
  let showAdminConfirmPassword = $state(false);
  let showFsharePassword = $state(false);
  let uiLanguage = $state<"vi" | "en">("vi");

  let adminUsername = $state("");
  let adminPassword = $state("");
  let adminConfirm = $state("");
  let fshareEmail = $state("");
  let fsharePassword = $state("");
  let downloadPath = $state("/downloads");
  let maxConcurrent = $state(6);

  onMount(async () => {
    uiLanguage = navigator.language.toLowerCase().startsWith("vi") ? "vi" : "en";
    try {
      const [setupRes, adminRes] = await Promise.all([
        fetch("/api/setup/status"),
        fetch("/api/auth/setup-status"),
      ]);
      const setup = setupRes.ok ? await setupRes.json() : { complete: false };
      const admin = adminRes.ok ? await adminRes.json() : { setup_required: true };
      if (setup.complete && !admin.setup_required) {
        goto("/discover");
      }
      if (!admin.setup_required) {
        step = 2;
        status = "";
      }
    } catch {
      status = "";
    }
  });

  async function createAdmin() {
    if (!adminUsername.trim() || adminPassword.length < 6) {
      status = "Username không được trống và mật khẩu admin cần ít nhất 6 ký tự.";
      return;
    }
    if (adminPassword !== adminConfirm) {
      status = "Mật khẩu xác nhận không khớp.";
      return;
    }

    loading = true;
    status = "Đang tạo tài khoản quản trị...";
    try {
      const res = await fetch("/api/auth/setup-admin", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          username: adminUsername.trim(),
          password: adminPassword,
          confirm_password: adminConfirm,
        }),
      });
      if (!res.ok && res.status !== 409) throw new Error(await safeText(res));

      await loginAdmin();
      step = 2;
      status = "";
    } catch (err) {
      status = `Tạo admin thất bại: ${messageOf(err)}`;
    } finally {
      loading = false;
    }
  }

  async function loginAdmin() {
    const res = await fetch("/api/auth/login", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "include",
      body: JSON.stringify({ username: adminUsername.trim(), password: adminPassword }),
    });
    if (!res.ok) throw new Error("Đã tạo admin nhưng chưa đăng nhập được.");
  }

  async function finishSetup() {
    loading = true;
    status = "Đang lưu cấu hình FHUB...";
    try {
      const payload: {
        downloads: { directory: string; max_concurrent: number };
        fshare?: { email: string; password: string };
      } = {
        downloads: {
          directory: downloadPath.trim() || "/downloads",
          max_concurrent: Number(maxConcurrent) || 6,
        },
      };

      if (fshareEmail.trim() && fsharePassword) {
        payload.fshare = { email: fshareEmail.trim(), password: fsharePassword };
      }

      const res = await fetch("/api/setup/complete", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify(payload),
      });
      if (!res.ok) throw new Error(await safeText(res));
      const result = await res.json();
      if (!result.success) throw new Error(result.message || "Không hoàn tất được setup.");
      if (fshareEmail.trim() && fsharePassword) {
        // Không chặn lần vào admin đầu tiên bằng verify FShare.
        // FShare có thể rate-limit/khóa 10 phút nếu sai mật khẩu; verify sẽ chạy nền,
        // còn trạng thái sai mật khẩu/UNVERIFIED xử lý trong Settings.
        void refreshFshareVip(fshareEmail.trim());
      }
      status = "Đang kiểm tra quyền admin...";
      await waitForCurrentUser();
      window.location.replace("/discover");
    } catch (err) {
      status = `Lưu cấu hình thất bại: ${messageOf(err)}`;
    } finally {
      loading = false;
    }
  }



  async function waitForCurrentUser() {
    const started = Date.now();
    for (let i = 0; i < 12; i += 1) {
      try {
        const response = await fetch("/api/auth/me", { credentials: "include" });
        if (response.ok) {
          const payload = await response.json();
          if (payload?.authenticated && payload?.user?.role) break;
        }
      } catch {
        // retry briefly
      }
      await new Promise((resolve) => setTimeout(resolve, 220));
    }
    const wait = Math.max(0, 850 - (Date.now() - started));
    if (wait) await new Promise((resolve) => setTimeout(resolve, wait));
  }

  async function refreshFshareVip(email: string): Promise<{ success: boolean; message?: string }> {
    try {
      const response = await fetch("/api/accounts", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ email, password: fsharePassword }),
      });
      if (!response.ok) return { success: false, message: `FShare lỗi HTTP ${response.status}` };
      const payload = await response.json();
      const message = payload?.message || "";
      if (/wrong password|invalid credential|authenticate fail|invalid credentials|login failed|sai mật khẩu|mật khẩu/i.test(message)) {
        return { success: false, message };
      }
      return { success: payload?.success !== false, message };
    } catch (error) {
      return { success: false, message: error instanceof Error ? error.message : "Không xác minh được FShare." };
    }
  }

  async function safeText(res: Response) {
    const text = await res.text();
    return text || `HTTP ${res.status}`;
  }

  function messageOf(err: unknown) {
    return err instanceof Error ? err.message : "lỗi không xác định";
  }
</script>

<div class="setup-screen">
  <section class="setup-card">
    <div class="brand brand-image" aria-label="FHub Movie Hub">
      <img src="/images/fhub-intro-logo.png" alt="FHub" />
      {#if step !== 1}<span>{uiLanguage === "vi" ? "ĐĂNG NHẬP FSHARE" : "FSHARE LOGIN"}</span>{/if}
    </div>
    {#if status}<p class:error={status.includes("thất bại") || status.includes("không") || status.includes("lỗi")}>{status}</p>{/if}

    {#if step === 1}
      <form onsubmit={(event) => { event.preventDefault(); createAdmin(); }}>
        <label class="field"><span class="material-icons">person</span><input bind:value={adminUsername} autocomplete="username" placeholder={uiLanguage === "vi" ? "Tài khoản" : "Account"} /></label>
        <label class="field"><span class="material-icons">lock</span><input bind:value={adminPassword} type={showAdminPassword ? "text" : "password"} autocomplete="new-password" placeholder={uiLanguage === "vi" ? "Mật khẩu" : "Password"} /><button type="button" class="icon-btn" aria-label={showAdminPassword ? "Ẩn mật khẩu" : "Hiện mật khẩu"} onclick={() => showAdminPassword = !showAdminPassword}><span class="material-icons">{showAdminPassword ? "visibility_off" : "visibility"}</span></button></label>
        <label class="field"><span class="material-icons">verified_user</span><input bind:value={adminConfirm} type={showAdminConfirmPassword ? "text" : "password"} autocomplete="new-password" placeholder={uiLanguage === "vi" ? "Nhập lại mật khẩu" : "Confirm password"} /><button type="button" class="icon-btn" aria-label={showAdminConfirmPassword ? "Ẩn mật khẩu" : "Hiện mật khẩu"} onclick={() => showAdminConfirmPassword = !showAdminConfirmPassword}><span class="material-icons">{showAdminConfirmPassword ? "visibility_off" : "visibility"}</span></button></label>
        <button class="submit" type="submit" disabled={loading}>{loading ? (uiLanguage === "vi" ? "Đang tạo..." : "Creating...") : (uiLanguage === "vi" ? "Tiếp tục" : "Continue")}</button>
      </form>
    {:else}
      <form onsubmit={(event) => { event.preventDefault(); finishSetup(); }}>
        <div class="form-grid">
          <label class="field"><span class="material-icons">alternate_email</span><input bind:value={fshareEmail} type="email" autocomplete="email" placeholder={uiLanguage === "vi" ? "Tài khoản FShare" : "FShare account"} /></label>
          <label class="field"><span class="material-icons">lock</span><input bind:value={fsharePassword} type={showFsharePassword ? "text" : "password"} autocomplete="current-password" placeholder={uiLanguage === "vi" ? "Mật khẩu" : "Password"} /><button type="button" class="icon-btn" aria-label={showFsharePassword ? "Ẩn mật khẩu" : "Hiện mật khẩu"} onclick={() => showFsharePassword = !showFsharePassword}><span class="material-icons">{showFsharePassword ? "visibility_off" : "visibility"}</span></button></label>
        </div>
        <div class="actions">
          <button class="submit" type="submit" disabled={loading}>{loading ? (uiLanguage === "vi" ? "Đang lưu..." : "Saving...") : (uiLanguage === "vi" ? "Đăng nhập" : "Login")}</button>
          <button class="secondary-btn" type="button" onclick={() => { fshareEmail = ""; fsharePassword = ""; finishSetup(); }} disabled={loading}>{uiLanguage === "vi" ? "Bỏ qua" : "Skip"}</button>
        </div>
      </form>
    {/if}
  </section>
</div>

<style>
  :global(body){margin:0;background:#050505;color:#fff8eb;font-family:Inter,system-ui,sans-serif;}
  .setup-screen{min-height:100dvh;display:grid;place-items:center;padding:1.2rem;background:radial-gradient(circle at 50% -8%,rgba(244,181,68,.18),transparent 32%),linear-gradient(180deg,#050505,#0d0906 52%,#050505)}
  .setup-card{width:min(100%,420px);display:grid;gap:1.05rem;padding:2rem;border-radius:34px;background:linear-gradient(180deg,rgba(25,22,18,.92),rgba(8,8,8,.9));border:1px solid rgba(244,181,68,.18);box-shadow:0 30px 90px rgba(0,0,0,.58);backdrop-filter:blur(20px)}
  .brand{display:grid;justify-items:center;text-align:center;margin-bottom:.24rem}.brand-image{gap:.34rem}.brand-image img{width:min(360px,100%);height:auto;object-fit:contain;filter:drop-shadow(0 0 30px rgba(244,181,68,.28))}.brand-image span{color:rgba(203,191,255,.9);font-size:.86rem;font-weight:950;letter-spacing:.38em;text-align:center}
  p{margin:0;padding:.65rem .75rem;border:1px solid rgba(244,181,68,.18);border-radius:14px;color:#ffd37a;background:rgba(244,181,68,.08);font-size:.86rem;line-height:1.35}p.error{color:#fecaca;border-color:rgba(248,113,113,.25);background:rgba(127,29,29,.16)}
  form{display:grid;gap:.85rem}.form-grid{display:grid;gap:.85rem}.field{display:flex;align-items:center;gap:.7rem;min-height:52px;padding:0 .85rem;border-radius:16px;background:rgba(5,5,5,.72);border:1px solid rgba(244,181,68,.14)}.material-icons{color:#f4b544}.field input{flex:1;min-width:0;border:0;outline:0;background:transparent;color:#fff8eb;font-size:1rem}.field input::placeholder{color:rgba(255,248,235,.46)}.icon-btn{border:0;background:transparent;padding:0;color:#f4b544;display:grid;place-items:center}
  .actions{display:grid;grid-template-columns:1fr auto;gap:.75rem}.submit{min-height:52px;border:0;border-radius:16px;background:linear-gradient(180deg,#ffd37a,#d99129);color:#130b04;font-weight:950;font-size:1rem}.secondary-btn{min-height:52px;border-radius:16px;padding:0 1rem;border:1px solid rgba(244,181,68,.16);color:#fff8eb;background:rgba(5,5,5,.55);font-weight:900}button:disabled{opacity:.65;cursor:wait}
  @media(max-width:520px){.setup-screen{padding:1rem;place-items:center}.setup-card{padding:1.2rem;border-radius:28px}.brand-image img{width:min(320px,100%)}.brand-image span{font-size:.74rem;letter-spacing:.28em}.actions{grid-template-columns:1fr}.secondary-btn{width:100%}}
</style>
