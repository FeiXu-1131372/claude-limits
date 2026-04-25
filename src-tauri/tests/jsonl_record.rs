use claude_usage_monitor_lib::jsonl_parser::SessionEvent;

#[test]
fn current_schema_parses_every_line() {
    let raw = include_str!("fixtures/jsonl/current_schema.jsonl");
    let events: Vec<SessionEvent> = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("parse"))
        .collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[1].model, "claude-opus-4-7-20260115");
    assert_eq!(events[0].cache_read_tokens, 200);
}

#[test]
fn older_schema_with_unknown_fields_still_parses() {
    let raw = include_str!("fixtures/jsonl/older_schema.jsonl");
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        let e: SessionEvent = serde_json::from_str(line).expect("parse older");
        assert!(!e.project.is_empty());
    }
}

#[test]
fn malformed_lines_are_individually_rejectable() {
    let raw = include_str!("fixtures/jsonl/malformed_lines.jsonl");
    let (ok, err): (Vec<_>, Vec<_>) = raw
        .lines()
        .filter(|l| !l.trim().is_empty())
        .partition(|l| serde_json::from_str::<SessionEvent>(l).is_ok());
    assert_eq!(ok.len(), 3);
    assert_eq!(err.len(), 2);
}
