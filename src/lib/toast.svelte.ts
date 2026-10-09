export interface Toast {
  id: number;
  text: string;
  kind: "info" | "error" | "ok";
}

let next = 1;
export const toasts = $state<Toast[]>([]);

export function toast(text: string, kind: Toast["kind"] = "info", ms = 3500) {
  const id = next++;
  toasts.push({ id, text, kind });
  setTimeout(() => {
    const i = toasts.findIndex((t) => t.id === id);
    if (i >= 0) toasts.splice(i, 1);
  }, ms);
}
