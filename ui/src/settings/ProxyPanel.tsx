import { useEffect, useState } from "react";
import { commands, type ProxyStatus } from "../bindings";

/**
 * Opt-in localhost logging proxy (off by default). It records usage
 * metadata only: no prompts, no responses, and no API keys.
 */
export function ProxyPanel() {
  const [st, setSt] = useState<ProxyStatus | null>(null);
  const [port, setPort] = useState("");
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    void commands.proxyStatus().then((r) => {
      if (r.status === "ok") {
        setSt(r.data);
        setPort(String(r.data.port));
      } else setError(r.error.message);
    });
  }, []);
  const save = async (enabled: boolean) => {
    const n = Number(port);
    if (!Number.isInteger(n) || n < 1024 || n > 65535) {
      setError("Port must be a whole number from 1024 to 65535.");
      return;
    }
    const r = await commands.setProxy(enabled, n);
    if (r.status === "ok") {
      setSt(r.data);
      setError(r.data.error);
    } else setError(r.error.message);
  };
  const base = `http://127.0.0.1:${st?.port ?? port}`;
  return (
    <section aria-labelledby="proxy-h">
      <h3 id="proxy-h">Local logging proxy</h3>
      {error && <p role="alert">{error}</p>}
      <p>
        Optional. Off by default. Point an API client at this proxy to record
        the usage of calls the readers can't see. It listens on 127.0.0.1 only
        and forwards to api.anthropic.com or api.openai.com. It records only the
        model and token counts. Prompts, responses and API keys are never
        stored.
      </p>
      <label>
        <input
          type="checkbox"
          checked={st?.enabled ?? false}
          disabled={st === null}
          onChange={(e) => void save(e.target.checked)}
        />{" "}
        Run the logging proxy
      </label>{" "}
      <label>
        Port{" "}
        <input
          inputMode="numeric"
          value={port}
          disabled={st === null}
          onChange={(e) => setPort(e.target.value)}
          onBlur={() => st?.enabled && void save(true)}
          size={6}
        />
      </label>
      {st?.enabled && st.running && (
        <ul aria-label="Proxy base URLs">
          <li>
            Anthropic: <code>{base}/anthropic</code>
          </li>
          <li>
            OpenAI: <code>{base}/openai/v1</code>
          </li>
        </ul>
      )}
    </section>
  );
}
