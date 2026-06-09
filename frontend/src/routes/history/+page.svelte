<script lang="ts">
  const rows = [
    { time: "08:42", action: "Đã mở phim", title: "Dune: Part Two", detail: "Từ thư viện local" },
    { time: "08:31", action: "Đã tải xong", title: "Interstellar", detail: "FShare · 24.8 GB" },
    { time: "08:18", action: "Đã tìm kiếm", title: "Oppenheimer 4K", detail: "Discover query" },
    { time: "07:56", action: "Đã thêm yêu thích", title: "The Dark Knight", detail: "Movie card" },
  ];
</script>

<div class="history-surface">
  <section class="hero"><span class="eyebrow">History</span><h1>Lịch sử FHUB.</h1><p>Theo dõi hoạt động xem, tải, tìm kiếm và thao tác thư viện trên cùng một timeline.</p></section>
  <section class="timeline">
    {#each rows as row}
      <article>
        <time>{row.time}</time>
        <div><strong>{row.action}</strong><span>{row.title}</span><small>{row.detail}</small></div>
      </article>
    {/each}
  </section>
</div>

<style>
  .history-surface{display:grid;gap:1rem;max-width:1100px;margin:0 auto}.hero{min-height:280px;display:grid;align-content:end;padding:clamp(1.4rem,4vw,2.8rem);border-radius:26px;border:1px solid rgba(167,139,250,.28);background:radial-gradient(circle at 12% 44%,rgba(124,58,237,.52),transparent 38%),linear-gradient(120deg,rgba(41,30,79,.98),rgba(9,13,23,.98))}.eyebrow{color:#c4b5fd;font-size:.73rem;font-weight:950;letter-spacing:.16em;text-transform:uppercase}h1,p{margin:0}h1{margin-top:.5rem;color:#fff;font-size:clamp(3.4rem,7vw,7rem);line-height:.9;letter-spacing:-.06em}p{max-width:620px;margin-top:1rem;color:rgba(226,232,240,.68)}.timeline{display:grid;gap:.85rem}.timeline article{display:grid;grid-template-columns:76px minmax(0,1fr);gap:1rem;padding:1rem;border-radius:20px;border:1px solid rgba(148,163,184,.16);background:linear-gradient(180deg,rgba(20,26,42,.92),rgba(9,12,21,.86));box-shadow:0 18px 48px rgba(0,0,0,.24)}time{color:#f8c14a;font-weight:950}.timeline strong,.timeline span,.timeline small{display:block}.timeline strong{color:#fff}.timeline span{margin-top:.2rem;color:rgba(226,232,240,.8);font-weight:850}.timeline small{margin-top:.2rem;color:rgba(226,232,240,.58)}
  @media(max-width:720px){.history-surface{gap:.7rem}.hero{min-height:auto;padding:1rem 1.05rem;border-radius:20px;align-content:center}.eyebrow{font-size:.62rem;letter-spacing:.14em}h1{margin-top:.25rem;font-size:clamp(2rem,11vw,3rem);line-height:.95}p{margin-top:.45rem;font-size:.88rem;line-height:1.35}.timeline{gap:.55rem}.timeline article{grid-template-columns:54px minmax(0,1fr);gap:.7rem;padding:.72rem .78rem;border-radius:16px;box-shadow:0 10px 28px rgba(0,0,0,.18)}time{font-size:.92rem}.timeline strong{font-size:.9rem}.timeline span{font-size:.9rem;margin-top:.08rem;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.timeline small{font-size:.78rem;margin-top:.08rem}}
</style>
