//! Deterministic "where to cut" suggestion. The LLM may later explain this
//! result in words, but it never produces the numbers.

use serde::Serialize;
use uuid::Uuid;

/// Cuts are rounded up to this step so suggestions read naturally ("Rp 100.000", not "Rp 93.117").
pub const ROUND_STEP: i64 = 10_000;

#[derive(Debug, Clone)]
pub struct Cuttable {
    pub category_id: Uuid,
    pub name: String,
    /// Unspent part of this month's limit. A cut can never exceed it.
    pub room: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cut {
    pub category_id: Uuid,
    pub name: String,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rebalance {
    pub shortfall: i64,
    pub cuts: Vec<Cut>,
    /// Part of the shortfall flexible categories can't cover.
    pub uncovered: i64,
}

fn round_up(v: i64, step: i64) -> i64 {
    (v + step - 1) / step * step
}

/// Spread `shortfall` over flexible categories in proportion to their room.
pub fn suggest(shortfall: i64, candidates: &[Cuttable]) -> Rebalance {
    let mut pool: Vec<&Cuttable> = candidates.iter().filter(|c| c.room > 0).collect();
    // Biggest room first: the order of greedy top-ups below.
    pool.sort_by(|a, b| b.room.cmp(&a.room).then(a.name.cmp(&b.name)));
    let total_room: i64 = pool.iter().map(|c| c.room).sum();

    if shortfall <= 0 || pool.is_empty() {
        return Rebalance {
            shortfall: shortfall.max(0),
            cuts: vec![],
            uncovered: shortfall.max(0),
        };
    }

    let target = shortfall.min(total_room);
    let mut amounts: Vec<i64> = pool
        .iter()
        .map(|c| {
            let share = (target as i128 * c.room as i128 / total_room as i128) as i64;
            round_up(share, ROUND_STEP).min(c.room)
        })
        .collect();

    // Rounding/capping may leave a gap; top up greedily in order of room.
    let mut covered: i64 = amounts.iter().sum();
    for (i, c) in pool.iter().enumerate() {
        if covered >= target {
            break;
        }
        let add = (target - covered).min(c.room - amounts[i]);
        let add = round_up(add, ROUND_STEP).min(c.room - amounts[i]);
        amounts[i] += add;
        covered += add;
    }

    let cuts = pool
        .iter()
        .zip(amounts)
        .filter(|(_, a)| *a > 0)
        .map(|(c, amount)| Cut {
            category_id: c.category_id,
            name: c.name.clone(),
            amount,
        })
        .collect();

    Rebalance {
        shortfall,
        cuts,
        uncovered: (shortfall - covered).max(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(name: &str, room: i64) -> Cuttable {
        Cuttable {
            category_id: Uuid::new_v4(),
            name: name.into(),
            room,
        }
    }

    #[test]
    fn proportional_and_rounded() {
        let r = suggest(150_000, &[cat("Food", 400_000), cat("Shopping", 200_000)]);
        let total: i64 = r.cuts.iter().map(|c| c.amount).sum();
        assert!(total >= 150_000);
        assert!(r.cuts.iter().all(|c| c.amount % ROUND_STEP == 0));
        assert_eq!(r.cuts[0].name, "Food");
        assert_eq!(r.cuts[0].amount, 100_000);
        assert_eq!(r.cuts[1].amount, 50_000);
        assert_eq!(r.uncovered, 0);
    }

    #[test]
    fn never_cuts_more_than_room() {
        let r = suggest(500_000, &[cat("Food", 120_000), cat("Fun", 35_000)]);
        for c in &r.cuts {
            let room = if c.name == "Food" { 120_000 } else { 35_000 };
            assert!(c.amount <= room);
        }
        assert_eq!(r.uncovered, 500_000 - 155_000);
    }

    #[test]
    fn nothing_to_do() {
        assert!(suggest(0, &[cat("Food", 100_000)]).cuts.is_empty());
        let r = suggest(50_000, &[cat("Food", 0)]);
        assert!(r.cuts.is_empty());
        assert_eq!(r.uncovered, 50_000);
    }

    #[test]
    fn small_room_tail_is_still_used() {
        // Room that isn't a multiple of the step must still be usable.
        let r = suggest(15_000, &[cat("Food", 15_000)]);
        assert_eq!(r.cuts[0].amount, 15_000);
        assert_eq!(r.uncovered, 0);
    }
}
