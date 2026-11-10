import { useEffect, useState } from "react";

export type Theme = "system" | "light" | "dark";
const KEY = "devhub.v2.theme";

function read(): Theme {
  try {
    const value = localStorage.getItem(KEY);
    return value === "light" || value === "dark" ? value : "system";
  } catch {
    return "system";
  }
}

export function useTheme() {
  const [theme, setTheme] = useState<Theme>(read);
  useEffect(() => {
    try {
      localStorage.setItem(KEY, theme);
    } catch {
      // private mode must not break the UI
    }
  }, [theme]);
  return { theme, setTheme };
}
