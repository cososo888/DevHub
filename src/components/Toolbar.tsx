import { useEffect, useRef } from "react";
import { CATEGORIES } from "../catalog";
import type { Theme } from "../hooks/useTheme";
import { ThemePicker } from "./ThemePicker";

export function Toolbar({ query, onQuery, category, onCategory, searchOpen, onSearchToggle, theme, onTheme }: {
  query: string;
  onQuery: (value: string) => void;
  category: string;
  onCategory: (value: string) => void;
  searchOpen: boolean;
  onSearchToggle: () => void;
  theme: Theme;
  onTheme: (value: Theme) => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  const search = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!searchOpen) return;
    const dismiss = (event: Event) => {
      if (event.target instanceof Node && search.current?.contains(event.target)) return;
      onSearchToggle();
    };
    document.addEventListener("click", dismiss);
    document.addEventListener("wheel", dismiss, { passive: true });
    return () => {
      document.removeEventListener("click", dismiss);
      document.removeEventListener("wheel", dismiss);
    };
  }, [searchOpen, onSearchToggle]);
  useEffect(() => {
    if (searchOpen) input.current?.focus();
  }, [searchOpen]);

  function toggleSearch() {
    if (searchOpen) onQuery("");
    onSearchToggle();
  }

  return (
    <div className="topbar" data-tauri-drag-region>
      <nav className="categories" data-tauri-drag-region aria-label="软件分类" title={query.trim().length >= 2 ? "搜索期间显示全部分类的结果；清空搜索后恢复所选分类" : "选择软件分类"}>
        {["全部", ...CATEGORIES].map(value => (
          <button key={value} type="button" className={`chip${category === value ? " active" : ""}`} aria-pressed={category === value} onClick={() => onCategory(value)}>{value}</button>
        ))}
      </nav>
      <div className="toolbar-right" data-tauri-drag-region>
        <div ref={search} className={`search${searchOpen ? " open" : ""}`}>
          {searchOpen && <input ref={input} aria-label="搜索软件" placeholder="搜索名称或包名…" value={query} onChange={event => onQuery(event.target.value)} onKeyDown={event => { if (event.key === "Escape") toggleSearch(); }} />}
          {searchOpen && query && <button type="button" className="icon-button" aria-label="清空搜索" title="清空搜索，恢复分类" onClick={() => { onQuery(""); input.current?.focus(); }}>×</button>}
          <button type="button" className="icon-button" aria-label={searchOpen ? "收起搜索" : "搜索软件"} title={searchOpen ? "收起搜索" : "搜索"} onClick={toggleSearch}>
            <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" strokeWidth="2" /><path d="m15.5 15.5 5 5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /></svg>
          </button>
        </div>
        <ThemePicker theme={theme} onTheme={onTheme} />
      </div>
    </div>
  );
}
