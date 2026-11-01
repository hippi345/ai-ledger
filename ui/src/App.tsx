import { PricesSettings } from "./settings/PricesSettings";
import { SourcesSettings } from "./settings/SourcesSettings";

export function App() {
  return (
    <main>
      <h1>AI Ledger</h1>
      <p>Local-first AI usage ledger. No data leaves this machine.</p>
      <SourcesSettings />
      <PricesSettings />
    </main>
  );
}
