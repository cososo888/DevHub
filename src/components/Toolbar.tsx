import { useEffect, useRef } from "react";
import { CATEGORIES } from "../catalog";
import type { Theme } from "../hooks/useTheme";

const THEMES: { value: Theme; label: string; icon: string }[] = [
  { value: "system", label: "跟随系统", icon: "◐" },
  { value: "light", label: "浅色", icon: "☀" },
  { value: "dark", label: "深色", icon: "☾" }
];

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
  useEffect(() => {
    if (searchOpen) input.current?.focus();
  }, [searchOpen]);

  function toggleSearch() {
    if (searchOpen) onQuery("");
    onSearchToggle();
  }

  return (
    <header className="topbar">
      <div className="brand">
        <span className="brand-mark" aria-hidden="true">D</span>
        <span className="brand-copy"><strong>DevHub</strong><small>V2 · 开发环境</small></span>
      </div>
      <nav className="categories" aria-label="软件分类">
        {["全部", ...CATEGORIES].map(value => (
          <button key={value} type="button" className={`chip${category === value ? " active" : ""}`} aria-pressed={category === value} onClick={() => onCategory(value)}>{value}</button>
        ))}
      </nav>
      <div className="toolbar-right">
        <div className={`search${searchOpen ? " open" : ""}`}>
          {searchOpen && <input ref={input} aria-label="搜索软件" placeholder="搜索软件名称…" value={query} onChange={event => onQuery(event.target.value)} onKeyDown={event => { if (event.key === "Escape") toggleSearch(); }} />}
          <button type="button" className="icon-button" aria-label={searchOpen ? "收起搜索" : "搜索软件"} title={searchOpen ? "收起搜索" : "搜索"} onClick={toggleSearch}>
            <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" strokeWidth="2" /><path d="m15.5 15.5 5 5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /></svg>
          </button>
        </div>
        <div className="theme-switch" role="group" aria-label="主题">
          {THEMES.map(item => <button key={item.value} type="button" className={theme === item.value ? "active" : ""} aria-label={item.label} aria-pressed={theme === item.value} title={item.label} onClick={() => onTheme(item.value)}>{item.icon}</button>)}
        </div>
      </div>
    </header>
  );
}
