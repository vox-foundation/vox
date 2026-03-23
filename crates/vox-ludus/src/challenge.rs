//! Coding challenges, leaderboards, and manager.

use serde::{Deserialize, Serialize};

/// Categories of coding challenges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChallengeType {
    /// Algorithm or data-structure puzzle.
    Algorithm,
    /// Code cleanup and restructuring task.
    Refactoring,
    /// Identify and fix an existing bug.
    Debugging,
    /// Improve performance of a given solution.
    Optimization,
    /// Fix or harden code against security threats.
    Security,
}

impl ChallengeType {
    /// Convert this challenge type to its canonical lower-case slug.
    pub fn as_str(&self) -> &'static str {
        match self {
            ChallengeType::Algorithm => "algorithm",
            ChallengeType::Refactoring => "refactoring",
            ChallengeType::Debugging => "debugging",
            ChallengeType::Optimization => "optimization",
            ChallengeType::Security => "security",
        }
    }
}

/// A coding challenge that can be attempted by users for XP and crystals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    /// Unique challenge identifier.
    pub id: String,
    /// Short display title.
    pub title: String,
    /// Full problem statement.
    pub description: String,
    /// Category of challenge.
    pub challenge_type: ChallengeType,
    /// Starter code provided to the user.
    pub base_code: String,
    /// Test cases that a valid solution must pass.
    pub test_cases: Vec<TestCase>,
    /// Crystal reward for successful completion.
    pub crystal_reward: u64,
    /// XP reward for successful completion.
    pub xp_reward: u64,
    /// Unix timestamp when the challenge expires.
    pub expires_at: i64,
}

/// A specific input/output pair or condition to test a challenge solution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Human-readable explanation of what this case tests.
    pub description: String,
    /// Input data for the test.
    pub input: String,
    /// Expected output or result.
    pub expected_output: String,
    /// Hidden test cases are not shown to the user before submission.
    pub is_hidden: bool,
}

/// A user's attempt at solving a challenge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeAttempt {
    /// Unique attempt identifier.
    pub id: String,
    /// ID of the challenge this attempt belongs to.
    pub challenge_id: String,
    /// ID of the user who submitted this attempt.
    pub user_id: String,
    /// Code submitted by the user.
    pub submitted_code: String,
    /// Whether the attempt passed all test cases.
    pub success: bool,
    /// Numeric score based on performance and efficiency.
    pub score: u32,
    /// Time taken to complete the solution in seconds.
    pub duration_secs: u64,
    /// Unix timestamp when the attempt was submitted.
    pub submitted_at: i64,
}

/// A leaderboard entry for coding challenges.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeLeaderboardEntry {
    /// ID of the user on the leaderboard.
    pub user_id: String,
    /// Challenge this ranking applies to.
    pub challenge_id: String,
    /// Score achieved in the best attempt.
    pub score: u32,
    /// Duration of the best attempt in seconds.
    pub duration_secs: u64,
}

/// Manager for generating and scoring challenges.
pub struct ChallengeManager;

impl ChallengeManager {
    /// Create a deterministic pseudo-random challenge for the given seed.
    /// Usually this would be driven by AI or a backend service.
    pub fn generate_daily_challenge(day_seed: u64) -> Challenge {
        // Fallback to deterministic static challenges for offline mode
        let types = [
            ChallengeType::Algorithm,
            ChallengeType::Refactoring,
            ChallengeType::Debugging,
            ChallengeType::Optimization,
            ChallengeType::Security,
        ];

        let c_type = types[(day_seed % 5) as usize].clone();

        Challenge {
            id: format!("daily-{}", day_seed),
            title: format!("Daily Challenge: {}", c_type.as_str()),
            description: "Solve this task to earn bonus XP and crystals!".to_string(),
            challenge_type: c_type,
            base_code: "fn solve() {\n  // your code here\n}".to_string(),
            test_cases: vec![TestCase {
                description: "Basic test".to_string(),
                input: "test".to_string(),
                expected_output: "test".to_string(),
                is_hidden: false,
            }],
            crystal_reward: 50,
            xp_reward: 200,
            expires_at: 0, // In practice, calculated from time of generation.
        }
    }

    /// Mock evaluation of an attempt.
    pub fn evaluate_attempt(_challenge: &Challenge, attempt: &ChallengeAttempt) -> bool {
        // A real system would compile/interpret the user's `submitted_code`
        // against the `test_cases` inside an isolated runtime.
        if attempt.submitted_code.contains("todo!()") {
            return false;
        }

        // Just hardcode some logic for testing
        !attempt.submitted_code.is_empty()
    }
}

// ── Tests ─────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_generation() {
        let ch1 = ChallengeManager::generate_daily_challenge(100);
        let ch2 = ChallengeManager::generate_daily_challenge(100);
        let ch3 = ChallengeManager::generate_daily_challenge(101);

        assert_eq!(ch1.id, ch2.id);
        assert_ne!(ch1.id, ch3.id);
        assert_eq!(ch1.challenge_type, ch2.challenge_type);
    }

    #[test]
    fn evaluate_mock_attempt() {
        let ch = ChallengeManager::generate_daily_challenge(1);
        let attempt = ChallengeAttempt {
            id: "a1".to_string(),
            challenge_id: ch.id.clone(),
            user_id: "user1".to_string(),
            submitted_code: "fn solve() { return; }".to_string(),
            success: false, // will re-evaluate
            score: 0,
            duration_secs: 10,
            submitted_at: 0,
        };

        assert!(ChallengeManager::evaluate_attempt(&ch, &attempt));

        let attempt_fail = ChallengeAttempt {
            submitted_code: "fn solve() { todo!() }".to_string(),
            ..attempt
        };

        assert!(!ChallengeManager::evaluate_attempt(&ch, &attempt_fail));
    }
}
