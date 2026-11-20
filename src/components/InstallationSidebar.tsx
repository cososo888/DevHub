export type InstallationFilter = "all" | "installed" | "not-installed";

export interface InstallationCounts {
  all: number;
  installed: number;
  "not-installed": number | null;
}

const FILTERS: { value: InstallationFilter; label: string; path: string }[] = [
  { value: "all", label: "全部软件", path: "M3 3h6v6H3zm12 0h6v6h-6zM3 15h6v6H3zm12 0h6v6h-6z" },
  { value: "installed", label: "已安装", path: "m5 12 4 4L19 6" },
  { value: "not-installed", label: "未安装", path: "M12 5v14M5 12h14" }
];

export function InstallationSidebar({ value, onChange, counts, onGuide, guideOpen }: {
  value: InstallationFilter;
  onChange: (value: InstallationFilter) => void;
  counts: InstallationCounts;
  onGuide: () => void;
  guideOpen: boolean;
}) {
  return <aside className="installation-sidebar">
    <nav aria-label="安装状态筛选">
      {FILTERS.map(filter => <button type="button" key={filter.value} aria-pressed={value === filter.value} className={value === filter.value ? "selected" : ""} onClick={() => onChange(filter.value)}>
        <svg viewBox="0 0 24 24" width="17" height="17" aria-hidden="true"><path d={filter.path} fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"/></svg>
        <span>{filter.label}</span>
        <small aria-label={`${filter.label}数量`}>{counts[filter.value] ?? "—"}</small>
      </button>)}
    </nav>
    <button type="button" id="guide-trigger" className="guide-trigger" aria-expanded={guideOpen} aria-controls={guideOpen ? "getting-started" : undefined} onClick={onGuide}>
      <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" aria-hidden="true"><path d="M12 6c-3-2-7-2-10-1v14c3-1 7-1 10 1 3-2 7-2 10-1V5c-3-1-7-1-10 1Zm0 0v14"/></svg>
      使用指南
    </button>
  </aside>;
}
