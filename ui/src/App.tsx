import { PricesSettings } from "./settings/PricesSettings";
import { SourcesSettings } from "./settings/SourcesSettings";
import { Summary } from "./summary/Summary";

export function App() {
  return (
    <main>
      <h1>AI Ledger</h1>
      <p>Local-first AI usage ledger. No data leaves this machine.</p>
      <Summary />
      <SourcesSettings />
      <PricesSettings />
    </main>
  );
}
