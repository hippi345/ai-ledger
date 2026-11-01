//! Tray mode (spec item 31): a tray icon whose menu shows today's usage.

use chrono::{DateTime, Duration, Local, LocalResult, NaiveTime, SecondsFormat, TimeZone, Utc};
use ledger_core::stats::Totals;

/// The UTC span `[start, end)` of the local calendar day containing `now`,
/// as RFC 3339 strings matching the ledger's `ts_utc` column.
pub fn today_span<Tz: TimeZone>(now: &DateTime<Tz>) -> (String, String) {
    let tz = now.timezone();
    let day = now.date_naive();
    let midnight = |d: chrono::NaiveDate| -> DateTime<Utc> {
        let naive = d.and_time(NaiveTime::MIN);
        match tz.from_local_datetime(&naive) {
            LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => t.with_timezone(&Utc),
            // Midnight skipped by a DST jump: the day starts an hour later.
            LocalResult::None => tz
                .from_local_datetime(&(naive + Duration::hours(1)))
                .earliest()
                .map_or_else(|| naive.and_utc(), |t| t.with_timezone(&Utc)),
        }
    };
    let fmt = |t: DateTime<Utc>| t.to_rfc3339_opts(SecondsFormat::Secs, true);
    let next = day.succ_opt().unwrap_or(day);
    (fmt(midnight(day)), fmt(midnight(next)))
}

/// Today's span in the machine's local time zone.
pub fn today_span_local() -> (String, String) {
    today_span(&Local::now())
}

/// The tray menu's summary line.
pub fn today_label(t: &Totals) -> String {
    format!(
        "Today: {} tokens · ${:.2}",
        group_thousands(t.tokens),
        t.cost_usd
    )
}

fn group_thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The "today" menu item, kept so its text can be refreshed after ingests.
pub struct TrayToday(pub tauri::menu::MenuItem<tauri::Wry>);

fn current_label(app: &tauri::AppHandle) -> String {
    use tauri::Manager;
    let (since, until) = today_span_local();
    app.try_state::<crate::state::AppState>()
        .and_then(|s| ledger_core::stats::totals_between(&s.db(), &since, &until).ok())
        .map_or_else(|| today_label(&Totals::default()), |t| today_label(&t))
}

/// Builds the tray icon with Today / Show / Quit. Call after the state is managed.
pub fn install(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;
    use tauri::Manager;

    let today = MenuItem::with_id(app, "today", current_label(app), false, None::<&str>)?;
    let show = MenuItem::with_id(app, "show", "Show AI Ledger", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&today, &sep, &show, &quit])?;
    let mut b = TrayIconBuilder::with_id("main")
        .tooltip("AI Ledger")
        .menu(&menu)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        b = b.icon(icon.clone());
    }
    b.build(app)?;
    app.manage(TrayToday(today));
    Ok(())
}

/// Re-reads today's totals into the tray menu (no-op without a tray).
pub fn refresh(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(t) = app.try_state::<TrayToday>() {
        let _ = t.0.set_text(current_label(app));
    }
}

/// True once the tray is up, so closing the window can hide it instead.
pub fn active(app: &tauri::AppHandle) -> bool {
    use tauri::Manager;
    app.try_state::<TrayToday>().is_some()
}
