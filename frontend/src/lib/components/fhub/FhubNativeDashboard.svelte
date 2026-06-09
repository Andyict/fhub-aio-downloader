<script lang="ts">
  import FhubHeroPanel from "./FhubHeroPanel.svelte";
  import FhubShellFrame from "./FhubShellFrame.svelte";
  import FhubStatusDeck from "./FhubStatusDeck.svelte";
  import FhubEmptyState from "./FhubEmptyState.svelte";
  import FhubPanel from "./FhubPanel.svelte";
  import FhubSystemPanel from "./FhubSystemPanel.svelte";

  export let speed = "0 B/s";
  export let active = 0;
  export let queued = 0;
  export let completed = 0;
  export let hasItems = false;
</script>

<FhubShellFrame
  title="FHub Cinema Workspace"
  subtitle="Không gian quản lý media, hàng đợi và trạng thái server trong một giao diện FHUB-native."
>
  <div class="fhub-native-dashboard">
    <FhubHeroPanel
      eyebrow="FHUB NATIVE"
      title="Cinema NAS Hub"
      subtitle="Theo dõi trạng thái hệ thống, chuẩn bị nguồn media và quản lý hàng đợi server-side theo phong cách điện ảnh."
      primaryLabel="Xem hàng đợi"
      secondaryLabel="Cấu hình"
    />

    <FhubStatusDeck
      speed={speed}
      active={active}
      queued={queued}
      completed={completed}
    />

    <section class="dashboard-lanes" aria-label="FHUB dashboard lanes">
      <FhubPanel
        eyebrow="FHUB LANE"
        title="Hàng đợi server"
        description="Theo dõi các tác vụ đang chạy, đang chờ và đã hoàn tất trong một lane native của FHUB."
      >
        {#if hasItems}
          <slot name="queue"></slot>
        {:else}
          <FhubEmptyState
            icon="movie_filter"
            title="Chưa có hàng đợi"
            message="Khi có tác vụ mới, FHUB sẽ hiển thị trạng thái xử lý tại đây."
          />
        {/if}
      </FhubPanel>

      <FhubPanel
        eyebrow="FHUB SYSTEM"
        title="Trạng thái server"
        description="Không gian hiển thị kết nối, hiệu năng và các cảnh báo quan trọng."
      >
        <slot name="system">
          <FhubSystemPanel />
        </slot>
      </FhubPanel>
    </section>
  </div>
</FhubShellFrame>

<style>
  .fhub-native-dashboard {
    display: grid;
    gap: 1rem;
  }

  .dashboard-lanes {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) minmax(280px, 0.8fr);
    gap: 1rem;
  }

  @media (max-width: 1080px) {
    .dashboard-lanes {
      grid-template-columns: 1fr;
    }
  }
</style>
