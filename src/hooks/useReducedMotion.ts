import { useEffect, useState } from "react";

const QUERY = "(prefers-reduced-motion: reduce)";

function initial(): boolean {
  try {
    return typeof window !== "undefined" && window.matchMedia?.(QUERY).matches === true;
  } catch {
    return false;
  }
}

export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState(initial);
  useEffect(() => {
    try {
      const query = window.matchMedia?.(QUERY);
      if (!query) return;
      const update = () => setReduced(query.matches);
      update();
      query.addEventListener?.("change", update);
      return () => query.removeEventListener?.("change", update);
    } catch {
      return;
    }
  }, []);
  return reduced;
}
