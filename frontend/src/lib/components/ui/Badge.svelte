<script lang="ts">
  export type BadgeVariant =
    | "default" | "primary" | "success" | "warning" | "danger" | "info"
    | "purple" | "orange" | "grey" | "quality" | "source" | "language"
    | "episode" | "status" | "vip" | "free" | "best" | "smart" | "hdr"
    | "dv" | "downloaded" | "count";

  interface Props {
    text: string;
    variant?: BadgeVariant;
    color?: string;
    size?: "xs" | "sm" | "md";
    noDot?: boolean;
    class?: string;
  }

  let { text, variant = "default", color, size = "sm", noDot = false, class: extraClass = "" }: Props = $props();

  const PALETTE: Record<string, [string, string, string]> = {
    default: ["rgba(226,232,240,0.08)", "rgba(226,232,240,0.14)", "#cbd5e1"],
    primary: ["rgba(255, 138, 31, 0.12)", "rgba(255, 138, 31, 0.26)", "#bae6fd"],
    success: ["rgba(52,211,153,0.12)", "rgba(52,211,153,0.26)", "#a7f3d0"],
    warning: ["rgba(251,191,36,0.12)", "rgba(251,191,36,0.28)", "#fde68a"],
    danger: ["rgba(248,113,113,0.12)", "rgba(248,113,113,0.28)", "#fecaca"],
    info: ["rgba(255, 179, 92, 0.12)", "rgba(255, 179, 92, 0.26)", "#bfdbfe"],
    purple: ["rgba(253, 186, 116, 0.12)", "rgba(253, 186, 116, 0.26)", "#fed7aa"],
    orange: ["rgba(139,92,246,0.12)", "rgba(139,92,246,0.26)", "#ddd6fe"],
    grey: ["rgba(148,163,184,0.09)", "rgba(148,163,184,0.16)", "#cbd5e1"],
    quality: ["rgba(255, 138, 31, 0.12)", "rgba(255, 138, 31, 0.26)", "#bae6fd"],
    source: ["rgba(226,232,240,0.08)", "rgba(226,232,240,0.14)", "#cbd5e1"],
    language: ["rgba(253, 186, 116, 0.12)", "rgba(253, 186, 116, 0.26)", "#fed7aa"],
    episode: ["rgba(253, 186, 116, 0.12)", "rgba(253, 186, 116, 0.26)", "#fed7aa"],
    status: ["rgba(226,232,240,0.08)", "rgba(226,232,240,0.14)", "#cbd5e1"],
    vip: ["rgba(251,191,36,0.12)", "rgba(251,191,36,0.28)", "#fde68a"],
    free: ["rgba(148,163,184,0.09)", "rgba(148,163,184,0.16)", "#cbd5e1"],
    best: ["rgba(251,191,36,0.12)", "rgba(251,191,36,0.28)", "#fde68a"],
    smart: ["rgba(255, 138, 31, 0.12)", "rgba(255, 179, 92, 0.24)", "#dbeafe"],
    downloaded: ["rgba(52,211,153,0.12)", "rgba(52,211,153,0.26)", "#a7f3d0"],
    count: ["rgba(226,232,240,0.08)", "rgba(226,232,240,0.14)", "#cbd5e1"],
    hdr: ["rgba(129,140,248,0.16)", "rgba(129,140,248,0.28)", "#e0e7ff"],
    dv: ["rgba(251,191,36,0.14)", "rgba(251,191,36,0.26)", "#fef3c7"],
  };

  function hexToRgba(hex: string, alpha: number): string {
    if (hex.startsWith("rgb")) return hex;
    const r = parseInt(hex.slice(1, 3), 16);
    const g = parseInt(hex.slice(3, 5), 16);
    const b = parseInt(hex.slice(5, 7), 16);
    return `rgba(${r},${g},${b},${alpha})`;
  }

  function getStyle(): string {
    if (color) {
      const bg = color.startsWith("#") ? hexToRgba(color, 0.12) : color;
      const bdr = color.startsWith("#") ? hexToRgba(color, 0.28) : color;
      return `background:${bg}; border:1px solid ${bdr}; color:${color};`;
    }
    const p = PALETTE[variant] ?? PALETTE.default;
    return `background:${p[0]}; border:1px solid ${p[1]}; color:${p[2]};`;
  }

  function getDotColor(): string {
    if (color) return color;
    return (PALETTE[variant] ?? PALETTE.default)[2];
  }
</script>

<span class="badge size-{size} {extraClass}" style={getStyle()}>
  {#if !noDot}
    <span class="badge-dot" style="background:{getDotColor()}"></span>
  {/if}
  {text}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border-radius: 999px;
    font-family: Inter, system-ui, sans-serif;
    font-weight: 800;
    letter-spacing: 0.015em;
    text-transform: none;
    white-space: nowrap;
    box-shadow: inset 0 1px 0 rgba(255,255,255,0.04);
  }
  .badge-dot { width: 5px; height: 5px; border-radius: 50%; flex-shrink: 0; opacity: 0.9; }
  .size-xs { font-size: 0.56rem; padding: 0.12rem 0.42rem; }
  .size-sm { font-size: 0.64rem; padding: 0.2rem 0.52rem; }
  .size-md { font-size: 0.74rem; padding: 0.28rem 0.68rem; }
</style>
