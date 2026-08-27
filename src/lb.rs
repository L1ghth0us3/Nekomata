use std::collections::HashSet;

use crate::history::util::{parse_duration_secs, parse_number};
use crate::model::{EncounterSummary, LimitBreakCast, LimitBreakSummary};

/// Decode hex-encoded ability damage from network 21/22 log lines (cactbot LogGuide).
pub fn parse_ability_damage(raw: &str) -> u64 {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "0" {
        return 0;
    }
    let padded = format!("{:0>8}", trimmed);
    let bytes = match u32::from_str_radix(&padded, 16) {
        Ok(v) => v.to_be_bytes(),
        Err(_) => return 0,
    };
    let [a, b, c, d] = bytes;
    if c == 0x40 {
        ((d as u64) << 16) | ((a as u64) << 8) | (b as u64)
    } else {
        u32::from_str_radix(&padded[..4], 16).unwrap_or(0) as u64
    }
}

pub fn should_reset_lb(prev: Option<&EncounterSummary>, next: &EncounterSummary) -> bool {
    let Some(prev) = prev else {
        return !next.is_active;
    };
    if !next.is_active {
        return false;
    }
    if !prev.is_active {
        return true;
    }
    let prev_secs = parse_duration_secs(&prev.duration).unwrap_or(0);
    let next_secs = parse_duration_secs(&next.duration).unwrap_or(0);
    if next_secs + 2 < prev_secs {
        return true;
    }
    let prev_damage = parse_number(&prev.damage);
    let next_damage = parse_number(&next.damage);
    next_damage + 1.0 < prev_damage
}

#[derive(Debug, Clone)]
struct ActiveLb {
    user: String,
    sequence: String,
    damage: u64,
    counted_targets: HashSet<String>,
}

#[derive(Debug, Default)]
pub struct LimitBreakTracker {
    active: Option<ActiveLb>,
}

impl LimitBreakTracker {
    pub fn summary(&self) -> Option<LimitBreakSummary> {
        self.active.as_ref().map(|lb| LimitBreakSummary {
            user: lb.user.clone(),
            damage: lb.damage,
        })
    }

    pub fn reset(&mut self) -> Option<LimitBreakSummary> {
        self.active = None;
        None
    }

    pub fn apply_line(
        &mut self,
        cast: &LimitBreakCast,
        target_id: &str,
        damage: u64,
    ) -> LimitBreakSummary {
        let same_cast = self
            .active
            .as_ref()
            .is_some_and(|active| active.sequence == cast.sequence && !cast.sequence.is_empty());

        if same_cast {
            let active = self.active.as_mut().expect("checked some");
            if damage > 0 && active.counted_targets.insert(target_id.to_string()) {
                active.damage = active.damage.saturating_add(damage);
            }
            return LimitBreakSummary {
                user: active.user.clone(),
                damage: active.damage,
            };
        }

        let mut counted_targets = HashSet::new();
        let mut total = 0u64;
        if damage > 0 {
            counted_targets.insert(target_id.to_string());
            total = damage;
        }
        self.active = Some(ActiveLb {
            user: cast.source_name.clone(),
            sequence: cast.sequence.clone(),
            damage: total,
            counted_targets,
        });
        LimitBreakSummary {
            user: cast.source_name.clone(),
            damage: total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_and_high_damage_masks() {
        assert_eq!(parse_ability_damage("47280000"), 18216);
        assert_eq!(parse_ability_damage("426B4001"), 82539);
        assert_eq!(parse_ability_damage("13E64002"), 136166);
    }

    #[test]
    fn aoe_hits_share_sequence_and_sum_per_target() {
        let cast = LimitBreakCast {
            source_id: "abc".into(),
            source_name: "Caster".into(),
            action_id: "C9".into(),
            sequence: "00003B7E".into(),
        };
        let mut tracker = LimitBreakTracker::default();

        let first = tracker.apply_line(&cast, "target1", 100_000);
        assert_eq!(first.damage, 100_000);

        let second = tracker.apply_line(&cast, "target2", 50_000);
        assert_eq!(second.damage, 150_000);

        let duplicate = tracker.apply_line(&cast, "target1", 100_000);
        assert_eq!(duplicate.damage, 150_000);
    }

    #[test]
    fn new_sequence_starts_fresh_total() {
        let mut tracker = LimitBreakTracker::default();
        let first_cast = LimitBreakCast {
            source_id: "abc".into(),
            source_name: "Caster".into(),
            action_id: "C9".into(),
            sequence: "seq1".into(),
        };
        tracker.apply_line(&first_cast, "t1", 100);

        let second_cast = LimitBreakCast {
            sequence: "seq2".into(),
            ..first_cast
        };
        let next = tracker.apply_line(&second_cast, "t1", 200);
        assert_eq!(next.damage, 200);
    }
}
