//! Conservative credit based only on acknowledged, uninterrupted observations.
use chill_domain::{Lease, Progress, final_tenth_start, valid_speed};
use std::time::Instant;

#[derive(Clone, Copy)]
pub struct Observation {
    pub position_ms: u64,
    pub duration_ms: u64,
    pub speed: f64,
    pub playing: bool,
    pub buffering: bool,
    pub seek: bool,
}
#[derive(Default)]
pub struct WatchClock {
    sequence: u64,
    acknowledged: Option<(Progress, Instant)>,
}
impl WatchClock {
    pub fn forget_credit(&mut self) {
        self.acknowledged = None;
    }
    pub fn event(&mut self, lease: &Lease, obs: Observation, now: Instant) -> Option<Progress> {
        if obs.duration_ms == 0 || obs.duration_ms > i64::MAX as u64 || !valid_speed(obs.speed) {
            self.forget_credit();
            return None;
        }
        self.sequence = self.sequence.checked_add(1)?;
        let position = obs.position_ms.min(obs.duration_ms);
        let mut credit = 0;
        if let Some((last, at)) = &self.acknowledged {
            let elapsed = now
                .saturating_duration_since(*at)
                .as_millis()
                .min(u64::MAX as u128) as u64;
            if obs.playing
                && !obs.buffering
                && !obs.seek
                && last.playing
                && !last.buffering
                && !last.seek
                && last.duration_ms == obs.duration_ms
                && last.speed == obs.speed
                && position >= last.position_ms
                && (position - last.position_ms) as f64 <= elapsed as f64 * obs.speed + 1000.0
            {
                let final_distance = position
                    .saturating_sub(last.position_ms.max(final_tenth_start(obs.duration_ms)));
                credit = elapsed.min((final_distance as f64 / obs.speed).floor() as u64);
            }
        }
        Some(Progress {
            session_id: lease.session_id.clone(),
            generation: lease.generation,
            media_id: lease.media_id.clone(),
            media_revision: lease.media_revision,
            sequence: self.sequence,
            position_ms: position,
            duration_ms: obs.duration_ms,
            speed: obs.speed,
            final_elapsed_ms: credit,
            playing: obs.playing,
            buffering: obs.buffering,
            seek: obs.seek,
        })
    }
    pub fn acknowledge(&mut self, event: Progress, received_at: Instant) {
        self.acknowledged = Some((event, received_at));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn lease() -> Lease {
        Lease {
            server_epoch: "epoch".into(),
            session_id: "session".into(),
            profile_id: "p".into(),
            device_id: "d".into(),
            media_id: "m".into(),
            media_revision: 1,
            generation: 1,
            remaining_ms: 30000,
            transferring: false,
            context: Default::default(),
        }
    }
    #[test]
    fn real_time_and_boundary_are_prorated_without_credit_for_seeks_or_buffering() {
        let t = Instant::now();
        let l = lease();
        let mut clock = WatchClock::default();
        let mut o = Observation {
            position_ms: 85000,
            duration_ms: 100000,
            speed: 2.0,
            playing: true,
            buffering: false,
            seek: false,
        };
        let e = clock.event(&l, o, t).unwrap();
        assert_eq!(e.final_elapsed_ms, 0);
        clock.acknowledge(e, t);
        o.position_ms = 95000;
        assert_eq!(
            clock
                .event(&l, o, t + Duration::from_secs(5))
                .unwrap()
                .final_elapsed_ms,
            2500
        );
        o.seek = true;
        assert_eq!(
            clock
                .event(&l, o, t + Duration::from_secs(5))
                .unwrap()
                .final_elapsed_ms,
            0
        );
        o.seek = false;
        o.buffering = true;
        assert_eq!(
            clock
                .event(&l, o, t + Duration::from_secs(5))
                .unwrap()
                .final_elapsed_ms,
            0
        );
        clock.forget_credit();
        o.buffering = false;
        assert_eq!(
            clock
                .event(&l, o, t + Duration::from_secs(5))
                .unwrap()
                .final_elapsed_ms,
            0
        );
        o.duration_ms = 0;
        assert!(clock.event(&l, o, t).is_none());
    }
}
