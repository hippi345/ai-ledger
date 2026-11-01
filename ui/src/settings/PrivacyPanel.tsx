import { useEffect, useState } from "react";
import { commands } from "../bindings";

/** ledger_core::settings::keys::HASH_PROJECT_NAMES. */
export const HASH_KEY = "privacy.hash_project_names";

/** Project-name hashing: a switch for new events, and an explicit, irreversible rewrite of stored names. */
export function PrivacyPanel() {
  const [hashNew, setHashNew] = useState<boolean | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void commands.getSetting(HASH_KEY).then((r) => {
      if (r.status === "ok") setHashNew(r.data === true);
      else setError(r.error.message);
    });
  }, []);

  const toggle = async (on: boolean) => {
    const r = await commands.setSetting(HASH_KEY, on);
    if (r.status === "ok") {
      setHashNew(on);
      setError(null);
    } else setError(r.error.message);
  };

  const hashExisting = async () => {
    setBusy(true);
    const r = await commands.hashExistingProjects();
    setBusy(false);
    setConfirming(false);
    if (r.status === "ok") {
      setNotice(
        r.data === 1
          ? "Hashed 1 project name."
          : `Hashed ${r.data} project names.`,
      );
      setError(null);
    } else setError(r.error.message);
  };

  return (
    <section aria-labelledby="privacy-h">
      <h3 id="privacy-h">Privacy</h3>
      <p>
        Prompts and responses are never stored, and nothing leaves this machine.
      </p>
      {error && <p role="alert">{error}</p>}
      {notice && <p role="status">{notice}</p>}
      <label>
        <input
          type="checkbox"
          checked={hashNew ?? false}
          disabled={hashNew === null}
          onChange={(e) => void toggle(e.target.checked)}
        />{" "}
        Hash project names of new usage
      </label>
      <p>
        Applies to usage recorded from now on. Names already stored are kept
        unless you hash them below.
      </p>
      {!confirming ? (
        <button type="button" onClick={() => setConfirming(true)}>
          Hash existing project names…
        </button>
      ) : (
        <div
          role="alertdialog"
          aria-labelledby="hash-confirm-h"
          aria-describedby="hash-confirm-d"
        >
          <h4 id="hash-confirm-h">Hash existing project names?</h4>
          <p id="hash-confirm-d">
            Every stored project name (in usage, sessions and totals) is
            replaced by a short hash. This can't be undone: the original names
            can't be recovered.
          </p>
          <button
            type="button"
            disabled={busy}
            onClick={() => void hashExisting()}
          >
            Hash names permanently
          </button>
          <button
            type="button"
            disabled={busy}
            onClick={() => setConfirming(false)}
          >
            Cancel
          </button>
        </div>
      )}
    </section>
  );
}
