import type { Software } from "../catalog";

/** Query is global: once it has 2+ chars, the selected category is ignored. */
export function filterCurated(items: Software[], query: string, category: string): Software[] {
  const needle = query.trim().toLowerCase();
  const global = needle.length >= 2;
  return items.filter(item => {
    if (!global && category !== "全部" && item.category !== category) return false;
    return !needle || item.name.toLowerCase().includes(needle) || item.package.toLowerCase().includes(needle);
  });
}
