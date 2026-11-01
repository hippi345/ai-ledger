type Props = {
  paused: boolean;
  lastIngest: Date | null;
  onToggle: () => void;
};

const hhmm = (d: Date) =>
  d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });

/** "Live" or "Paused" status, the last update time, and a pause toggle. */
export function LiveIndicator({ paused, lastIngest, onToggle }: Props) {
  return (
    <div
      role="status"
      aria-label="Live updates"
      className={paused ? "live paused" : "live"}
    >
      <span aria-hidden="true">{paused ? "○" : "●"}</span>{" "}
      {paused ? "Paused" : "Live"}
      {lastIngest && <span> · updated {hhmm(lastIngest)}</span>}
      <button type="button" aria-pressed={paused} onClick={onToggle}>
        {paused ? "Resume" : "Pause"}
      </button>
    </div>
  );
}
