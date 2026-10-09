<script lang="ts">
  // Minimal stroke icon set (24x24 grid)
  const paths: Record<string, string> = {
    settings:
      "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z",
    save: "M12 3v12m0 0l-4-4m4 4l4-4M5 21h14",
    refresh: "M21 12a9 9 0 1 1-2.6-6.4M21 4v5h-5",
    reset: "M3 12a9 9 0 1 0 2.6-6.4M3 4v5h5",
    plus: "M12 5v14M5 12h14",
    trash: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3",
    edit: "M4 20h4L19 9l-4-4L4 16v4zM13.5 6.5l4 4",
    copy: "M9 9h11v11H9zM5 15H4V4h11v1",
    x: "M6 6l12 12M18 6L6 18",
    zap: "M13 2L4 14h7l-1 8 9-12h-7l1-8z",
    mouse: "M12 3a6 6 0 0 0-6 6v6a6 6 0 0 0 12 0V9a6 6 0 0 0-6-6zM12 7v4",
    chevron: "M9 6l6 6-6 6",
    moon: "M20 14.5A8 8 0 0 1 9.5 4 8 8 0 1 0 20 14.5z",
    play: "M7 5l12 7-12 7z",
    usb: "M12 2v14M9 5l3-3 3 3M8 10v2l4 3 4-3V9M12 16a2 2 0 1 0 0 4 2 2 0 0 0 0-4z",
    wifi: "M2 9a15 15 0 0 1 20 0M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0M12 20h.01",
    info: "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20zM12 16v-5M12 8h.01",
  };
  let { name, size = 16 }: { name: string; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"><path d={paths[name] ?? ""} /></svg>
