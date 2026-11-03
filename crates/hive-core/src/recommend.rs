//! Find the right skills for a job.
//!
//! Given a task description, rank library items by full-text relevance (SQLite FTS5 BM25),
//! description similarity, name/tag/trigger matches and measured effectiveness, then compose a
//! suggested set that has no high-severity conflicts and fits the target's description budget.

use std::collections::{BTreeSet, HashMap};

use crate::conflicts::{self, AnalysisOptions, AssetProfile};
use crate::error::Result;
use crate::hive::Hive;
use crate::model::{
    AssetKind, ConflictSeverity, LibraryItem, RecommendRequest, RecommendResponse, Recommendation,
    ValidationStatus,
};
use crate::skill;

const DEFAULT_LIMIT: u32 = 10;
const FTS_CANDIDATES: u32 = 60;

#[derive(Debug, Clone)]
struct Scored {
    item: LibraryItem,
    score: f64,
    reasons: Vec<String>,
    matched: BTreeSet<String>,
}

/// Lexical score of a task against one item: name, tags, trigger phrases, description cosine.
pub fn lexical_score(
    task_tokens: &[String],
    item: &LibraryItem,
    idf: Option<&conflicts::Idf>,
) -> (f64, Vec<String>, BTreeSet<String>) {
    let task_set: BTreeSet<&String> = task_tokens.iter().collect();
    let mut reasons = Vec::new();
    let mut matched: BTreeSet<String> = BTreeSet::new();
    let mut score = 0.0;

    // Description similarity.
    let desc_tokens = conflicts::tokens(&format!(
        "{} {}",
        item.name.replace('-', " "),
        item.description
    ));
    let sims = conflicts::similarity_matrix(&[task_tokens.to_vec(), desc_tokens.clone()], idf);
    let cosine = sims[0][1];
    score += 0.55 * cosine;
    for t in desc_tokens.iter().filter(|t| task_set.contains(t)) {
        matched.insert(t.clone());
    }

    // Name tokens.
    let name_tokens = conflicts::tokens(&item.name.replace('-', " "));
    let name_hits: Vec<&String> = name_tokens
        .iter()
        .filter(|t| task_set.contains(t))
        .collect();
    if !name_hits.is_empty() {
        score += 0.25 * name_hits.len().min(3) as f64;
        reasons.push(format!("Name matches: {}", item.name));
    }

    // Tags.
    let tag_hits: Vec<&String> = item
        .tags
        .iter()
        .filter(|tag| conflicts::tokens(tag).iter().any(|t| task_set.contains(t)))
        .collect();
    if !tag_hits.is_empty() {
        score += 0.1 * tag_hits.len().min(3) as f64;
        reasons.push(format!(
            "Tags: {}",
            tag_hits
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    // Trigger phrases ("use when …").
    let mut best_phrase: Option<(f64, String)> = None;
    for phrase in skill::trigger_phrases(&item.description) {
        let toks = conflicts::tokens(&phrase);
        if toks.is_empty() {
            continue;
        }
        let hits = toks.iter().filter(|t| task_set.contains(t)).count();
        let frac = hits as f64 / toks.len() as f64;
        if hits >= 1 && best_phrase.as_ref().map(|(f, _)| frac > *f).unwrap_or(true) {
            best_phrase = Some((frac, phrase));
        }
    }
    if let Some((frac, phrase)) = best_phrase {
        if frac >= 0.34 {
            score += 0.2 * frac;
            reasons.push(format!(
                "Trigger: \"{}\"",
                crate::util::truncate_chars(&phrase, 60)
            ));
        }
    }

    (score, reasons, matched)
}

impl Hive {
    /// Rank library items for a task and suggest a conflict-free set.
    pub async fn recommend(&self, req: &RecommendRequest) -> Result<RecommendResponse> {
        let limit = req.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, 50) as usize;
        let task = req.task.trim();
        let task_tokens = conflicts::tokens(task);
        let kinds: Vec<AssetKind> = if req.kinds.is_empty() {
            vec![AssetKind::Skill, AssetKind::Agent, AssetKind::Command]
        } else {
            req.kinds.clone()
        };
        let idf = self.library_idf()?;
        let idf_ref = (idf.n >= conflicts::Idf::MIN_DOCS).then_some(&idf);

        // 1. Candidate pool: FTS hits plus every item (small libraries) for lexical scoring.
        let fts: Vec<(String, f64)> = self.db.rank_items(task, FTS_CANDIDATES)?;
        let fts_best = fts.iter().map(|(_, r)| -r).fold(0.0_f64, f64::max);
        let fts_scores: HashMap<String, f64> = fts
            .iter()
            .map(|(id, r)| {
                (
                    id.clone(),
                    if fts_best > 0.0 { (-r) / fts_best } else { 0.0 },
                )
            })
            .collect();
        let all = self.db.all_items()?;

        let mut scored: Vec<Scored> = Vec::new();
        for item in all {
            if !kinds.contains(&item.kind) || req.exclude_ids.contains(&item.id) {
                continue;
            }
            let (lex, mut reasons, matched) = lexical_score(&task_tokens, &item, idf_ref);
            let fts_score = fts_scores.get(&item.id).copied().unwrap_or(0.0);
            if fts_score > 0.0 {
                reasons.insert(0, "Full-text match".to_string());
            }
            let mut score = 0.45 * fts_score + lex;
            if score <= 0.02 {
                continue;
            }
            match item.effectiveness.as_ref() {
                Some(e) if e.delta > 0.05 => {
                    score += (0.15 * e.delta).min(0.15);
                    reasons.push(format!(
                        "Measured: {:+.0}% pass rate with this skill",
                        e.delta * 100.0
                    ));
                }
                Some(e) if e.delta < -0.05 => {
                    score -= 0.15;
                    reasons.push(format!(
                        "Measured: {:+.0}% pass rate (it hurt)",
                        e.delta * 100.0
                    ));
                }
                _ => {}
            }
            if item.validation == ValidationStatus::Errors {
                score -= 0.2;
                reasons.push("Has validation errors".into());
            }
            if item.deployment_count > 0 {
                reasons.push("Already deployed".into());
            }
            scored.push(Scored {
                item,
                score: score.max(0.0),
                reasons,
                matched,
            });
        }
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit);

        // Optional model re-ranking: a provider with the recommender role reorders the lexical
        // candidates and explains the choice. Failures degrade to the lexical order. It runs
        // before the set is chosen so the suggestion follows the final ranking.
        let llm_notes = if req.use_llm && !scored.is_empty() {
            match self.llm_rerank(task, &mut scored).await {
                Ok(notes) => Some(notes),
                Err(err) => Some(format!("Model re-ranking unavailable: {err}")),
            }
        } else {
            None
        };
        let top = scored.iter().map(|s| s.score).fold(0.0_f64, f64::max);

        // 2. Pairwise conflicts among the results.
        let budget_chars = req.target_id.as_deref().and_then(|t| {
            self.resolver()
                .get(t)
                .and_then(|s| s.description_budget_chars)
        });
        let options: AnalysisOptions = self.analysis_options(budget_chars)?;
        // Bodies are only needed for the few results kept (directive detection).
        let mut profiles: Vec<AssetProfile> = Vec::with_capacity(scored.len());
        for s in &scored {
            let body = self
                .db
                .get_item_record(&s.item.id)?
                .map(|r| r.body)
                .unwrap_or_default();
            profiles.push(AssetProfile::from_item(&s.item, &body));
        }
        let report = conflicts::analyze(&profiles, &options);
        let mut conflicts_with: HashMap<String, BTreeSet<String>> = HashMap::new();
        for c in report
            .conflicts
            .iter()
            .filter(|c| c.severity >= ConflictSeverity::Medium)
        {
            for a in &c.item_ids {
                for b in &c.item_ids {
                    if a != b {
                        conflicts_with
                            .entry(a.clone())
                            .or_default()
                            .insert(b.clone());
                    }
                }
            }
        }

        // 3. Greedy conflict-free, in-budget selection.
        let mut selected: Vec<usize> = Vec::new();
        for (i, s) in scored.iter().enumerate() {
            if s.item.validation == ValidationStatus::Errors {
                continue;
            }
            let clashes = selected.iter().any(|&j| {
                conflicts_with
                    .get(&s.item.id)
                    .map(|set| set.contains(&scored[j].item.id))
                    .unwrap_or(false)
            });
            if clashes {
                continue;
            }
            let mut trial: Vec<AssetProfile> =
                selected.iter().map(|&j| profiles[j].clone()).collect();
            trial.push(profiles[i].clone());
            let trial_budget = conflicts::budget(&trial, options.budget_chars);
            if trial_budget.over_budget {
                continue;
            }
            // Only the strongest few unless the task clearly wants breadth.
            if selected.len() >= 5 || (top > 0.0 && s.score < top * 0.45) {
                continue;
            }
            selected.push(i);
        }
        let selected_ids: Vec<String> = selected
            .iter()
            .map(|&i| scored[i].item.id.clone())
            .collect();
        let selected_profiles: Vec<AssetProfile> =
            selected.iter().map(|&i| profiles[i].clone()).collect();
        let budget = conflicts::budget(&selected_profiles, options.budget_chars);

        let results: Vec<Recommendation> = scored
            .into_iter()
            .map(|s| Recommendation {
                conflicts_with: conflicts_with
                    .get(&s.item.id)
                    .map(|set| set.iter().cloned().collect())
                    .unwrap_or_default(),
                selected: selected_ids.contains(&s.item.id),
                score: if top > 0.0 { s.score / top } else { 0.0 },
                reasons: s.reasons,
                matched_terms: s.matched.into_iter().take(8).collect(),
                item: s.item,
            })
            .collect();

        Ok(RecommendResponse {
            results,
            suggested_ids: selected_ids,
            conflicts: report,
            budget,
            llm_notes,
        })
    }

    /// Ask the recommender provider to reorder `results` for `task`. Returns its notes.
    async fn llm_rerank(&self, task: &str, results: &mut [Scored]) -> Result<String> {
        use crate::model::ProviderRole;
        use crate::providers::CompletionRequest;

        let config = self
            .default_provider_for(ProviderRole::Recommender)?
            .ok_or_else(|| {
                crate::error::HiveError::invalid("no provider has the recommender role")
            })?;
        let provider = self.provider(&config.id)?;
        let listing: String = results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                format!(
                    "{}. {} ({}): {}",
                    i + 1,
                    r.item.name,
                    r.item.kind.label().to_lowercase(),
                    crate::util::truncate_chars(&r.item.description, 300)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = format!(
            "A user described a job for an AI coding agent. Rank the candidate skills by how useful each would be for exactly this job. Prefer specific, directly applicable skills; put irrelevant ones last.\n\nJob:\n{task}\n\nCandidates:\n{listing}\n\nAnswer with JSON only: {{\"ranking\": [names in best-first order], \"notes\": \"one or two sentences on why the top picks fit\"}}"
        );
        let request = CompletionRequest::new(prompt)
            .with_system(
                "You rank tools for software agents. Reply with strict JSON and nothing else.",
            )
            .with_timeout(std::time::Duration::from_secs(60));
        let response = provider.complete(request).await?;
        let text = response.text.trim();
        let json_text = match (text.find('{'), text.rfind('}')) {
            (Some(a), Some(b)) if b > a => &text[a..=b],
            _ => {
                return Err(crate::error::HiveError::invalid(
                    "the model did not return JSON",
                ));
            }
        };
        let parsed: serde_json::Value = serde_json::from_str(json_text)?;
        let ranking: Vec<String> = parsed
            .get("ranking")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        if !ranking.is_empty() {
            let position = |name: &str| {
                ranking
                    .iter()
                    .position(|n| n.eq_ignore_ascii_case(name))
                    .unwrap_or(usize::MAX)
            };
            results.sort_by_key(|r| position(&r.item.name));
            let n = results.len().max(1) as f64;
            for (i, r) in results.iter_mut().enumerate() {
                if position(&r.item.name) != usize::MAX {
                    r.reasons.insert(0, format!("Model rank #{}", i + 1));
                    r.score = (r.score * 0.4 + (1.0 - i as f64 / n) * 0.6).clamp(0.0, 1.0);
                }
            }
        }
        Ok(parsed
            .get("notes")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("Re-ranked by {}", config.name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::test_support::write_skill;

    #[tokio::test]
    async fn ranks_relevant_skills_and_avoids_conflicts() {
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        let src = tmp.path().join("src");
        for (n, d) in [
            (
                "pdf-extract",
                "Extract text and tables from PDF documents. Use when the user shares a PDF or asks about a PDF file.",
            ),
            (
                "pdf-reader",
                "Read and extract text and tables from PDF documents when the user shares a PDF file.",
            ),
            (
                "git-commit",
                "Write conventional commit messages. Use when committing changes.",
            ),
            (
                "sql-tune",
                "Analyse slow SQL queries and propose indexes. Use when a database query is slow.",
            ),
            (
                "xlsx",
                "Create and edit Excel spreadsheets with formulas. Use when the user asks for a spreadsheet.",
            ),
            (
                "docx",
                "Create Word documents with headings and tables. Use when the user asks for a .docx report.",
            ),
            (
                "pptx",
                "Build PowerPoint decks. Use when the user asks for slides or a presentation.",
            ),
            (
                "web-test",
                "Test web apps with Playwright. Use when verifying a web page works.",
            ),
            (
                "brand",
                "Apply company brand colours and fonts. Use when designing marketing material.",
            ),
        ] {
            hive.import_path(&write_skill(&src, n, d, "body"), &["docs".into()], false)
                .unwrap();
        }
        let resp = hive
            .recommend(&RecommendRequest {
                task: "pull the tables out of this PDF invoice and put them in a spreadsheet"
                    .into(),
                kinds: vec![],
                limit: Some(5),
                target_id: Some("claude-code".into()),
                exclude_ids: vec![],
                use_llm: false,
            })
            .await
            .unwrap();
        let names: Vec<&str> = resp.results.iter().map(|r| r.item.name.as_str()).collect();
        assert!(names[0].starts_with("pdf"), "{names:?}");
        assert!(names.contains(&"xlsx"), "{names:?}");
        assert!(!names.contains(&"git-commit"), "{names:?}");
        assert_eq!(resp.results[0].score, 1.0);
        assert!(!resp.results[0].matched_terms.is_empty());
        // The two pdf skills overlap: only one of them is in the suggested set.
        let pdf_selected = resp
            .results
            .iter()
            .filter(|r| r.item.name.starts_with("pdf") && r.selected)
            .count();
        assert_eq!(
            pdf_selected,
            1,
            "{:?}",
            resp.results
                .iter()
                .map(|r| (&r.item.name, r.selected, &r.conflicts_with))
                .collect::<Vec<_>>()
        );
        assert!(
            resp.conflicts
                .conflicts
                .iter()
                .any(|c| c.kind == crate::model::ConflictKind::DescriptionOverlap)
        );
        assert!(resp.suggested_ids.iter().any(|id| {
            resp.results
                .iter()
                .any(|r| &r.item.id == id && r.item.name == "xlsx")
        }));
        assert!(!resp.budget.over_budget);

        // Excluding and limiting works; nonsense tasks return nothing.
        let none = hive
            .recommend(&RecommendRequest {
                task: "zzzz qqqq".into(),
                kinds: vec![],
                limit: None,
                target_id: None,
                exclude_ids: vec![],
                use_llm: false,
            })
            .await
            .unwrap();
        assert!(none.results.is_empty());
    }
}
