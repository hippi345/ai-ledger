import { useRef } from "react";
import {
  Link,
  Outlet,
  useNavigate,
  useSearch,
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  type RouterHistory,
} from "@tanstack/react-router";
import { PricesSettings } from "./settings/PricesSettings";
import { SourcesSettings } from "./settings/SourcesSettings";
import { Overview } from "./overview/Overview";
import { DimensionPage } from "./pages/DimensionPage";
import { SessionsPage } from "./pages/SessionsPage";
import { ToolsPage } from "./pages/ToolsPage";
import { DateRangePicker } from "./filters/DateRangePicker";
import { FilterBar } from "./filters/FilterBar";
import { parseFilterSearch, type FilterSearch } from "./filters/filters";
import { useFilters } from "./filters/useFilters";
import { SavedViews } from "./views/SavedViews";
import { useSavedViews } from "./views/useSavedViews";
import { VIEW_KEYS, viewSearch } from "./views/views";
import { ExportPagePngButton } from "./export/ExportPagePngButton";
import { LiveIndicator } from "./live/LiveIndicator";
import { useLive } from "./live/useLive";
import {
  parseRangeSearch,
  resolveRange,
  utcToday,
  type RangeSearch,
} from "./filters/range";

/** Sidebar pages in order (spec §D). */
export const PAGES = [
  { path: "/", label: "Overview" },
  { path: "/apps", label: "Apps" },
  { path: "/models", label: "Models" },
  { path: "/tools", label: "Tools & MCP" },
  { path: "/projects", label: "Projects" },
  { path: "/sessions", label: "Sessions" },
  { path: "/costs", label: "Costs" },
  { path: "/settings", label: "Settings" },
] as const;

/** The global date-range picker, synced to the URL search. */
function GlobalRange() {
  const search = useSearch({ strict: false }) as RangeSearch;
  const navigate = useNavigate();
  return (
    <DateRangePicker
      value={resolveRange(search, utcToday())}
      search={search}
      onChange={(next) =>
        void navigate({
          to: ".",
          search: (prev: Record<string, unknown>) => ({
            ...prev,
            range: undefined,
            from: undefined,
            to: undefined,
            ...next,
          }),
        })
      }
    />
  );
}

/** Active filter chips, synced to the URL search. */
function GlobalFilters() {
  const f = useFilters();
  return (
    <FilterBar filters={f.filters} onRemove={f.toggle} onClear={f.clear} />
  );
}

/** Saved views: capture the current range and filters, or reapply one. */
function GlobalViews() {
  const search = useSearch({ strict: false }) as Record<string, unknown>;
  const navigate = useNavigate();
  const v = useSavedViews();
  return (
    <SavedViews
      views={v.views}
      onSave={(name) => void v.save(name, viewSearch(search))}
      onDelete={(name) => void v.remove(name)}
      onApply={(view) =>
        void navigate({
          to: ".",
          search: (prev: Record<string, unknown>) => {
            const out: Record<string, unknown> = { ...prev };
            for (const k of VIEW_KEYS) out[k] = undefined;
            return { ...out, ...view.search };
          },
        })
      }
    />
  );
}

/** Live status and pause toggle; tiles refresh themselves on ingest. */
function GlobalLive() {
  const live = useLive();
  return (
    <LiveIndicator
      paused={live.paused}
      lastIngest={live.lastIngest}
      onToggle={() => void live.toggle()}
    />
  );
}

function Shell() {
  return (
    <div className="shell">
      <nav aria-label="Main">
        <h1>AI Ledger</h1>
        <ul>
          {PAGES.map((p) => (
            <li key={p.path}>
              <Link
                to={p.path}
                activeOptions={{ exact: true }}
                activeProps={{ "aria-current": "page" }}
              >
                {p.label}
              </Link>
            </li>
          ))}
        </ul>
        <p>Local-first. No data leaves this machine.</p>
      </nav>
      <main>
        <header>
          <GlobalLive />
          <GlobalRange />
          <GlobalFilters />
          <GlobalViews />
        </header>
        <Outlet />
      </main>
    </div>
  );
}

function Page({
  title,
  children,
}: {
  title: string;
  children?: React.ReactNode;
}) {
  const ref = useRef<HTMLElement>(null);
  return (
    <section ref={ref} aria-labelledby="page-h">
      <h2 id="page-h">{title}</h2>
      <ExportPagePngButton target={ref} name={title} />
      {children}
    </section>
  );
}

const root = createRootRoute({
  component: Shell,
  validateSearch: (
    raw: Record<string, unknown>,
  ): RangeSearch & FilterSearch => ({
    ...parseRangeSearch(raw),
    ...parseFilterSearch(raw),
  }),
});

const routes = PAGES.map((p) =>
  createRoute({
    getParentRoute: () => root,
    path: p.path,
    component: () => {
      if (p.path === "/")
        return (
          <Page title={p.label}>
            <Overview />
          </Page>
        );
      if (p.path === "/apps")
        return (
          <Page title={p.label}>
            <DimensionPage dim="app" />
          </Page>
        );
      if (p.path === "/models")
        return (
          <Page title={p.label}>
            <DimensionPage dim="model" />
          </Page>
        );
      if (p.path === "/projects")
        return (
          <Page title={p.label}>
            <DimensionPage dim="project" />
          </Page>
        );
      if (p.path === "/tools")
        return (
          <Page title={p.label}>
            <ToolsPage />
          </Page>
        );
      if (p.path === "/sessions")
        return (
          <Page title={p.label}>
            <SessionsPage />
          </Page>
        );
      if (p.path === "/settings")
        return (
          <Page title={p.label}>
            <SourcesSettings />
            <PricesSettings />
          </Page>
        );
      return <Page title={p.label} />;
    },
  }),
);

/** Hash history suits the Tauri webview (no server rewrites). */
export function makeRouter(history: RouterHistory = createHashHistory()) {
  return createRouter({ routeTree: root.addChildren(routes), history });
}

declare module "@tanstack/react-router" {
  interface Register {
    router: ReturnType<typeof makeRouter>;
  }
}
