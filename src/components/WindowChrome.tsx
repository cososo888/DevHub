import type { ReactNode } from "react";

/** One continuous glass surface; native traffic lights occupy the leading space. */
export function WindowChrome({ children }: { children: ReactNode }) {
  return (
    <header className="window-chrome" data-tauri-drag-region>
      <div className="window-toolbar" data-tauri-drag-region>
        <div className="devhub-brand" data-tauri-drag-region>
          <img src="/brand/devhub.svg" width="32" height="32" alt="DevHub" draggable={false} />
        </div>
        {children}
      </div>
    </header>
  );
}
