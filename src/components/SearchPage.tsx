import { useEffect, useRef } from "react";
import type { CSSProperties } from "react";
import type { Software } from "../catalog";
import type { Theme } from "../hooks/useTheme";
import type { OrbOrigin } from "./DetailOverlay";
import { SoftwareLogo } from "./SoftwareLogo";
import { ThemePicker } from "./ThemePicker";
import { NotificationMenu } from "./NotificationMenu";

function SearchIcon({ size = 20 }: { size?: number }) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden="true"><circle cx="10.5" cy="10.5" r="6.5"/><path d="m16 16 5 5"/></svg>;
}

export function SearchToolbar({ query, onQuery, onBack, onSearch, theme, onTheme }: {
  query: string; onQuery: (value: string) => void; onBack: () => void; onSearch: () => void;
  theme: Theme; onTheme: (value: Theme) => void;
}) {
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => { input.current?.focus(); }, []);
  return <div className="topbar search-toolbar" data-tauri-drag-region>
    <div className="search-navigation" data-tauri-drag-region>
      <button type="button" className="search-back icon-button" aria-label="返回软件库" title="返回软件库（Esc）" onClick={onBack}>
        <svg viewBox="0 0 24 24" width="21" height="21" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="m14 6-6 6 6 6M8 12h12"/></svg>
      </button>
      <form className="global-search-field" role="search" onSubmit={event => { event.preventDefault(); onSearch(); }}>
        <SearchIcon />
        <input ref={input} type="search" aria-label="搜索全部软件" placeholder="搜索软件名称或包名" value={query} onChange={event => onQuery(event.target.value)} autoComplete="off" spellCheck={false}/>
        {query && <button type="button" className="search-clear" aria-label="清空搜索" onClick={() => { onQuery(""); input.current?.focus(); }}>×</button>}
        <button type="submit" className="search-submit">搜索</button>
      </form>
    </div>
    <div className="toolbar-right" data-tauri-drag-region><NotificationMenu /><ThemePicker theme={theme} onTheme={onTheme}/></div>
  </div>;
}

function SearchResult({ item, installed, busy, source, onOpen }: {
  item: Software; installed: boolean; busy: boolean; source: boolean;
  onOpen: (item: Software, origin: OrbOrigin) => void;
}) {
  const artwork = useRef<HTMLSpanElement>(null);
  return <button type="button" className={`search-result${installed ? " is-installed" : ""}${source ? " is-source" : ""}`} aria-label={`${item.name}${installed ? "，已安装" : ""}`} aria-busy={busy || undefined}
    onClick={event => onOpen(item, (artwork.current ?? event.currentTarget).getBoundingClientRect())}>
    <span className="search-result-art" ref={artwork} style={{ "--orb-accent": item.accent } as CSSProperties}><SoftwareLogo id={item.id} name={item.name}/></span>
    <span className="search-result-copy"><strong>{item.name}</strong><span>{item.description}</span><small>{item.package} · {item.kind === "cask" ? "桌面应用" : "命令行工具"}</small></span>
    <span className="search-result-state">{busy ? <><span className="mini-spinner"/>处理中</> : installed ? "已安装" : <svg viewBox="0 0 16 16" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden="true"><path d="m6 3 5 5-5 5"/></svg>}</span>
  </button>;
}

export function SearchPage({ query, curated, remote, searching, error, onQuery, onRetry, isInstalled, isBusy, sourceId, onOpen }: {
  query: string; curated: Software[]; remote: Software[]; searching: boolean; error: string | null;
  onQuery: (value: string) => void; onRetry: () => void;
  isInstalled: (item: Software) => boolean; isBusy: (item: Software) => boolean;
  sourceId: string | null; onOpen: (item: Software, origin: OrbOrigin) => void;
}) {
  const text = query.trim();
  const row = (item: Software) => <SearchResult key={`${item.kind}:${item.package}`} item={item} installed={isInstalled(item)} busy={isBusy(item)} source={sourceId === item.id} onOpen={onOpen}/>;
  return <main className="search-page" aria-label="全局搜索页面">
    {!text ? <div className="search-welcome">
      <span className="search-welcome-icon"><SearchIcon size={30}/></span>
      <h1>搜索全部软件</h1><p>查找开发工具、运行时和桌面应用</p>
      <div className="search-suggestions" aria-label="常用搜索">{["Python", "Node.js", "Docker", "Redis", "Git", "VS Code"].map(value => <button type="button" key={value} onClick={() => onQuery(value)}>{value}</button>)}</div>
    </div> : <>
      <div className="search-page-heading"><h1>“{text}” 的搜索结果</h1><p>全软件库搜索</p></div>
      {curated.length > 0 && <section className="search-result-group" aria-label="精选软件搜索结果"><h2>精选软件 <span>{curated.length}</span></h2><div className="search-result-list">{curated.map(row)}</div></section>}
      {text.length < 2 ? <p className="search-hint" role="status">输入至少 2 个字符，搜索 Homebrew 软件库</p> : <section className="search-result-group" aria-label="Homebrew 软件库搜索结果">
        <h2>Homebrew 软件库 {remote.length > 0 && <span>{remote.length}</span>}</h2>
        {remote.length > 0 && <div className="search-result-list">{remote.map(row)}</div>}
        {searching && <p className="search-hint" role="status"><span className="mini-spinner"/>正在查找更多软件…</p>}
        {error && <div className="search-failure" role="alert"><p>软件库搜索未完成</p><button type="button" onClick={onRetry}>重试</button><details><summary>错误详情</summary><pre>{error}</pre></details></div>}
        {!searching && !error && remote.length === 0 && <p className="search-hint" role="status">{curated.length ? "没有其他匹配的软件" : "没有找到匹配的软件，试试其他名称或包名"}</p>}
      </section>}
    </>}
  </main>;
}
