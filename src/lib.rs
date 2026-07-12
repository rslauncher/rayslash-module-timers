#[allow(warnings)]
mod bindings;

use bindings::exports::rayslash::module::provider::Guest;
use bindings::rayslash::module::types::{
    Action, Icon, ModuleError, QueryContext, QueryResponse, ResultItem,
};

struct Component;

impl Guest for Component {
    fn query(context: QueryContext) -> Result<QueryResponse, ModuleError> {
        let Some(parsed) = parse(context.query.trim()) else {
            return Ok(QueryResponse {
                results: Vec::new(),
                exclusive: false,
            });
        };
        Ok(QueryResponse {
            results: vec![parsed],
            exclusive: false,
        })
    }
}

fn parse(input: &str) -> Option<ResultItem> {
    let lower = input.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("timer ") {
        let (duration, message) = duration_and_message(rest)?;
        let message = if message.is_empty() {
            "Timer finished"
        } else {
            message
        };
        return Some(item(
            input,
            "Timer",
            format!("Timer in {}", describe(duration)),
            "T",
            Action::ScheduleNotification((duration, "rayslash timer".into(), message.into())),
        ));
    }
    if let Some(rest) = lower.strip_prefix("reminder in ") {
        let (duration, message) = duration_and_message(rest)?;
        let message = message.strip_prefix("to ").unwrap_or(message);
        if message.is_empty() {
            return None;
        }
        return Some(item(
            input,
            "Reminder",
            format!("Reminder in {}", describe(duration)),
            "R",
            Action::ScheduleNotification((duration, "rayslash reminder".into(), message.into())),
        ));
    }
    let actions = [
        ("reboot", "Reboot", vec!["systemctl", "reboot"]),
        ("restart", "Reboot", vec!["systemctl", "reboot"]),
        ("shutdown", "Shut down", vec!["systemctl", "poweroff"]),
        ("poweroff", "Shut down", vec!["systemctl", "poweroff"]),
        (
            "logout",
            "Log out",
            vec!["loginctl", "terminate-session", "self"],
        ),
        ("lock", "Lock", vec!["loginctl", "lock-session", "self"]),
    ];
    for (prefix, title, command) in actions {
        if lower == prefix || lower.starts_with(&format!("{prefix} in ")) {
            let delay = lower
                .strip_prefix(&format!("{prefix} in "))
                .and_then(parse_duration)
                .unwrap_or(0);
            let args = command.into_iter().map(str::to_owned).collect::<Vec<_>>();
            let subtitle = if delay == 0 {
                format!("{title} now")
            } else {
                format!("{title} in {}", describe(delay))
            };
            return Some(item(
                input,
                title,
                subtitle,
                "!",
                Action::ScheduleCommand((delay, args)),
            ));
        }
    }
    None
}

fn duration_and_message(value: &str) -> Option<(u64, &str)> {
    let mut parts = value.splitn(2, char::is_whitespace);
    let duration = parse_duration(parts.next()?)?;
    Some((duration, parts.next().unwrap_or("").trim()))
}
fn parse_duration(value: &str) -> Option<u64> {
    let split = value.find(|ch: char| !ch.is_ascii_digit())?;
    let amount = value[..split].parse::<u64>().ok()?;
    let multiplier = match value[split..].trim() {
        "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => 60,
        "h" | "hr" | "hour" | "hours" => 3600,
        _ => return None,
    };
    amount.checked_mul(multiplier)
}
fn describe(seconds: u64) -> String {
    if seconds.is_multiple_of(3600) {
        format!("{}h", seconds / 3600)
    } else if seconds.is_multiple_of(60) {
        format!("{}min", seconds / 60)
    } else {
        format!("{seconds}s")
    }
}
fn item(input: &str, title: &str, subtitle: String, icon: &str, action: Action) -> ResultItem {
    ResultItem {
        id: format!("timers:{}", input.to_ascii_lowercase()),
        title: title.into(),
        subtitle,
        icon: Icon::Text(icon.into()),
        score: None,
        action,
    }
}

bindings::export!(Component with_types_in bindings);

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_timer() {
        assert!(matches!(
            parse("timer 10min break").unwrap().action,
            Action::ScheduleNotification((600, _, _))
        ));
    }
    #[test]
    fn parses_delayed_shutdown() {
        assert!(matches!(
            parse("shutdown in 5min").unwrap().action,
            Action::ScheduleCommand((300, _))
        ));
    }
}
