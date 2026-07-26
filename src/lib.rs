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
        let (duration, message) = timed_message(rest);
        let notification = if message.is_empty() {
            "Timer finished"
        } else {
            message.as_str()
        };
        let title = if message.is_empty() {
            "Timer".to_owned()
        } else {
            format!("Timer: {message}")
        };
        return Some(item(
            input,
            &title,
            format!("Timer in {}", describe(duration)),
            Action::ScheduleNotification((duration, "Timer finished".into(), notification.into())),
        ));
    }
    if lower.starts_with("reminder in ")
        || lower.starts_with("remind me in ")
        || lower.starts_with("remind in ")
    {
        let prefix = if lower.starts_with("reminder in ") {
            "reminder in "
        } else if lower.starts_with("remind me in ") {
            "remind me in "
        } else {
            "remind in "
        };
        let rest = &input[prefix.len()..];
        let (duration, consumed) = parse_duration_prefix(rest)?;
        let message = rest[consumed..].trim();
        let message = strip_ascii_prefix(message, "to ").unwrap_or(message);
        if message.is_empty() {
            return None;
        }
        let title = format!("Reminder: {message}");
        return Some(item(
            input,
            &title,
            format!("Reminder in {}", describe(duration)),
            Action::ScheduleNotification((duration, "Reminder".into(), message.into())),
        ));
    }
    if lower.starts_with("remind me to ") || lower.starts_with("remind to ") {
        let prefix = if lower.starts_with("remind me to ") {
            "remind me to "
        } else {
            "remind to "
        };
        let rest = &input[prefix.len()..];
        let (duration, message) = trailing_duration(rest)
            .map(|(duration, message)| (duration, message.to_owned()))
            .unwrap_or_else(|| (30, rest.trim().to_owned()));
        if message.is_empty() {
            return None;
        }
        let title = format!("Reminder: {message}");
        return Some(item(
            input,
            &title,
            format!("Reminder in {}", describe(duration)),
            Action::ScheduleNotification((duration, "Reminder".into(), message)),
        ));
    }
    let actions = [
        ("reboot", "Reboot", vec!["systemctl", "reboot"]),
        ("restart", "Reboot", vec!["systemctl", "reboot"]),
        ("shutdown", "Shut down", vec!["systemctl", "poweroff"]),
        ("shut down", "Shut down", vec!["systemctl", "poweroff"]),
        ("turn off", "Shut down", vec!["systemctl", "poweroff"]),
        ("poweroff", "Shut down", vec!["systemctl", "poweroff"]),
        ("logout", "Log out", vec!["gnome-session-quit", "--logout"]),
        ("log out", "Log out", vec!["gnome-session-quit", "--logout"]),
        ("lock", "Lock", vec!["loginctl", "lock-sessions"]),
    ];
    for (prefix, title, command) in &actions {
        if lower == *prefix || lower.starts_with(&format!("{prefix} in ")) {
            let delay = if lower == *prefix {
                30
            } else {
                let duration = input[prefix.len() + " in ".len()..].trim();
                parse_duration_exact(duration)?
            };
            let args = command
                .iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>();
            let subtitle = format!("{title} in {}", describe(delay));
            return Some(item(
                input,
                title,
                subtitle,
                Action::ScheduleCommand((delay, args)),
            ));
        }
    }

    if lower.len() >= 3 {
        let mut matches = actions
            .iter()
            .filter(|(prefix, _, _)| prefix.starts_with(&lower));
        let (_, title, command) = matches.next()?;
        if matches.all(|(_, candidate_title, _)| candidate_title == title) {
            return Some(item(
                input,
                title,
                format!("{title} in 30s"),
                Action::ScheduleCommand((
                    30,
                    command
                        .iter()
                        .map(|value| (*value).to_owned())
                        .collect::<Vec<_>>(),
                )),
            ));
        }
    }
    None
}

