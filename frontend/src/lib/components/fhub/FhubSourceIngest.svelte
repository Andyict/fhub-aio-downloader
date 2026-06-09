<script lang="ts">
  import ServerFsharePanel from "$lib/components/downloads/ServerFsharePanel.svelte";
</script>

<section class="fhub-source-ingest" aria-label="FHUB source ingest">
  <div class="fhub-ingest-frame">
    <div class="fhub-ingest-kicker">FHUB SOURCE INGEST</div>
    <ServerFsharePanel />
  </div>
</section>

<style>
  .fhub-source-ingest {
    display: grid;
    gap: 0.85rem;
  }

  .fhub-ingest-frame {
    position: relative;
  }

  .fhub-ingest-kicker {
    margin: 0 0 0.55rem 0.15rem;
    color: rgba(245, 184, 92, 0.78);
    font-size: 0.66rem;
    font-weight: 950;
    letter-spacing: 0.24em;
    text-transform: uppercase;
  }

  :global(.fhub-source-ingest .server-fshare-panel) {
    margin-bottom: 0;
  }
</style>
