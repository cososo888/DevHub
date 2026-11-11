import type { Software } from "../catalog";

/** Query is global: once it has 2+ chars, the selected category is ignored. */
export function filterCurated(items: Software[], query: string, category: string): Software[] {
  const needle = query.trim().normalize("NFKC").toLocaleLowerCase();
  const global = needle.length >= 2;
  return items.filter(item => {
    if (!global && category !== "全部" && item.category !== category) return false;
    return !needle
      || item.name.normalize("NFKC").toLocaleLowerCase().includes(needle)
      || item.package.normalize("NFKC").toLocaleLowerCase().includes(needle);
  });
}
