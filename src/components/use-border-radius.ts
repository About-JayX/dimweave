import { useCallback, useEffect, useState } from "react";

export type RadiusMode = "rounded" | "sharp";

const STORAGE_KEY = "dimweave:radius";

function getStoredRadius(): RadiusMode {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "rounded" || v === "sharp") return v;
  } catch {}
  return "rounded";
}

function applyRadius(mode: RadiusMode) {
  const root = document.documentElement;
  if (mode === "sharp") {
    root.style.setProperty("--radius", "0px");
    root.style.setProperty("--app-radius", "0px");
  } else {
    root.style.setProperty("--radius", "0.75rem");
    root.style.setProperty("--app-radius", "0.75rem");
  }
}

export function useBorderRadius() {
  const [mode, setMode] = useState<RadiusMode>(getStoredRadius);

  useEffect(() => {
    applyRadius(mode);
    try {
      localStorage.setItem(STORAGE_KEY, mode);
    } catch {}
  }, [mode]);

  const toggle = useCallback(() => {
    setMode((cur) => (cur === "rounded" ? "sharp" : "rounded"));
  }, []);

  return { mode, setMode, toggle } as const;
}
