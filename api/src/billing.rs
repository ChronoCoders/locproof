//! Stripe Price ID ↔ plan mapping, loaded from env at boot. The webhook
//! resolves a subscription's price to a plan name; checkout resolves a plan
//! to the Price ID to bill. Free and enterprise have no self-serve price:
//! free is the default, enterprise is sales-assisted.

#[derive(Clone, Default)]
pub struct StripePrices {
    starter: Option<String>,
    growth: Option<String>,
}

impl StripePrices {
    pub fn from_env() -> Self {
        Self {
            starter: std::env::var("STRIPE_PRICE_STARTER").ok(),
            growth: std::env::var("STRIPE_PRICE_GROWTH").ok(),
        }
    }

    /// Price ID for a self-serve plan, or `None` if the plan isn't billable
    /// via checkout (free/enterprise) or its price isn't configured.
    pub fn price_for_plan(&self, plan: &str) -> Option<&str> {
        match plan {
            "starter" => self.starter.as_deref(),
            "growth" => self.growth.as_deref(),
            _ => None,
        }
    }

    /// Plan name for a Stripe Price ID, or `None` if it maps to no known plan
    /// (e.g. a price created outside this configuration).
    pub fn plan_for_price(&self, price_id: &str) -> Option<&'static str> {
        if self.starter.as_deref() == Some(price_id) {
            Some("starter")
        } else if self.growth.as_deref() == Some(price_id) {
            Some("growth")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_configured_plans() {
        let prices = StripePrices {
            starter: Some("price_starter".into()),
            growth: Some("price_growth".into()),
        };
        assert_eq!(prices.price_for_plan("starter"), Some("price_starter"));
        assert_eq!(prices.price_for_plan("growth"), Some("price_growth"));
        assert_eq!(prices.plan_for_price("price_starter"), Some("starter"));
        assert_eq!(prices.plan_for_price("price_growth"), Some("growth"));
    }

    #[test]
    fn unbillable_and_unknown_map_to_none() {
        let prices = StripePrices {
            starter: Some("price_starter".into()),
            growth: None,
        };
        assert_eq!(prices.price_for_plan("free"), None);
        assert_eq!(prices.price_for_plan("enterprise"), None);
        assert_eq!(prices.price_for_plan("growth"), None);
        assert_eq!(prices.plan_for_price("price_unknown"), None);
    }
}
