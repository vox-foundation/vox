//! Periodic unique reward system.
//!
//! Generates a deterministic weekly rotating pool of 30 Roman-themed
//! rewards from a fixed pool. Users may claim one reward per day.
//! Random-drop rewards fire probabilistically on qualifying build events.

mod evaluate;
mod pool;
mod rotation;
mod types;

pub use evaluate::evaluate_condition;
pub use rotation::{
    current_week_number, current_weekly_reward, evaluate_random_drop, generate_weekly_reward,
};
pub use types::{PeriodicCondition, PeriodicReward, PeriodicRewardParams};

#[cfg(test)]
mod tests {
    use super::pool::REWARD_POOL;
    use super::*;

    #[test]
    fn pool_contains_thirty_entries() {
        assert_eq!(REWARD_POOL.len(), 30);
    }

    #[test]
    fn generate_weekly_reward_deterministic() {
        let r1 = generate_weekly_reward("user-abc", 42, 0);
        let r2 = generate_weekly_reward("user-abc", 42, 0);
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn different_users_may_differ() {
        let r1 = generate_weekly_reward("user-alice", 1, 0);
        let r2 = generate_weekly_reward("user-bob", 1, 0);
        assert!(!r1.name.is_empty());
        assert!(!r2.name.is_empty());
    }

    #[test]
    fn different_weeks_produce_different_rewards() {
        let r1 = generate_weekly_reward("user-x", 10, 0);
        let r2 = generate_weekly_reward("user-x", 11, 0);
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn random_drop_fires_at_threshold() {
        assert!(evaluate_random_drop(0.02, 0.019));
        assert!(!evaluate_random_drop(0.02, 0.021));
    }

    #[test]
    fn random_drop_clamps_probability() {
        assert!(evaluate_random_drop(1.5, 0.999));
        assert!(!evaluate_random_drop(-0.5, 0.001));
    }

    #[test]
    fn claim_marks_redeemed() {
        let mut r = generate_weekly_reward("user-1", 1, 0);
        assert!(!r.redeemed);
        r.claim();
        assert!(r.redeemed);
    }

    #[test]
    fn is_expired_logic() {
        let r = generate_weekly_reward("user-1", 1, 1_000);
        assert!(r.is_expired(2_000));
        assert!(!r.is_expired(500));
    }

    #[test]
    fn no_expiry_when_valid_until_zero() {
        let r = generate_weekly_reward("user-1", 1, 0);
        assert!(!r.is_expired(i64::MAX));
    }
}
