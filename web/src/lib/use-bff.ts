"use client";
import { useEffect, useState } from "react";
import { bff, type BffResult } from "./bff";

/**
 * Reads `path` through the BFF whenever it changes. Only the answer for the current path is shown, so a slow, older
 * answer never replaces a newer one. `reload` asks again for the same path.
 */
export function useBff<T>(path: string) {
  const [tick, setTick] = useState(0);
  const [res, setRes] = useState<{ key: string; path: string; r: BffResult<T> } | null>(null);
  const key = `${tick}:${path}`;
  useEffect(() => {
    if (!path) return; // nothing to read
    let live = true;
    bff<T>(path).then((r) => {
      if (live) setRes({ key, path, r });
    });
    return () => {
      live = false;
    };
  }, [key, path]);
  const current = res?.key === key ? res.r : null;
  // While reloading the same path, keep the last answer on screen (unsaved input elsewhere on the page survives).
  const shown = current ?? (res?.path === path ? res.r : null);
  return {
    loading: current === null,
    data: shown?.ok ? shown.data : null,
    etag: shown?.ok ? shown.etag : null,
    problem: current && !current.ok ? current.problem : null,
    reload: () => setTick((n) => n + 1),
  };
}