fn timed_message(value: &str) -> (u64, String) {
    let value = value.trim();
    if value.is_empty() {
        return (30, String::new());
    }
    for (start, _) in word_offsets(value) {
        if let Some((duration, consumed)) = parse_duration_prefix(&value[start..]) {
            let end = start + consumed;
            let before = value[..start].trim().trim_end();
            let before = if before.eq_ignore_ascii_case("in") {
                ""
            } else {
                strip_ascii_suffix(before, " in").unwrap_or(before).trim()
            };
            let after = value[end..].trim();
            let message = [before, after]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            return (duration, message);
        }
    }
    (30, value.to_owned())
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
fn strip_ascii_suffix<'a>(value: &'a str, suffix: &str) -> Option<&'a str> {
    value
        .get(value.len().checked_sub(suffix.len())?..)
        .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
        .then(|| &value[..value.len() - suffix.len()])
}
fn trailing_duration(value: &str) -> Option<(u64, &str)> {
    let lower = value.to_ascii_lowercase();
    let index = lower.rfind(" in ")?;
    let duration = parse_duration_exact(value[index + " in ".len()..].trim())?;
    Some((duration, value[..index].trim()))
}
fn parse_duration_exact(value: &str) -> Option<u64> {
    let value = value.trim();
    let (seconds, consumed) = parse_duration_prefix(value)?;
    (value[consumed..].trim().is_empty()).then_some(seconds)
}
fn parse_duration_prefix(value: &str) -> Option<(u64, usize)> {
    let leading = value.len() - value.trim_start().len();
    let value = &value[leading..];
    let digits = value
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(value.len());
    if digits == 0 {
        return None;
    }
    let amount = value[..digits].parse::<u64>().ok()?;
    let after_digits = &value[digits..];
    let attached_unit_len = after_digits
        .find(|character: char| !character.is_ascii_alphabetic())
        .unwrap_or(after_digits.len());
    let (unit, consumed) = if attached_unit_len > 0 {
        (
            &after_digits[..attached_unit_len],
            digits + attached_unit_len,
        )
    } else {
        let whitespace = after_digits.len() - after_digits.trim_start().len();
        let after_space = &after_digits[whitespace..];
        let separated_unit_len = after_space
            .find(|character: char| !character.is_ascii_alphabetic())
            .unwrap_or(after_space.len());
        let separated_unit = &after_space[..separated_unit_len];
        if whitespace > 0 && duration_multiplier(separated_unit).is_some() {
            (separated_unit, digits + whitespace + separated_unit_len)
        } else {
            ("s", digits)
        }
    };
    let multiplier = duration_multiplier(unit)?;
    amount
        .checked_mul(multiplier)
        .map(|seconds| (seconds, leading + consumed))
}
fn duration_multiplier(unit: &str) -> Option<u64> {
    Some(match unit.to_ascii_lowercase().as_str() {
        "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => 3600,
        _ => return None,
    })
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
fn item(input: &str, title: &str, subtitle: String, action: Action) -> ResultItem {
    ResultItem {
        id: format!("timers:{}", input.to_ascii_lowercase()),
        title: title.into(),
        subtitle,
        icon: Icon::PackagePath("icon.svg".into()),
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
            Action::ScheduleNotification((600, ref title, ref message))
                if title == "Timer finished" && message == "Take a Break"
        ));
    }
    #[test]
    fn parses_delayed_shutdown() {
        for query in [
            "shutdown in 35",
            "shutdown in 35s",
            "shutdown in 35 sec",
            "shutdown in 35 seconds",
        ] {
            assert!(matches!(
                parse(query).unwrap().action,
                Action::ScheduleCommand((35, _))
            ));
        }
        for query in [
            "shutdown in 5m",
            "shutdown in 5min",
            "shutdown in 5 min",
            "shutdown in 5 minutes",
        ] {
            assert!(matches!(
                parse(query).unwrap().action,
                Action::ScheduleCommand((300, _))
            ));
        }
    }
    #[test]
    fn parses_natural_shutdown_aliases() {
        for query in ["turn off", "shut down", "turn off in 5min"] {
            let item = parse(query).unwrap();
            assert_eq!(item.title, "Shut down");
            assert!(matches!(item.action, Action::ScheduleCommand((_, _))));
        }
    }
    #[test]
    fn suggests_unambiguous_actions_before_the_full_term() {
        for (query, title) in [
            ("reb", "Reboot"),
            ("loc", "Lock"),
            ("log", "Log out"),
            ("shut", "Shut down"),
        ] {
            assert_eq!(parse(query).unwrap().title, title);
        }
        assert!(parse("lo").is_none());
    }
    #[test]
    fn parses_spaced_logout_alias() {
        let item = parse("log out").unwrap();
        assert_eq!(item.title, "Log out");
        assert!(matches!(
            item.action,
            Action::ScheduleCommand((30, ref command))
                if command == &["gnome-session-quit", "--logout"]
        ));
    }
    #[test]
    fn lock_targets_the_graphical_sessions_from_a_background_service() {
        assert!(matches!(
            parse("lock").unwrap().action,
            Action::ScheduleCommand((30, ref command))
                if command == &["loginctl", "lock-sessions"]
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
            Action::ScheduleNotification((600, ref title, ref message))
                if title == "Reminder" && message == "feed the cat"
        ));
        assert!(matches!(
            parse("remind in 10min to feed the cat").unwrap().action,
            Action::ScheduleNotification((600, _, ref message)) if message == "feed the cat"
        ));
        assert!(matches!(
            parse("remind me in 10min to feed the cat").unwrap().action,
            Action::ScheduleNotification((600, _, ref message)) if message == "feed the cat"
        ));
        assert!(matches!(
            parse("remind me in 30 to feed the cat").unwrap().action,
            Action::ScheduleNotification((30, _, ref message)) if message == "feed the cat"
        ));
        assert!(matches!(
            parse("timer in 35s feed the cat").unwrap().action,
            Action::ScheduleNotification((35, _, ref message)) if message == "feed the cat"
        ));
        assert!(matches!(
            parse("remind me to feed the cat").unwrap().action,
            Action::ScheduleNotification((30, _, ref message)) if message == "feed the cat"
        ));
        assert_eq!(
            parse("remind me to feed the cat in 35s").unwrap().title,
            "Reminder: feed the cat"
        );
    }
    #[test]
    fn invalid_timer_input_is_not_actionable() {
        assert!(parse("timer").is_none());
        assert!(parse("remind me sometime").is_none());
    }
}
