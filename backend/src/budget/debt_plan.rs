//! Debt payoff simulation: avalanche (highest interest first) or snowball
//! (smallest balance first). Integer rupiah, monthly compounding.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 50 years. Beyond this the plan is reported as not feasible.
pub const MAX_MONTHS: u32 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    Avalanche,
    Snowball,
}

#[derive(Debug, Clone)]
pub struct DebtInput {
    pub id: Uuid,
    pub name: String,
    pub balance: i64,
    pub interest_rate_bp: i32,
    pub min_payment: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebtPayoff {
    pub id: Uuid,
    pub name: String,
    /// 1-based month index when it reaches zero; None if not within MAX_MONTHS.
    pub payoff_month: Option<u32>,
    pub interest_paid: i64,
    /// Position in which extra money is directed at it.
    pub priority: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub strategy: Strategy,
    pub monthly_budget: i64,
    pub months: Option<u32>,
    pub total_interest: i64,
    pub feasible: bool,
    pub debts: Vec<DebtPayoff>,
}

/// Monthly interest on `balance` at an annual rate in basis points, rounded half up.
fn monthly_interest(balance: i64, rate_bp: i32) -> i64 {
    ((balance as i128 * rate_bp as i128 + 60_000) / 120_000) as i64
}

fn priority_order(debts: &[DebtInput], strategy: Strategy) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..debts.len()).collect();
    idx.sort_by(|&a, &b| {
        let (x, y) = (&debts[a], &debts[b]);
        match strategy {
            Strategy::Avalanche => y
                .interest_rate_bp
                .cmp(&x.interest_rate_bp)
                .then(x.balance.cmp(&y.balance)),
            Strategy::Snowball => x
                .balance
                .cmp(&y.balance)
                .then(y.interest_rate_bp.cmp(&x.interest_rate_bp)),
        }
        .then(x.name.cmp(&y.name))
    });
    idx
}

/// `extra` is paid on top of all minimums every month. Minimums of paid-off
/// debts roll over to the next target, so the monthly budget stays constant.
pub fn simulate(debts: &[DebtInput], extra: i64, strategy: Strategy) -> Plan {
    let active: Vec<DebtInput> = debts.iter().filter(|d| d.balance > 0).cloned().collect();
    let order = priority_order(&active, strategy);
    let monthly_budget: i64 = active.iter().map(|d| d.min_payment).sum::<i64>() + extra.max(0);

    let mut balance: Vec<i64> = active.iter().map(|d| d.balance).collect();
    let mut interest: Vec<i64> = vec![0; active.len()];
    let mut payoff: Vec<Option<u32>> = vec![None; active.len()];
    let mut month = 0;

    while balance.iter().any(|b| *b > 0) && month < MAX_MONTHS {
        month += 1;
        for i in 0..active.len() {
            if balance[i] > 0 {
                let int = monthly_interest(balance[i], active[i].interest_rate_bp);
                balance[i] += int;
                interest[i] += int;
            }
        }
        let mut budget = monthly_budget;
        // Minimums first.
        for i in 0..active.len() {
            let pay = active[i].min_payment.min(balance[i]).min(budget);
            balance[i] -= pay;
            budget -= pay;
        }
        // Everything left goes to the priority targets in order.
        for &i in &order {
            if budget == 0 {
                break;
            }
            let pay = balance[i].min(budget);
            balance[i] -= pay;
            budget -= pay;
        }
        for i in 0..active.len() {
            if balance[i] == 0 && payoff[i].is_none() {
                payoff[i] = Some(month);
            }
        }
    }

    let feasible = balance.iter().all(|b| *b == 0);
    let mut out: Vec<DebtPayoff> = order
        .iter()
        .enumerate()
        .map(|(rank, &i)| DebtPayoff {
            id: active[i].id,
            name: active[i].name.clone(),
            payoff_month: payoff[i],
            interest_paid: interest[i],
            priority: rank + 1,
        })
        .collect();
    out.sort_by_key(|d| d.priority);

    Plan {
        strategy,
        monthly_budget,
        months: feasible.then_some(month),
        total_interest: interest.iter().sum(),
        feasible,
        debts: out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debt(name: &str, balance: i64, rate_bp: i32, min: i64) -> DebtInput {
        DebtInput {
            id: Uuid::new_v4(),
            name: name.into(),
            balance,
            interest_rate_bp: rate_bp,
            min_payment: min,
        }
    }

    #[test]
    fn zero_interest_installment() {
        let p = simulate(
            &[debt("Laptop", 3_000_000, 0, 750_000)],
            0,
            Strategy::Avalanche,
        );
        assert_eq!(p.months, Some(4));
        assert_eq!(p.total_interest, 0);
        assert!(p.feasible);
    }

    #[test]
    fn avalanche_beats_snowball_on_interest() {
        let debts = [
            debt("Card", 5_000_000, 2400, 250_000),
            debt("Friend", 1_000_000, 0, 100_000),
        ];
        let a = simulate(&debts, 500_000, Strategy::Avalanche);
        let s = simulate(&debts, 500_000, Strategy::Snowball);
        assert_eq!(a.debts[0].name, "Card");
        assert_eq!(s.debts[0].name, "Friend");
        assert!(a.total_interest <= s.total_interest);
        assert!(a.feasible && s.feasible);
    }

    #[test]
    fn minimum_below_interest_is_infeasible() {
        // 10M at 36%/yr accrues 300k/month; paying 200k never ends.
        let p = simulate(
            &[debt("Pinjol", 10_000_000, 3600, 200_000)],
            0,
            Strategy::Avalanche,
        );
        assert!(!p.feasible);
        assert_eq!(p.months, None);
        assert_eq!(p.debts[0].payoff_month, None);
    }

    #[test]
    fn interest_rounding() {
        assert_eq!(monthly_interest(1_000_000, 1200), 10_000);
        assert_eq!(monthly_interest(1_000, 1), 0);
    }
}
