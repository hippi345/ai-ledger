import { useEffect, useState } from "react";
import { commands } from "../bindings";

/** ledger_core::settings::keys::LAUNCH_AT_LOGIN. */
export const LAUNCH_KEY = "app.launch_at_login";

/** Launch at login (off by default); the backend registers it with the OS. */
export function StartupPanel() {
  const [on, setOn] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void commands.getSetting(LAUNCH_KEY).then((r) => {
      if (r.status === "ok") setOn(r.data === true);
      else setError(r.error.message);
    });
  }, []);
  const toggle = async (next: boolean) => {
    const r = await commands.setLaunchAtLogin(next);
    if (r.status === "ok") {
      setOn(next);
      setError(null);
    } else setError(r.error.message);
  };
  return (
    <section aria-labelledby="startup-h">
      <h3 id="startup-h">Startup</h3>
      {error && <p role="alert">{error}</p>}
      <label>
        <input
          className="switch"
          type="checkbox"
          checked={on ?? false}
          disabled={on === null}
          onChange={(e) => void toggle(e.target.checked)}
        />{" "}
        Launch AI Ledger at login
      </label>
      <p>
        Closing the window keeps AI Ledger in the tray so it can keep recording.
      </p>
    </section>
  );
}
