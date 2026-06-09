<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { toasts } from "$lib/stores/toasts";

  type Role = "admin" | "user";
  type AppUser = {
    id: string;
    username: string;
    role: Role;
    is_active: boolean;
    created_at: string;
    updated_at: string;
    last_login_at?: string | null;
  };

  let loading = $state(true);
  let saving = $state(false);
  let users = $state<AppUser[]>([]);
  let me = $state<AppUser | null>(null);

  let newUsername = $state("");
  let newPassword = $state("");
  let newRole = $state<Role>("user");

  let resetPasswords = $state<Record<string, string>>({});

  function isSelf(user: AppUser) {
    return !!me && user.id === me.id;
  }

  async function fetchMe() {
    const res = await fetch("/api/auth/me", { credentials: "include" });
    if (!res.ok) {
      goto("/login");
      return null;
    }
    const data = await res.json();
    return data.user as AppUser;
  }

  async function fetchUsers() {
    const res = await fetch("/api/auth/users", { credentials: "include" });
    if (res.status === 403) {
      toasts.error("Bạn không có quyền admin");
      goto("/");
      return;
    }
    if (!res.ok) {
      throw new Error("load users failed");
    }
    const data = await res.json();
    users = data.users || [];
  }

  onMount(async () => {
    try {
      me = await fetchMe();
      if (!me) return;
      if (me.role !== "admin") {
        toasts.error("Trang này chỉ dành cho admin");
        goto("/");
        return;
      }
      await fetchUsers();
    } catch (e) {
      toasts.error("Không tải được danh sách tài khoản");
    } finally {
      loading = false;
    }
  });

  async function createUser() {
    if (!newUsername.trim() || newPassword.trim().length < 6) {
      toasts.error("Username cần có và password tối thiểu 6 ký tự");
      return;
    }
    saving = true;
    try {
      const res = await fetch("/api/auth/users", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          username: newUsername.trim(),
          password: newPassword,
          role: newRole,
        }),
      });
      if (!res.ok) {
        toasts.error("Tạo tài khoản thất bại hoặc username đã tồn tại");
        return;
      }
      toasts.success("Đã tạo tài khoản");
      newUsername = "";
      newPassword = "";
      newRole = "user";
      await fetchUsers();
    } catch (_) {
      toasts.error("Lỗi kết nối khi tạo tài khoản");
    } finally {
      saving = false;
    }
  }

  async function updateUser(user: AppUser, patch: Record<string, unknown>, successText: string) {
    if (isSelf(user) && patch.is_active === false) {
      toasts.error("Không thể tự khóa tài khoản đang đăng nhập");
      return;
    }
    try {
      const res = await fetch(`/api/auth/users/${user.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify(patch),
      });
      if (!res.ok) {
        toasts.error("Cập nhật tài khoản thất bại");
        return;
      }
      toasts.success(successText);
      await fetchUsers();
    } catch (_) {
      toasts.error("Lỗi kết nối khi cập nhật tài khoản");
    }
  }

  async function resetPassword(user: AppUser) {
    const password = resetPasswords[user.id]?.trim() || "";
    if (password.length < 6) {
      toasts.error("Password mới tối thiểu 6 ký tự");
      return;
    }
    await updateUser(user, { password }, `Đã reset password cho ${user.username}`);
    resetPasswords = { ...resetPasswords, [user.id]: "" };
  }

  async function removeUser(user: AppUser) {
    if (isSelf(user)) {
      toasts.error("Không thể xóa tài khoản đang đăng nhập");
      return;
    }
    if (!confirm(`Xóa tài khoản ${user.username}?`)) return;
    try {
      const res = await fetch(`/api/auth/users/${user.id}`, {
        method: "DELETE",
        credentials: "include",
      });
      if (!res.ok) {
        toasts.error("Xóa tài khoản thất bại");
        return;
      }
      toasts.success(`Đã xóa ${user.username}`);
      await fetchUsers();
    } catch (_) {
      toasts.error("Lỗi kết nối khi xóa tài khoản");
    }
  }

  function fmt(ts?: string | null) {
    if (!ts) return "-";
    return new Date(ts).toLocaleString();
  }
</script>

<svelte:head>
  <title>Admin Users</title>
</svelte:head>

<div class="admin-users-page">
  <div class="page-head">
    <div>
      <h1>Quản trị tài khoản</h1>
      <p>Tạo tài khoản phụ, đổi quyền, khóa mở và reset mật khẩu.</p>
    </div>
    <a class="back-link" href="/">← Về dashboard</a>
  </div>

  <section class="create-card">
    <h2>Tạo tài khoản mới</h2>
    <div class="form-grid">
      <input bind:value={newUsername} placeholder="username" />
      <input bind:value={newPassword} type="password" placeholder="password tạm" />
      <select bind:value={newRole}>
        <option value="user">user</option>
        <option value="admin">admin</option>
      </select>
      <button onclick={createUser} disabled={saving}>
        {saving ? "ĐANG TẠO..." : "TẠO TÀI KHOẢN"}
      </button>
    </div>
  </section>

  <section class="list-card">
    <div class="section-title">
      <h2>Danh sách tài khoản</h2>
      <button class="refresh-btn" onclick={fetchUsers}>Làm mới</button>
    </div>

    {#if loading}
      <div class="empty">Đang tải...</div>
    {:else if users.length === 0}
      <div class="empty">Chưa có tài khoản nào</div>
    {:else}
      <div class="table-wrap">
        <table>
          <thead>
            <tr>
              <th>User</th>
              <th>Role</th>
              <th>Trạng thái</th>
              <th>Last login</th>
              <th>Reset password</th>
              <th>Hành động</th>
            </tr>
          </thead>
          <tbody>
            {#each users as user}
              <tr>
                <td>
                  <div class="user-cell">
                    <strong>{user.username}</strong>
                    {#if me && user.id === me.id}
                      <span class="me-badge">bạn</span>
                    {/if}
                  </div>
                </td>
                <td>
                  <select
                    value={user.role}
                    onchange={(e) => updateUser(user, { role: (e.currentTarget as HTMLSelectElement).value }, `Đã đổi quyền ${user.username}`)}
                  >
                    <option value="user">user</option>
                    <option value="admin">admin</option>
                  </select>
                </td>
                <td>
                  <button
                    class="status-switch"
                    class:on={user.is_active}
                    class:off={!user.is_active}
                    onclick={() => updateUser(user, { is_active: !user.is_active }, user.is_active ? `Đã tắt ${user.username}` : `Đã bật ${user.username}`)}
                    disabled={isSelf(user)}
                    title={isSelf(user) ? "Không thể tự tắt tài khoản đang đăng nhập" : ""}
                    aria-label={user.is_active ? "Tắt user" : "Bật user"}
                  >
                    <span class="switch-track"><span class="switch-knob"></span></span>
                    <span class="switch-label">{user.is_active ? "ON" : "OFF"}</span>
                  </button>
                </td>
                <td>{fmt(user.last_login_at)}</td>
                <td>
                  <div class="reset-box">
                    <input
                      type="password"
                      placeholder="password mới"
                      value={resetPasswords[user.id] || ""}
                      oninput={(e) => {
                        resetPasswords = {
                          ...resetPasswords,
                          [user.id]: (e.currentTarget as HTMLInputElement).value,
                        };
                      }}
                    />
                    <button class="apply-btn" onclick={() => resetPassword(user)}>Apply</button>
                  </div>
                </td>
                <td>
                  <button class="danger-btn" onclick={() => removeUser(user)} disabled={isSelf(user)} title={isSelf(user) ? "Không thể tự xóa tài khoản đang đăng nhập" : ""}>Xóa</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  </section>
</div>

<style>
  .admin-users-page {
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    color: #fff;
  }
  .page-head {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
  }
  .page-head h1 { margin: 0 0 8px; }
  .page-head p { margin: 0; color: #9cb0c8; }
  .back-link { color: #71d7ff; text-decoration: none; }
  .create-card, .list-card {
    background: rgba(10, 15, 22, 0.88);
    border: 1px solid rgba(255,255,255,0.08);
    padding: 18px;
  }
  .create-card h2, .list-card h2 { margin-top: 0; }
  .form-grid {
    display: grid;
    grid-template-columns: 1.2fr 1.2fr 0.8fr auto;
    gap: 12px;
  }
  input, select, button {
    background: rgba(255,255,255,0.05);
    border: 1px solid rgba(255,255,255,0.12);
    color: #fff;
    padding: 10px 12px;
  }
  button { cursor: pointer; }
  .section-title {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 14px;
  }
  .table-wrap { overflow-x: auto; }
  table { width: 100%; border-collapse: collapse; }
  th, td { padding: 12px 10px; border-top: 1px solid rgba(255,255,255,0.08); text-align: left; vertical-align: top; }
  th { color: #8ea5be; font-size: 0.85rem; }
  .user-cell { display: flex; gap: 8px; align-items: center; }
  .me-badge {
    background: rgba(25,181,255,0.18);
    color: #8ddcff;
    padding: 2px 8px;
    font-size: 0.75rem;
  }
  .status-switch {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-width: 92px;
    border-radius: 999px;
    padding: 7px 10px;
    font-weight: 900;
    letter-spacing: .05em;
  }
  .status-switch.on { color: #8cf0b0; border-color: rgba(34,197,94,.35); background: rgba(34,197,94,.08); }
  .status-switch.off { color: #ff9d9d; border-color: rgba(248,113,113,.35); background: rgba(248,113,113,.08); }
  .status-switch:disabled { opacity: .65; cursor: not-allowed; }
  .switch-track {
    width: 34px;
    height: 18px;
    border-radius: 999px;
    position: relative;
    background: rgba(148,163,184,.35);
    transition: background .2s;
  }
  .status-switch.on .switch-track { background: rgba(34,197,94,.75); }
  .status-switch.off .switch-track { background: rgba(248,113,113,.55); }
  .switch-knob {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 2px 8px rgba(0,0,0,.35);
    transition: transform .2s;
  }
  .status-switch.on .switch-knob { transform: translateX(16px); }
  .switch-label { min-width: 28px; text-align: left; }
  .reset-box {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
  }
  .apply-btn {
    color: #dbeafe;
    border-color: rgba(255, 138, 31, .35);
    background: rgba(14,165,233,.12);
    font-weight: 800;
  }
  .apply-btn:hover { border-color: rgba(255, 138, 31, .7); background: rgba(14,165,233,.2); }
  .danger-btn {
    color: #ff9d9d;
    border-color: rgba(255, 120, 120, 0.25);
  }
  .empty {
    color: #9cb0c8;
    padding: 20px 0;
  }
  .refresh-btn { white-space: nowrap; }
  @media (max-width: 900px) {
    .form-grid { grid-template-columns: 1fr; }
    .page-head { flex-direction: column; }
  }
</style>
