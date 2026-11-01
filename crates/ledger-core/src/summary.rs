//! Plain-English summary text (spec §B), templated with no LLM call:
//! "This week: 4.2M tokens, ~$18, mostly Cursor + claude-sonnet, up 30%."

/// Compact token counts: 950, 12.3K, 4.2M, 1.1B.
pub fn tokens(n: f64) -> String {
    let n = n.max(0.0);
    let (v, unit) = if n >= 1e9 {
        (n / 1e9, "B")
    } else if n >= 1e6 {
        (n / 1e6, "M")
    } else if n >= 1e3 {
        (n / 1e3, "K")
    } else {
        return format!("{n:.0}");
    };
    let s = if v >= 100.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    };
    format!("{}{unit}", s.trim_end_matches(".0"))
}

/// Approximate cost: ~$0, <$0.01, ~$0.42, ~$18, ~$1,240.
pub fn cost(usd: f64) -> String {
    let usd = usd.max(0.0);
    if usd == 0.0 {
        "~$0".into()
    } else if usd < 0.01 {
        "<$0.01".into()
    } else if usd < 10.0 {
        format!("~${usd:.2}")
    } else {
        format!("~${}", group(usd.round() as u64))
    }
}

fn group(n: u64) -> String {
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

/// Delta wording: "up 30%", "down 12%", "flat" within ±5%, `None` without a baseline.
pub fn change(pct: Option<f64>) -> Option<String> {
    let p = pct?;
    Some(if p.abs() <= 5.0 {
        "flat".into()
    } else if p > 0.0 {
        format!("up {p:.0}%")
    } else {
        format!("down {:.0}%", -p)
    })
}

/// Human name for a stored app id: `cursor` -> `Cursor`, `claude_code` ->
/// `Claude Code`. Unknown ids are shown as stored.
pub fn app_label(id: &str) -> String {
    match id {
        "cursor" => "Cursor".into(),
        "claude_code" | "claude-code" => "Claude Code".into(),
        "codex" => "Codex".into(),
        "openai" => "OpenAI".into(),
        "anthropic" => "Anthropic".into(),
        "chatgpt-web" => "ChatGPT".into(),
        _ => id.to_owned(),
    }
}

/// Inputs for one period card.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PeriodSummary<'a> {
    /// "Today", "This week", ...
    pub label: &'a str,
    pub tokens: f64,
    pub cost_usd: f64,
    /// Leading apps/models, most used first (at most two are named).
    pub top: Vec<&'a str>,
    pub change_pct: Option<f64>,
}

/// One sentence for a period card.
pub fn sentence(s: &PeriodSummary<'_>) -> String {
    if s.tokens <= 0.0 && s.cost_usd <= 0.0 {
        return format!("{}: no usage yet.", s.label);
    }
    let mut parts = vec![format!("{} tokens", tokens(s.tokens)), cost(s.cost_usd)];
    let top: Vec<&str> = s
        .top
        .iter()
        .copied()
        .filter(|t| !t.is_empty())
        .take(2)
        .collect();
    if !top.is_empty() {
        parts.push(format!("mostly {}", top.join(" + ")));
    }
    if let Some(c) = change(s.change_pct) {
        parts.push(c);
    }
    format!("{}: {}.", s.label, parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_example() {
        let s = PeriodSummary {
            label: "This week",
            tokens: 4_210_000.0,
            cost_usd: 18.2,
            top: vec!["Cursor", "claude-sonnet", "gpt-5"],
            change_pct: Some(30.4),
        };
        assert_eq!(
            sentence(&s),
            "This week: 4.2M tokens, ~$18, mostly Cursor + claude-sonnet, up 30%."
        );
    }

    #[test]
    fn number_formats() {
        assert_eq!(tokens(950.0), "950");
        assert_eq!(tokens(12_340.0), "12.3K");
        assert_eq!(tokens(2_000_000.0), "2M");
        assert_eq!(tokens(250_000_000.0), "250M");
        assert_eq!(tokens(1_100_000_000.0), "1.1B");
        assert_eq!(cost(0.0), "~$0");
        assert_eq!(cost(0.004), "<$0.01");
        assert_eq!(cost(0.42), "~$0.42");
        assert_eq!(cost(1240.4), "~$1,240");
    }

    #[test]
    fn delta_wording() {
        assert_eq!(change(Some(5.0)).as_deref(), Some("flat"));
        assert_eq!(change(Some(-4.9)).as_deref(), Some("flat"));
        assert_eq!(change(Some(-12.2)).as_deref(), Some("down 12%"));
        assert_eq!(change(None), None);
    }

    #[test]
    fn empty_period_and_no_baseline() {
        let mut s = PeriodSummary {
            label: "Today",
            ..Default::default()
        };
        assert_eq!(sentence(&s), "Today: no usage yet.");
        s.tokens = 900.0;
        s.cost_usd = 0.02;
        assert_eq!(sentence(&s), "Today: 900 tokens, ~$0.02.");
    }

    #[test]
    fn app_labels() {
        assert_eq!(app_label("cursor"), "Cursor");
        assert_eq!(app_label("claude_code"), "Claude Code");
        assert_eq!(app_label("my-tool"), "my-tool");
    }
}
