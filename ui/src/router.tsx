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
import { Summary } from "./summary/Summary";
import { DateRangePicker } from "./filters/DateRangePicker";
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
          <GlobalRange />
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
  return (
    <section aria-labelledby="page-h">
      <h2 id="page-h">{title}</h2>
      {children}
    </section>
  );
}

const root = createRootRoute({
  component: Shell,
  validateSearch: (raw: Record<string, unknown>): RangeSearch =>
    parseRangeSearch(raw),
});

const routes = PAGES.map((p) =>
  createRoute({
    getParentRoute: () => root,
    path: p.path,
    component: () => {
      if (p.path === "/")
        return (
          <Page title={p.label}>
            <Summary />
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
