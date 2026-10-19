use crate::manifest::Component;
use crate::state::{IntentKind, PersistentState, PolicyMode};
use crate::{Result, UpdateError};
use time::{OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

const MAX_RETRIES_PER_LOCAL_DATE: u8 = 3;
const RETRY_DELAYS_SECONDS: [i64; 3] = [15 * 60, 60 * 60, 4 * 60 * 60];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledAttempt {
    pub component: Component,
    pub kind: IntentKind,
    pub local_date: String,
    pub retry: u8,
}

/// Determine and persist admissions for a caller-supplied local wall clock.
///
/// Supplying the offset explicitly keeps timezone policy in the OS/integrator.
/// Comparing date + wall time makes a DST spring gap a missed attempt and a
/// repeated fall hour a single attempt because the local-date ledger is durable.
pub fn due_attempts(
    state: &mut PersistentState,
    now: OffsetDateTime,
    local_offset: UtcOffset,
) -> Result<Vec<ScheduledAttempt>> {
    let local = now.to_offset(local_offset);
    let date = local.date();
    let date_text = date.to_string();
    let mut due = Vec::new();

    for component in [
        Component::Mac,
        Component::Server,
        Component::Web,
        Component::Themes,
    ] {
        let policy = state
            .policies
            .get(&component)
            .cloned()
            .unwrap_or_else(|| crate::Policy::defaults_for(component));
        if policy.mode == PolicyMode::Manual {
            continue;
        }
        let scheduled_time = Time::from_hms(policy.hour, policy.minute, 0)
            .map_err(|_| UpdateError::Invalid("invalid persisted schedule".into()))?;
        let scheduled = PrimitiveDateTime::new(date, scheduled_time).assume_offset(local_offset);
        let component_state = state.components.entry(component).or_default();

        if component_state.last_scheduled_local_date.as_deref() != Some(&date_text) {
            if local.time() >= scheduled_time {
                component_state.last_scheduled_local_date = Some(date_text.clone());
                component_state.retry_count = 0;
                component_state.retry_after_unix = None;
                due.push(ScheduledAttempt {
                    component,
                    kind: IntentKind::Check,
                    local_date: date_text.clone(),
                    retry: 0,
                });
            }
            continue;
        }

        if let Some(retry_after) = component_state.retry_after_unix
            && component_state.retry_count < MAX_RETRIES_PER_LOCAL_DATE
            && now.unix_timestamp() >= retry_after
            && now >= scheduled
        {
            due.push(ScheduledAttempt {
                component,
                kind: IntentKind::Check,
                local_date: date_text.clone(),
                retry: component_state.retry_count,
            });
            component_state.retry_after_unix = None;
        }
    }
    Ok(due)
}

pub fn record_retry(
    state: &mut PersistentState,
    component: Component,
    now: OffsetDateTime,
) -> Result<bool> {
    let component_state = state.components.entry(component).or_default();
    if component_state.retry_count >= MAX_RETRIES_PER_LOCAL_DATE {
        return Ok(false);
    }
    let index = component_state.retry_count as usize;
    component_state.retry_count += 1;
    component_state.retry_after_unix = Some(
        now.unix_timestamp()
            .saturating_add(RETRY_DELAYS_SECONDS[index]),
    );
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month};

    fn at(hour: u8, minute: u8) -> OffsetDateTime {
        PrimitiveDateTime::new(
            Date::from_calendar_date(2026, Month::October, 25).unwrap(),
            Time::from_hms(hour, minute, 0).unwrap(),
        )
        .assume_utc()
    }

    #[test]
    fn missed_time_runs_once_per_local_date() {
        let mut state = PersistentState::default();
        let attempts = due_attempts(&mut state, at(3, 0), UtcOffset::UTC).unwrap();
        assert_eq!(attempts.len(), 4);
        assert!(
            due_attempts(&mut state, at(3, 30), UtcOffset::UTC)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn bounded_retry_is_not_a_busy_loop() {
        let mut state = PersistentState::default();
        due_attempts(&mut state, at(3, 0), UtcOffset::UTC).unwrap();
        assert!(record_retry(&mut state, Component::Server, at(3, 0)).unwrap());
        assert!(
            due_attempts(&mut state, at(3, 10), UtcOffset::UTC)
                .unwrap()
                .is_empty()
        );
        let due = due_attempts(&mut state, at(3, 16), UtcOffset::UTC).unwrap();
        assert_eq!(
            due.iter()
                .filter(|a| a.component == Component::Server)
                .count(),
            1
        );
    }
}
