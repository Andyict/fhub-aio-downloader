<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";

  onMount(() => {
    goto("/discover", { replaceState: true });
  });
</script>

<div class="redirecting">Đang mở Khám phá...</div>

<style>
  .redirecting { min-height: 60dvh; display: grid; place-items: center; color: rgba(226,232,240,.72); font-weight: 800; }
</style>
