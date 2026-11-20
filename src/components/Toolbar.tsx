import { CATEGORIES } from "../catalog";
import type { Theme } from "../hooks/useTheme";
import { ThemePicker } from "./ThemePicker";
import { NotificationMenu } from "./NotificationMenu";

export function Toolbar({ category, onCategory, onSearch, theme, onTheme }: {
  category: string;
  onCategory: (value: string) => void;
  onSearch: () => void;
  theme: Theme;
  onTheme: (value: Theme) => void;
}) {
  return (
    <div className="topbar" data-tauri-drag-region>
      <nav className="categories" data-tauri-drag-region aria-label="软件分类">
        {["全部", ...CATEGORIES].map(value => (
          <button key={value} type="button" className={`chip${category === value ? " active" : ""}`} aria-pressed={category === value} onClick={() => onCategory(value)}>{value}</button>
        ))}
      </nav>
      <div className="toolbar-right" data-tauri-drag-region>
        <button type="button" className="icon-button" aria-label="全局搜索" title="全局搜索" onClick={onSearch}>
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6.5" fill="none" stroke="currentColor" strokeWidth="2" /><path d="m15.5 15.5 5 5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" /></svg>
        </button>
        <NotificationMenu />
        <ThemePicker theme={theme} onTheme={onTheme} />
      </div>
    </div>
  );
}
