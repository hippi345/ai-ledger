import { useCallback, useEffect, useState } from "react";
import { commands } from "../bindings";
import {
  deleteView,
  parseViews,
  saveView,
  VIEWS_KEY,
  type SavedView,
  type ViewSearch,
} from "./views";

/** Saved views, loaded from settings and saved on every change. */
export function useSavedViews() {
  const [views, setViews] = useState<SavedView[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void commands.getSetting(VIEWS_KEY).then((r) => {
      if (r.status === "ok") setViews(parseViews(r.data));
    });
  }, []);

  const persist = useCallback(async (next: SavedView[]) => {
    setViews(next);
    const r = await commands.setSetting(VIEWS_KEY, next);
    setError(r.status === "error" ? r.error.message : null);
  }, []);

  return {
    views,
    error,
    save: (name: string, search: ViewSearch) =>
      persist(saveView(views, name, search)),
    remove: (name: string) => persist(deleteView(views, name)),
  };
}
