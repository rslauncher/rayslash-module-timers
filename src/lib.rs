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
    if lower.starts_with("timer ") {
        let rest = &input["timer ".len()..];
        let (duration, message) = timer_parts(rest)?;
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
    if lower.starts_with("reminder in ") || lower.starts_with("remind in ") {
        let prefix = if lower.starts_with("reminder in ") {
            "reminder in "
        } else {
            "remind in "
        };
        let rest = &input[prefix.len()..];
        let (duration, message) = duration_and_message(rest)?;
        let message = strip_ascii_prefix(message, "to ").unwrap_or(message);
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
    if lower.starts_with("remind me to ") || lower.starts_with("remind to ") {
        let prefix = if lower.starts_with("remind me to ") {
            "remind me to "
        } else {
            "remind to "
        };
        let rest = &input[prefix.len()..];
        let (message, duration_text) = split_ascii_once_from_end(rest, " in ")?;
        let duration = parse_duration(duration_text.trim())?;
        if message.trim().is_empty() {
            return None;
        }
        return Some(item(
            input,
            "Reminder",
            format!("Reminder in {}", describe(duration)),
            "R",
            Action::ScheduleNotification((
                duration,
                "rayslash reminder".into(),
                message.trim().into(),
            )),
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
fn timer_parts(value: &str) -> Option<(u64, &str)> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    for (start, token) in word_offsets(value) {
        if let Some(duration) = parse_duration(token) {
            let end = start + token.len();
            let before = value[..start].trim();
            let after = value[end..].trim();
            let message = if before.is_empty() { after } else { before };
            return Some((duration, message));
        }
    }
    Some((30, value))
}
fn word_offsets(value: &str) -> impl Iterator<Item = (usize, &str)> {
    value
        .match_indices(|ch: char| !ch.is_whitespace())
        .filter(|(start, _)| *start == 0 || value[..*start].ends_with(char::is_whitespace))
        .map(move |(start, _)| {
            let end = value[start..]
                .find(char::is_whitespace)
                .map_or(value.len(), |end| start + end);
            (start, &value[start..end])
        })
}
fn strip_ascii_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
        .then(|| &value[prefix.len()..])
}
fn split_ascii_once_from_end<'a>(value: &'a str, delimiter: &str) -> Option<(&'a str, &'a str)> {
    let lower = value.to_ascii_lowercase();
    let index = lower.rfind(delimiter)?;
    Some((&value[..index], &value[index + delimiter.len()..]))
}
fn parse_duration(value: &str) -> Option<u64> {
    let split = value.find(|ch: char| !ch.is_ascii_digit())?;
    let amount = value[..split].parse::<u64>().ok()?;
    let unit = value[split..].trim().to_ascii_lowercase();
    let multiplier = match unit.as_str() {
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
        let item = parse("timer 10MIN Take a Break").unwrap();
        assert!(matches!(
            item.action,
            Action::ScheduleNotification((600, _, ref message)) if message == "Take a Break"
        ));
    }
    #[test]
    fn parses_delayed_shutdown() {
        assert!(matches!(
            parse("shutdown in 5min").unwrap().action,
            Action::ScheduleCommand((300, _))
        ));
    }
    #[test]
    fn preserves_legacy_timer_and_reminder_syntax() {
        assert!(matches!(
            parse("timer feed the cat 10min").unwrap().action,
            Action::ScheduleNotification((600, _, ref message)) if message == "feed the cat"
        ));
        assert!(matches!(
            parse("remind me to feed the cat in 10 minutes").unwrap().action,
            Action::ScheduleNotification((600, _, ref message)) if message == "feed the cat"
        ));
        assert!(matches!(
            parse("remind in 10min to feed the cat").unwrap().action,
            Action::ScheduleNotification((600, _, ref message)) if message == "feed the cat"
        ));
    }
    #[test]
    fn invalid_timer_input_is_not_actionable() {
        assert!(parse("timer").is_none());
        assert!(parse("remind me sometime").is_none());
    }
}
