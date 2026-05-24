//! Plan → monthly quota mapping. Single source of truth for billing tiers.
//! 4c will read this in the `POST /v1/proofs` quota check; 4a uses it for
//! the `GET /v1/usage` response.

/// Monthly proof quota for the given plan string (the value stored in
/// `customers.plan`). Unknown plan strings fall back to `free` so a typo'd
/// row can't accidentally grant unlimited proofs.
///
/// Invariant: every quota is `>= 1`. `db::record_usage_within_quota` applies
/// its `< quota` guard only on the `ON CONFLICT` update path; the first insert
/// of a month always records count 1, which is correct precisely because no
/// plan has a quota below 1.
pub fn quota_for(plan: &str) -> u32 {
    match plan {
        "free" => 100,
        "starter" => 5_000,
        "growth" => 25_000,
        "enterprise" => u32::MAX,
        _ => 100,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_plans() {
        assert_eq!(quota_for("free"), 100);
        assert_eq!(quota_for("starter"), 5_000);
        assert_eq!(quota_for("growth"), 25_000);
        assert_eq!(quota_for("enterprise"), u32::MAX);
    }

    #[test]
    fn unknown_plan_falls_back_to_free() {
        assert_eq!(quota_for("bogus"), 100);
        assert_eq!(quota_for(""), 100);
    }
}
