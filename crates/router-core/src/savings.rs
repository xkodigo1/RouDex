//! Estimated savings of routing (spec 005, D5): what the turns of the last
//! days cost with the prices the user set in `router.toml`, what the same
//! tokens would have cost on the `fuerte` model, and what the classifier
//! itself cost. RouDex ships no prices; a model without one is left out and
//! counted apart.

use crate::chat::TokenCounts;
use crate::persistence::{RouterStore, StoreError};
use crate::tiers::{ModelPrice, RouterConfig, Tier};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Days the summary covers, today included.
pub const SAVINGS_DAYS: u32 = 30;

/// Where a turn came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UsageSource {
    Chat,
    Tasks,
    CodexDesktop,
}

/// One turn's tokens, as the summary needs them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    pub source: UsageSource,
    /// Local date, `YYYY-MM-DD`.
    pub day: String,
    pub model: String,
    pub tier: Option<Tier>,
    pub tokens: TokenCounts,
}

/// Cost in USD of `tokens` at `price` (USD per million tokens). Input
/// includes the cached part and output includes reasoning, as in Codex's
/// counters, so cached tokens are charged only at the cached price.
pub fn cost(tokens: &TokenCounts, price: &ModelPrice) -> f64 {
    let cached = tokens.cached_input.clamp(0, tokens.input.max(0));
    let fresh = tokens.input.max(0) - cached;
    (fresh as f64 * price.input
        + cached as f64 * price.cached_input
        + tokens.output.max(0) as f64 * price.output)
        / 1_000_000.0
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSavings {
    pub source: Option<UsageSource>,
    pub turns: i64,
    /// Turns whose model and the `fuerte` model both have a price.
    pub priced_turns: i64,
    pub tokens: TokenCounts,
    pub cost: f64,
    /// The priced turns' tokens at the `fuerte` model's price.
    pub baseline: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouterOverhead {
    pub turns: i64,
    pub priced_turns: i64,
    pub tokens: TokenCounts,
    pub cost: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TierTokens {
    pub tier: Option<Tier>,
    pub turns: i64,
    pub tokens: TokenCounts,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySavings {
    pub date: String,
    pub cost: f64,
    pub baseline: f64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavingsSummary {
    pub days: u32,
    /// The `fuerte` model the baseline is priced at, and whether it has a price.
    pub baseline_model: String,
    pub baseline_priced: bool,
    /// Models with a price in `router.toml`.
    pub priced_models: usize,
    pub sources: Vec<SourceSavings>,
    pub router: RouterOverhead,
    pub cost: f64,
    pub baseline: f64,
    /// Baseline − cost − router overhead.
    pub savings: f64,
    pub unpriced_turns: i64,
    pub unpriced_models: Vec<String>,
    pub tiers: Vec<TierTokens>,
    pub by_day: Vec<DaySavings>,
}

/// Add up `turns` and the classifier's `router` turns with `config`'s prices.
pub fn summarize(turns: &[Usage], router: &[Usage], config: &RouterConfig) -> SavingsSummary {
    let baseline_model = config.tiers.fuerte.model.clone();
    let baseline_price = config.prices.get(&baseline_model);
    let mut sources = BTreeMap::<UsageSource, SourceSavings>::new();
    let mut tiers = BTreeMap::<Option<Tier>, TierTokens>::new();
    let mut days = BTreeMap::<String, DaySavings>::new();
    let mut unpriced = BTreeSet::new();
    let mut unpriced_turns = 0;
    for turn in turns {
        let source = sources.entry(turn.source).or_default();
        source.source = Some(turn.source);
        source.turns += 1;
        add(&mut source.tokens, &turn.tokens);
        let tier = tiers.entry(turn.tier).or_default();
        tier.tier = turn.tier;
        tier.turns += 1;
        add(&mut tier.tokens, &turn.tokens);
        match (config.prices.get(&turn.model), baseline_price) {
            (Some(price), Some(strong)) => {
                let (spent, would) = (cost(&turn.tokens, price), cost(&turn.tokens, strong));
                source.priced_turns += 1;
                source.cost += spent;
                source.baseline += would;
                let day = days.entry(turn.day.clone()).or_default();
                day.date = turn.day.clone();
                day.cost += spent;
                day.baseline += would;
            }
            (price, _) => {
                unpriced_turns += 1;
                if price.is_none() {
                    unpriced.insert(turn.model.clone());
                }
            }
        }
    }
    let mut overhead = RouterOverhead::default();
    for turn in router {
        overhead.turns += 1;
        add(&mut overhead.tokens, &turn.tokens);
        match config.prices.get(&turn.model) {
            Some(price) => {
                let spent = cost(&turn.tokens, price);
                overhead.priced_turns += 1;
                overhead.cost += spent;
                let day = days.entry(turn.day.clone()).or_default();
                day.date = turn.day.clone();
                day.cost += spent;
            }
            None => {
                unpriced.insert(turn.model.clone());
            }
        }
    }
    let sources: Vec<_> = sources.into_values().collect();
    let cost_sum: f64 = sources.iter().map(|source| source.cost).sum();
    let baseline_sum: f64 = sources.iter().map(|source| source.baseline).sum();
    if baseline_price.is_none() && !turns.is_empty() {
        unpriced.insert(baseline_model.clone());
    }
    SavingsSummary {
        days: SAVINGS_DAYS,
        baseline_priced: baseline_price.is_some(),
        baseline_model,
        priced_models: config.prices.len(),
        sources,
        savings: baseline_sum - cost_sum - overhead.cost,
        router: overhead,
        cost: cost_sum,
        baseline: baseline_sum,
        unpriced_turns,
        unpriced_models: unpriced.into_iter().collect(),
        tiers: tiers.into_values().collect(),
        by_day: days.into_values().collect(),
    }
}

fn add(total: &mut TokenCounts, tokens: &TokenCounts) {
    total.input += tokens.input;
    total.cached_input += tokens.cached_input;
    total.output += tokens.output;
    total.reasoning += tokens.reasoning;
    total.total += tokens.total;
}

const DAY: &str = "date(CAST(created_at AS INTEGER), 'unixepoch', 'localtime')";

impl RouterStore {
    /// Chat and task turns, and the classifier's turns, of the last
    /// [`SAVINGS_DAYS`] days.
    pub fn usage(&self) -> Result<(Vec<Usage>, Vec<Usage>), StoreError> {
        let window = format!("{DAY} > date('now', 'localtime', '-{SAVINGS_DAYS} days')");
        let tokens = |row: &rusqlite::Row, first: usize| -> rusqlite::Result<TokenCounts> {
            Ok(TokenCounts {
                input: row.get(first)?,
                cached_input: row.get(first + 1)?,
                output: row.get(first + 2)?,
                reasoning: row.get(first + 3)?,
                total: row.get(first + 4)?,
            })
        };
        let mut turns = self
            .connection
            .prepare(&format!(
                "SELECT {DAY}, model, tier, input_tokens, cached_input_tokens, output_tokens,
                        reasoning_tokens, total_tokens
                 FROM chat_turns WHERE total_tokens > 0 AND {window}"
            ))?
            .query_map([], |row| {
                Ok(Usage {
                    source: UsageSource::Chat,
                    day: row.get(0)?,
                    model: row.get(1)?,
                    tier: Tier::parse(&row.get::<_, String>(2)?),
                    tokens: tokens(row, 3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        // Task runs keep Codex's token notifications as evidence; the last one
        // holds the thread's total.
        let runs = self
            .connection
            .prepare(&format!(
                "SELECT {DAY}, model, profile,
                        (SELECT payload FROM events e
                         WHERE e.run_id = runs.run_id AND e.event_type = 'thread/tokenUsage/updated'
                         ORDER BY e.id DESC LIMIT 1)
                 FROM runs WHERE {window}"
            ))?
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        for (day, model, profile, payload) in runs {
            let Some(payload) = payload else { continue };
            let usage: Value = serde_json::from_str(&payload).unwrap_or_default();
            let tokens = TokenCounts::from_breakdown(&usage["tokenUsage"]["total"]);
            if tokens.total > 0 {
                turns.push(Usage {
                    source: UsageSource::Tasks,
                    day,
                    model,
                    tier: Tier::parse(&profile),
                    tokens,
                });
            }
        }
        let router = self
            .connection
            .prepare(&format!(
                "SELECT {DAY}, model, input_tokens, cached_input_tokens, output_tokens,
                        reasoning_tokens, total_tokens
                 FROM router_usage WHERE {window}"
            ))?
            .query_map([], |row| {
                Ok(Usage {
                    source: UsageSource::Chat,
                    day: row.get(0)?,
                    model: row.get(1)?,
                    tier: None,
                    tokens: tokens(row, 2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok((turns, router))
    }

    /// The summary; with `codex_sessions`, Codex Desktop turns from that
    /// folder are read (only new or changed files) and added as their own
    /// source. Without it, no session file is opened.
    pub fn savings(
        &self,
        config: &RouterConfig,
        codex_sessions: Option<&std::path::Path>,
    ) -> Result<SavingsSummary, StoreError> {
        let (mut turns, router) = self.usage()?;
        if let Some(dir) = codex_sessions {
            self.refresh_codex_sessions(dir, std::time::SystemTime::now())?;
            turns.extend(self.codex_desktop_usage()?);
        }
        Ok(summarize(&turns, &router, config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(input: i64, cached: i64, output: i64) -> TokenCounts {
        TokenCounts {
            input,
            cached_input: cached,
            output,
            reasoning: 0,
            total: input + output,
        }
    }

    fn price(input: f64, cached_input: f64, output: f64) -> ModelPrice {
        ModelPrice {
            input,
            cached_input,
            output,
        }
    }

    fn priced() -> RouterConfig {
        let mut config = RouterConfig::default();
        config
            .prices
            .insert("gpt-6.1-sol".into(), price(2.0, 0.5, 10.0));
        config
            .prices
            .insert("gpt-6-luna".into(), price(0.5, 0.1, 2.0));
        config
    }

    fn turn(source: UsageSource, model: &str, tier: Tier, tokens: TokenCounts) -> Usage {
        Usage {
            source,
            day: "2026-10-14".into(),
            model: model.into(),
            tier: Some(tier),
            tokens,
        }
    }

    #[test]
    fn the_store_adds_chat_turns_task_runs_and_the_classifier() {
        let store = RouterStore::in_memory().unwrap();
        let now = crate::persistence::now_string();
        let old = (now.parse::<i64>().unwrap() - 40 * 86_400).to_string();
        let connection = &store.connection;
        connection
            .execute(
                "INSERT INTO chat_threads (thread_id, cwd, title, created_at, updated_at)
                 VALUES ('t', '.', 't', ?1, ?1)",
                [&now],
            )
            .unwrap();
        for (turn, created, total) in [
            ("u1", &now, 1_100_000),
            ("u2", &old, 1_100_000),
            ("u3", &now, 0),
        ] {
            connection
                .execute(
                    "INSERT INTO chat_turns (turn_id, thread_id, prompt, tier, model, effort, source,
                         reason, status, input_tokens, cached_input_tokens, output_tokens, total_tokens,
                         created_at)
                     VALUES (?1, 't', 'p', 'economico', 'gpt-6-luna', 'high', 'rule', 'r', 'completed',
                         1000000, 0, 100000, ?2, ?3)",
                    rusqlite::params![turn, total, created],
                )
                .unwrap();
        }
        connection
            .execute(
                "INSERT INTO runs (run_id, route_id, session_id, turn_id, cwd, prompt, profile, model,
                     effort, status, created_at)
                 VALUES ('r1', 'route', 'spec', 'T-1', '.', 'p', 'fuerte', 'gpt-6.1-sol', 'high',
                     'completed', ?1)",
                [&now],
            )
            .unwrap();
        for total in [500, 1_100_000] {
            let payload = serde_json::json!({ "threadId": "x", "tokenUsage": { "total": {
                "inputTokens": 1_000_000, "cachedInputTokens": 0, "outputTokens": 100_000,
                "reasoningOutputTokens": 0, "totalTokens": total } } });
            connection
                .execute(
                    "INSERT INTO events (run_id, event_type, payload, created_at)
                     VALUES ('r1', 'thread/tokenUsage/updated', ?1, ?2)",
                    rusqlite::params![payload.to_string(), now],
                )
                .unwrap();
        }
        store
            .record_router_usage(&crate::classifier::RouterUsage {
                model: "gpt-6-luna".into(),
                effort: "low".into(),
                tokens: tokens(200_000, 0, 50_000),
            })
            .unwrap();

        let (turns, router) = store.usage().unwrap();
        assert_eq!(
            turns.len(),
            2,
            "the old turn and the empty one are left out"
        );
        let task = turns
            .iter()
            .find(|turn| turn.source == UsageSource::Tasks)
            .unwrap();
        assert_eq!(task.tokens.total, 1_100_000, "the last notification wins");
        assert_eq!(task.tier, Some(Tier::Fuerte));
        assert_eq!(router.len(), 1);

        let summary = store.savings(&priced(), None).unwrap();
        // Chat 0.7 vs 3.0, task 3.0 vs 3.0, classifier 0.2.
        assert!((summary.cost - 3.7).abs() < 1e-9, "{}", summary.cost);
        assert!(
            (summary.savings - (6.0 - 3.7 - 0.2)).abs() < 1e-9,
            "{}",
            summary.savings
        );
    }

    #[test]
    fn cached_input_is_charged_at_its_own_price() {
        // 1M input of which 400k cached, 100k output.
        let spent = cost(&tokens(1_000_000, 400_000, 100_000), &price(2.0, 0.5, 10.0));
        assert!(
            (spent - (0.6 * 2.0 + 0.4 * 0.5 + 0.1 * 10.0)).abs() < 1e-9,
            "{spent}"
        );
        // More cached than input cannot charge negative fresh input.
        assert!(cost(&tokens(10, 50, 0), &price(1.0, 1.0, 1.0)) >= 0.0);
    }

    #[test]
    fn savings_compare_each_turn_with_the_strong_model() {
        let turns = [
            turn(
                UsageSource::Chat,
                "gpt-6-luna",
                Tier::Economico,
                tokens(1_000_000, 0, 100_000),
            ),
            turn(
                UsageSource::Tasks,
                "gpt-6.1-sol",
                Tier::Fuerte,
                tokens(1_000_000, 0, 100_000),
            ),
        ];
        let summary = summarize(&turns, &[], &priced());
        // luna: 0.5 + 0.2 = 0.7; sol: 2.0 + 1.0 = 3.0.
        assert!((summary.cost - 3.7).abs() < 1e-9, "{}", summary.cost);
        assert!(
            (summary.baseline - 6.0).abs() < 1e-9,
            "{}",
            summary.baseline
        );
        assert!((summary.savings - 2.3).abs() < 1e-9, "{}", summary.savings);
        assert_eq!(summary.sources.len(), 2);
        assert_eq!(summary.unpriced_turns, 0);
        assert_eq!(summary.by_day.len(), 1);
        assert!(summary.baseline_priced);
    }

    #[test]
    fn the_classifier_counts_against_the_savings() {
        let turns = [turn(
            UsageSource::Chat,
            "gpt-6-luna",
            Tier::Economico,
            tokens(1_000_000, 0, 0),
        )];
        let router = [turn(
            UsageSource::Chat,
            "gpt-6-luna",
            Tier::Economico,
            tokens(200_000, 0, 50_000),
        )];
        let summary = summarize(&turns, &router, &priced());
        // Turn: 0.5 vs 2.0 at sol; classifier: 0.1 + 0.1 = 0.2.
        assert!(
            (summary.router.cost - 0.2).abs() < 1e-9,
            "{}",
            summary.router.cost
        );
        assert!(
            (summary.savings - (2.0 - 0.5 - 0.2)).abs() < 1e-9,
            "{}",
            summary.savings
        );
    }

    #[test]
    fn without_prices_no_money_is_estimated_but_tokens_by_tier_are_kept() {
        let turns = [
            turn(
                UsageSource::Chat,
                "gpt-6-luna",
                Tier::Economico,
                tokens(100, 0, 10),
            ),
            turn(
                UsageSource::Chat,
                "gpt-6.1-sol",
                Tier::Fuerte,
                tokens(300, 0, 30),
            ),
        ];
        let summary = summarize(&turns, &[], &RouterConfig::default());
        assert_eq!(
            (summary.cost, summary.baseline, summary.savings),
            (0.0, 0.0, 0.0)
        );
        assert_eq!(summary.unpriced_turns, 2);
        assert!(!summary.baseline_priced);
        assert_eq!(summary.unpriced_models, ["gpt-6-luna", "gpt-6.1-sol"]);
        assert_eq!(summary.tiers.len(), 2);
        assert_eq!(summary.tiers[1].tokens.total, 330);

        // A model without a price is counted apart; the rest still adds up.
        let mut partial = priced();
        partial.prices.remove("gpt-6-luna");
        let summary = summarize(&turns, &[], &partial);
        assert_eq!(summary.unpriced_turns, 1);
        assert_eq!(summary.unpriced_models, ["gpt-6-luna"]);
        assert_eq!(summary.sources[0].priced_turns, 1);
    }
}
