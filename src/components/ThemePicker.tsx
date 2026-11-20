import { useEffect, useId, useRef, useState } from "react";
import type { Theme } from "../hooks/useTheme";

const OPTIONS: { value: Theme; label: string }[] = [
  { value: "light", label: "日间" },
  { value: "dark", label: "夜间" },
  { value: "system", label: "自动" }
];

function ThemeIcon({ value }: { value: Theme }) {
  return (
    <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
      {value === "light" ? <><circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.4 1.4m11.2 11.2L19 19M5 19l1.4-1.4M17.6 6.4 19 5" /></>
        : value === "dark" ? <path d="M20.5 13a8.5 8.5 0 0 1-9.5-9.5A8.5 8.5 0 1 0 20.5 13Z" />
          : <><circle cx="12" cy="12" r="8.5" /><path d="M12 3.5v17a8.5 8.5 0 0 0 0-17Z" fill="currentColor" stroke="none" /></>}
    </svg>
  );
}

export function ThemePicker({ theme, onTheme }: { theme: Theme; onTheme: (value: Theme) => void }) {
  const [open, setOpen] = useState(false);
  const [focused, setFocused] = useState(0);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const choices = useRef<(HTMLButtonElement | null)[]>([]);
  const id = useId();
  const current = OPTIONS.find(option => option.value === theme) ?? OPTIONS[2];

  useEffect(() => {
    if (!open) return;
    const index = OPTIONS.findIndex(option => option.value === theme);
    setFocused(index);
    choices.current[index]?.focus();
    const dismiss = (event: Event) => {
      if (event.target instanceof Node && root.current?.contains(event.target)) return;
      setOpen(false);
    };
    document.addEventListener("pointerdown", dismiss);
    document.addEventListener("wheel", dismiss, { passive: true });
    document.addEventListener("focusin", dismiss);
    return () => {
      document.removeEventListener("pointerdown", dismiss);
      document.removeEventListener("wheel", dismiss);
      document.removeEventListener("focusin", dismiss);
    };
  }, [open, theme]);

  function closeAndFocus() {
    setOpen(false);
    trigger.current?.focus();
  }

  function focusChoice(index: number) {
    setFocused(index);
    choices.current[index]?.focus();
  }

  return (
    <div className="appearance-picker" ref={root}>
      <button ref={trigger} type="button" className="icon-button appearance-trigger" aria-label="外观设置" title={`外观：${current.label}`} aria-haspopup="menu" aria-expanded={open} aria-controls={open ? id : undefined}
        onClick={() => setOpen(value => !value)}
        onKeyDown={event => {
          if (event.key === "ArrowDown" || event.key === "ArrowUp") {
            event.preventDefault();
            setOpen(true);
          } else if (event.key === "Escape" && open) {
            event.preventDefault();
            closeAndFocus();
          }
        }}>
        <ThemeIcon value={theme} />
      </button>
      {open && <div className="appearance-menu" id={id} role="menu" aria-label="选择外观"
        onKeyDown={event => {
          if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); closeAndFocus(); }
          else if (event.key === "ArrowDown") { event.preventDefault(); focusChoice((focused + 1) % OPTIONS.length); }
          else if (event.key === "ArrowUp") { event.preventDefault(); focusChoice((focused + OPTIONS.length - 1) % OPTIONS.length); }
          else if (event.key === "Home") { event.preventDefault(); focusChoice(0); }
          else if (event.key === "End") { event.preventDefault(); focusChoice(OPTIONS.length - 1); }
          else if (event.key === "Tab") { setOpen(false); trigger.current?.focus(); }
        }}>
        {OPTIONS.map((option, index) => <button key={option.value} ref={element => { choices.current[index] = element; }} type="button" role="menuitemradio" aria-checked={theme === option.value} tabIndex={focused === index ? 0 : -1}
          onFocus={() => setFocused(index)}
          onClick={() => { onTheme(option.value); closeAndFocus(); }}>
          <ThemeIcon value={option.value} />
          <span>{option.label}</span>
          {theme === option.value && <svg className="appearance-check" viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true"><path d="m3 8 3 3 7-7" /></svg>}
        </button>)}
      </div>}
    </div>
  );
}
