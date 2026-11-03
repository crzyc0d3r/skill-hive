//! Conflict analysis for a set of skills that would share one agent.
//!
//! Agents choose a skill from the list of names and descriptions in their prompt. When two
//! descriptions say the same thing, the agent picks one at random; when the list is longer than
//! the agent's budget, the tail is silently dropped. This module finds both problems before a
//! deployment happens.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::model::{
    AssetKind, BudgetSummary, Conflict, ConflictKind, ConflictReport, ConflictSeverity,
    LibraryItem, SkillFrontmatter,
};
use crate::skill;

/// What the analyser needs to know about one asset. Built from library items or installed assets.
#[derive(Debug, Clone)]
pub struct AssetProfile {
    /// Library id when known; empty for unmanaged installs.
    pub id: String,
    pub kind: AssetKind,
    pub name: String,
    pub description: String,
    /// First part of the body (instructions), used for directive detection.
    pub body_head: String,
    pub content_hash: String,
    pub allowed_tools: Vec<String>,
    pub description_tokens: u32,
    pub body_tokens: u32,
}

impl AssetProfile {
    pub fn from_item(item: &LibraryItem, body: &str) -> Self {
        let fm: SkillFrontmatter = item
            .frontmatter
            .as_object()
            .map(skill::skill_frontmatter_from_json)
            .unwrap_or_default();
        Self {
            id: item.id.clone(),
            kind: item.kind,
            name: item.name.clone(),
            description: item.description.clone(),
            body_head: body.chars().take(4000).collect(),
            content_hash: item.content_hash.clone(),
            allowed_tools: fm.allowed_tools,
            description_tokens: item.description_tokens,
            body_tokens: item.token_estimate,
        }
    }
}

/// Document frequencies from a background corpus (normally the whole library), so that common
/// words weigh less than rare ones even when only two skills are compared.
#[derive(Debug, Clone, Default)]
pub struct Idf {
    pub df: HashMap<String, usize>,
    pub n: usize,
}

impl Idf {
    pub fn from_texts<'a>(texts: impl IntoIterator<Item = &'a str>) -> Self {
        let mut df: HashMap<String, usize> = HashMap::new();
        let mut n = 0;
        for text in texts {
            n += 1;
            for t in tokens(text).into_iter().collect::<BTreeSet<_>>() {
                *df.entry(t).or_insert(0) += 1;
            }
        }
        Self { df, n }
    }

    /// Below this many background documents the statistics are noise; fall back to plain TF.
    pub const MIN_DOCS: usize = 8;

    fn weight(&self, term: &str) -> f64 {
        if self.n < Self::MIN_DOCS {
            return 1.0;
        }
        let d = self.df.get(term).copied().unwrap_or(0) as f64;
        ((self.n as f64 + 1.0) / (d + 0.5)).ln().max(0.0) + 1.0
    }
}

#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    /// Cosine similarity above which two descriptions overlap.
    pub overlap_threshold: f64,
    /// Character budget for the list of descriptions; `None` skips the budget check.
    pub budget_chars: Option<u32>,
    /// Background corpus statistics; plain term-frequency cosine when `None`.
    pub idf: Option<Idf>,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            overlap_threshold: crate::settings::DEFAULT_OVERLAP_THRESHOLD,
            budget_chars: Some(crate::targets::CLAUDE_CODE_DESCRIPTION_BUDGET_CHARS),
            idf: None,
        }
    }
}

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "are", "as", "at", "be", "by", "for", "from", "has", "in", "is", "it", "its",
    "of", "on", "or", "that", "the", "this", "to", "use", "used", "using", "when", "with", "you",
    "your", "will", "can", "should", "into", "any", "all", "also", "than", "then", "them", "they",
    "their", "there", "these", "those", "about", "after", "before", "which", "while", "would",
    "could", "each", "other", "such", "only", "more", "most", "some", "skill", "user", "asks",
    "ask", "asked", "wants", "want", "needs", "need", "help", "helps", "task", "tasks", "work",
    "working", "file", "files", "code", "project", "run", "running", "make", "create",
];

/// Lowercased alphanumeric tokens with stop words removed and a light stemmer applied.
pub fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(|t| t.to_lowercase())
        .filter(|t| t.len() >= 3 && !STOPWORDS.contains(&t.as_str()))
        .map(|t| stem(&t))
        .collect()
}

/// A crude but consistent normaliser: plural, `-ing`, `-ed` and a trailing `e` are removed so
/// `share`, `shared`, `sharing` and `shares` all map to the same stem.
pub fn stem(word: &str) -> String {
    let mut w = word.to_string();
    if w.len() > 4 && w.ends_with("ies") {
        w.truncate(w.len() - 3);
        w.push('y');
    } else if w.len() > 5
        && (w.ends_with("sses")
            || w.ends_with("xes")
            || w.ends_with("zes")
            || w.ends_with("ches")
            || w.ends_with("shes"))
    {
        w.truncate(w.len() - 2);
    } else if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") && !w.ends_with("us") {
        w.truncate(w.len() - 1);
    }
    if w.len() > 5 && w.ends_with("ing") {
        w.truncate(w.len() - 3);
    } else if w.len() > 4 && w.ends_with("ed") {
        w.truncate(w.len() - 2);
    }
    let bytes = w.as_bytes();
    if bytes.len() > 3
        && bytes[bytes.len() - 1] == bytes[bytes.len() - 2]
        && !matches!(bytes[bytes.len() - 1], b'l' | b's' | b'z')
    {
        w.truncate(w.len() - 1);
    }
    if w.len() > 4 && w.ends_with('e') {
        w.truncate(w.len() - 1);
    }
    w
}

fn term_freq(tokens: &[String]) -> HashMap<String, f64> {
    let mut map: HashMap<String, f64> = HashMap::new();
    for t in tokens {
        *map.entry(t.clone()).or_insert(0.0) += 1.0;
    }
    map
}

/// Cosine similarity matrix over the given token lists (log-TF, optionally IDF weighted).
pub fn similarity_matrix(docs: &[Vec<String>], idf: Option<&Idf>) -> Vec<Vec<f64>> {
    let n = docs.len();
    let tfs: Vec<HashMap<String, f64>> = docs.iter().map(|d| term_freq(d)).collect();
    let weight = |term: &str| -> f64 { idf.map(|i| i.weight(term)).unwrap_or(1.0) };
    let vectors: Vec<HashMap<&str, f64>> = tfs
        .iter()
        .map(|tf| {
            tf.iter()
                .map(|(term, count)| (term.as_str(), (1.0 + count.ln()) * weight(term)))
                .collect()
        })
        .collect();
    let norms: Vec<f64> = vectors
        .iter()
        .map(|v| v.values().map(|x| x * x).sum::<f64>().sqrt())
        .collect();
    let mut matrix = vec![vec![0.0; n]; n];
    for i in 0..n {
        matrix[i][i] = 1.0;
        for j in (i + 1)..n {
            if norms[i] == 0.0 || norms[j] == 0.0 {
                continue;
            }
            let (small, large) = if vectors[i].len() <= vectors[j].len() {
                (&vectors[i], &vectors[j])
            } else {
                (&vectors[j], &vectors[i])
            };
            let dot: f64 = small
                .iter()
                .filter_map(|(term, w)| large.get(term).map(|w2| w * w2))
                .sum();
            let sim = dot / (norms[i] * norms[j]);
            matrix[i][j] = sim;
            matrix[j][i] = sim;
        }
    }
    matrix
}

/// Terms two token lists share, most informative first (by rarity across the whole set),
/// rendered with the surface form they had in the text.
fn shared_terms(
    a: &[String],
    b: &[String],
    df: &HashMap<String, usize>,
    surface: &HashMap<String, String>,
) -> Vec<String> {
    let sa: BTreeSet<&String> = a.iter().collect();
    let sb: BTreeSet<&String> = b.iter().collect();
    let mut shared: Vec<String> = sa.intersection(&sb).map(|s| (*s).clone()).collect();
    shared.sort_by_key(|t| df.get(t).copied().unwrap_or(0));
    shared.truncate(6);
    shared
        .into_iter()
        .map(|s| surface.get(&s).cloned().unwrap_or(s))
        .collect()
}

/// Map from stem to the first surface word that produced it.
fn surface_forms(texts: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for text in texts {
        for word in text.split(|c: char| !c.is_alphanumeric()) {
            let lower = word.to_lowercase();
            if lower.len() >= 3 && !STOPWORDS.contains(&lower.as_str()) {
                map.entry(stem(&lower)).or_insert(lower);
            }
        }
    }
    map
}

/// Directive phrases ("always X", "never X") used to spot contradictions.
fn directives(body: &str) -> Vec<(bool, String)> {
    let mut out = Vec::new();
    for line in body.lines() {
        let lower = line
            .trim()
            .trim_start_matches(['-', '*', '•'])
            .trim()
            .to_lowercase();
        for (positive, marker) in [
            (true, "always "),
            (false, "never "),
            (false, "do not "),
            (false, "don't "),
        ] {
            if let Some(rest) = lower.strip_prefix(marker) {
                let subject: Vec<String> = tokens(rest).into_iter().take(4).collect();
                if subject.len() >= 2 {
                    out.push((positive, subject.join(" ")));
                }
            }
        }
    }
    out
}

/// Budget summary for a set of profiles.
pub fn budget(profiles: &[AssetProfile], budget_chars: Option<u32>) -> BudgetSummary {
    let description_tokens: u32 = profiles.iter().map(|p| p.description_tokens).sum();
    let description_chars: u32 = profiles
        .iter()
        .map(|p| (p.name.chars().count() + p.description.chars().count() + 4) as u32)
        .sum();
    let body_tokens: u32 = profiles.iter().map(|p| p.body_tokens).sum();
    let used_fraction = budget_chars.map(|b| description_chars as f64 / b.max(1) as f64);
    BudgetSummary {
        description_tokens,
        description_chars,
        body_tokens,
        budget_chars,
        used_fraction,
        over_budget: used_fraction.map(|f| f > 1.0).unwrap_or(false),
    }
}

/// Analyse a set of assets that would be installed together.
pub fn analyze(profiles: &[AssetProfile], options: &AnalysisOptions) -> ConflictReport {
    let mut conflicts: Vec<Conflict> = Vec::new();

    // 1. Name collisions (per kind) and duplicate content.
    let mut by_name: BTreeMap<(AssetKind, String), Vec<usize>> = BTreeMap::new();
    let mut by_hash: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, p) in profiles.iter().enumerate() {
        by_name
            .entry((p.kind, p.name.to_lowercase()))
            .or_default()
            .push(i);
        if !p.content_hash.is_empty() {
            by_hash.entry(p.content_hash.clone()).or_default().push(i);
        }
    }
    for ((kind, name), idx) in &by_name {
        if idx.len() > 1 {
            conflicts.push(Conflict {
                kind: ConflictKind::NameCollision,
                severity: ConflictSeverity::High,
                item_ids: idx.iter().map(|&i| profiles[i].id.clone()).filter(|id| !id.is_empty()).collect(),
                names: idx.iter().map(|&i| profiles[i].name.clone()).collect(),
                detail: format!(
                    "{} {}s are named `{name}`; only one can exist in a directory and the agent cannot tell them apart",
                    idx.len(),
                    kind.label().to_lowercase()
                ),
                similarity: None,
                shared_terms: Vec::new(),
            });
        }
    }
    for idx in by_hash.values() {
        if idx.len() > 1 {
            let names: BTreeSet<String> = idx
                .iter()
                .map(|&i| profiles[i].name.to_lowercase())
                .collect();
            if names.len() > 1 {
                conflicts.push(Conflict {
                    kind: ConflictKind::DuplicateContent,
                    severity: ConflictSeverity::Medium,
                    item_ids: idx
                        .iter()
                        .map(|&i| profiles[i].id.clone())
                        .filter(|id| !id.is_empty())
                        .collect(),
                    names: idx.iter().map(|&i| profiles[i].name.clone()).collect(),
                    detail: "Identical content under different names; keep one".to_string(),
                    similarity: Some(1.0),
                    shared_terms: Vec::new(),
                });
            }
        }
    }

    // 2. Description overlap (cosine) and trigger overlap.
    let texts: Vec<String> = profiles
        .iter()
        .map(|p| format!("{} {}", p.name.replace('-', " "), p.description))
        .collect();
    let docs: Vec<Vec<String>> = texts.iter().map(|t| tokens(t)).collect();
    let surface = surface_forms(&texts);
    let mut df: HashMap<String, usize> = HashMap::new();
    for d in &docs {
        for t in d.iter().collect::<BTreeSet<_>>() {
            *df.entry(t.clone()).or_insert(0) += 1;
        }
    }
    let sims = similarity_matrix(&docs, options.idf.as_ref());
    let triggers: Vec<BTreeSet<String>> = profiles
        .iter()
        .map(|p| {
            skill::trigger_phrases(&p.description)
                .into_iter()
                .flat_map(|ph| tokens(&ph))
                .collect()
        })
        .collect();
    for i in 0..profiles.len() {
        for j in (i + 1)..profiles.len() {
            if profiles[i].kind != profiles[j].kind {
                continue;
            }
            let same_name = profiles[i].name.eq_ignore_ascii_case(&profiles[j].name);
            let same_hash = !profiles[i].content_hash.is_empty()
                && profiles[i].content_hash == profiles[j].content_hash;
            if same_name || same_hash {
                continue; // already reported above
            }
            let sim = sims[i][j];
            if sim >= options.overlap_threshold {
                let severity = if sim >= 0.85 {
                    ConflictSeverity::High
                } else {
                    ConflictSeverity::Medium
                };
                conflicts.push(Conflict {
                    kind: ConflictKind::DescriptionOverlap,
                    severity,
                    item_ids: [&profiles[i], &profiles[j]].iter().map(|p| p.id.clone()).filter(|id| !id.is_empty()).collect(),
                    names: vec![profiles[i].name.clone(), profiles[j].name.clone()],
                    detail: format!(
                        "Descriptions are {:.0}% similar; the agent will not reliably pick the right one",
                        sim * 100.0
                    ),
                    similarity: Some(sim),
                    shared_terms: shared_terms(&docs[i], &docs[j], &df, &surface),
                });
                continue;
            }
            let shared: Vec<String> = triggers[i]
                .intersection(&triggers[j])
                .map(|s| surface.get(s).cloned().unwrap_or_else(|| s.clone()))
                .collect();
            if shared.len() >= 2 && !triggers[i].is_empty() && !triggers[j].is_empty() {
                let overlap =
                    shared.len() as f64 / triggers[i].len().min(triggers[j].len()).max(1) as f64;
                if overlap >= 0.5 {
                    conflicts.push(Conflict {
                        kind: ConflictKind::TriggerOverlap,
                        severity: ConflictSeverity::Low,
                        item_ids: [&profiles[i], &profiles[j]].iter().map(|p| p.id.clone()).filter(|id| !id.is_empty()).collect(),
                        names: vec![profiles[i].name.clone(), profiles[j].name.clone()],
                        detail: "Both describe the same trigger situation; consider sharpening when each applies".to_string(),
                        similarity: Some(overlap),
                        shared_terms: shared.into_iter().take(6).collect(),
                    });
                }
            }
        }
    }

    // 3. Tool overlap: same pre-approved tools and overlapping descriptions below the threshold.
    for i in 0..profiles.len() {
        for j in (i + 1)..profiles.len() {
            if profiles[i].allowed_tools.is_empty() || profiles[j].allowed_tools.is_empty() {
                continue;
            }
            let a: BTreeSet<&String> = profiles[i].allowed_tools.iter().collect();
            let b: BTreeSet<&String> = profiles[j].allowed_tools.iter().collect();
            let shared: Vec<String> = a.intersection(&b).map(|s| (*s).clone()).collect();
            if !shared.is_empty()
                && sims[i][j] >= options.overlap_threshold * 0.6
                && sims[i][j] < options.overlap_threshold
            {
                conflicts.push(Conflict {
                    kind: ConflictKind::ToolOverlap,
                    severity: ConflictSeverity::Low,
                    item_ids: [&profiles[i], &profiles[j]]
                        .iter()
                        .map(|p| p.id.clone())
                        .filter(|id| !id.is_empty())
                        .collect(),
                    names: vec![profiles[i].name.clone(), profiles[j].name.clone()],
                    detail: format!("Both pre-approve {} for related tasks", shared.join(", ")),
                    similarity: Some(sims[i][j]),
                    shared_terms: shared,
                });
            }
        }
    }

    // 4. Contradictory directives.
    let dirs: Vec<Vec<(bool, String)>> =
        profiles.iter().map(|p| directives(&p.body_head)).collect();
    for i in 0..profiles.len() {
        for j in (i + 1)..profiles.len() {
            for (pa, sa) in &dirs[i] {
                for (pb, sb) in &dirs[j] {
                    if pa != pb && sa == sb {
                        conflicts.push(Conflict {
                            kind: ConflictKind::ContradictoryDirective,
                            severity: ConflictSeverity::Medium,
                            item_ids: [&profiles[i], &profiles[j]]
                                .iter()
                                .map(|p| p.id.clone())
                                .filter(|id| !id.is_empty())
                                .collect(),
                            names: vec![profiles[i].name.clone(), profiles[j].name.clone()],
                            detail: format!("One says always, the other never: \"{sa}\""),
                            similarity: None,
                            shared_terms: sa.split(' ').map(|s| s.to_string()).collect(),
                        });
                    }
                }
            }
        }
    }

    // 5. Budget.
    let b = budget(profiles, options.budget_chars);
    if b.over_budget {
        conflicts.push(Conflict {
            kind: ConflictKind::BudgetExceeded,
            severity: ConflictSeverity::High,
            item_ids: Vec::new(),
            names: Vec::new(),
            detail: format!(
                "Descriptions total {} characters against a budget of {}; skills past the limit are invisible to the agent",
                b.description_chars,
                b.budget_chars.unwrap_or(0)
            ),
            similarity: b.used_fraction,
            shared_terms: Vec::new(),
        });
    } else if b.used_fraction.map(|f| f > 0.8).unwrap_or(false) {
        conflicts.push(Conflict {
            kind: ConflictKind::BudgetExceeded,
            severity: ConflictSeverity::Low,
            item_ids: Vec::new(),
            names: Vec::new(),
            detail: format!(
                "Descriptions use {:.0}% of the budget",
                b.used_fraction.unwrap_or(0.0) * 100.0
            ),
            similarity: b.used_fraction,
            shared_terms: Vec::new(),
        });
    }

    conflicts.sort_by(|a, b| b.severity.cmp(&a.severity).then(a.names.cmp(&b.names)));
    let score = score(&conflicts, profiles.len());
    ConflictReport { conflicts, score }
}

fn score(conflicts: &[Conflict], n: usize) -> f64 {
    if conflicts.is_empty() {
        return 0.0;
    }
    let weight: f64 = conflicts
        .iter()
        .map(|c| match c.severity {
            ConflictSeverity::High => 1.0,
            ConflictSeverity::Medium => 0.5,
            ConflictSeverity::Low => 0.2,
        })
        .sum();
    (weight / (n.max(1) as f64)).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, name: &str, description: &str, body: &str) -> AssetProfile {
        AssetProfile {
            id: id.into(),
            kind: AssetKind::Skill,
            name: name.into(),
            description: description.into(),
            body_head: body.into(),
            content_hash: format!("h-{id}"),
            allowed_tools: vec![],
            description_tokens: crate::util::estimate_tokens(description),
            body_tokens: crate::util::estimate_tokens(body),
        }
    }

    #[test]
    fn clean_set_has_no_conflicts() {
        let set = vec![
            profile(
                "1",
                "pdf-extract",
                "Extract text and tables from PDF documents. Use when the user shares a PDF.",
                "Run extract.py",
            ),
            profile(
                "2",
                "git-commit",
                "Write conventional commit messages. Use when committing changes.",
                "Run git log",
            ),
            profile(
                "3",
                "sql-tuning",
                "Analyse slow SQL queries and propose indexes. Use when a query is slow.",
                "EXPLAIN ANALYZE",
            ),
        ];
        let report = analyze(&set, &AnalysisOptions::default());
        assert!(report.conflicts.is_empty(), "{report:?}");
        assert_eq!(report.score, 0.0);
    }

    #[test]
    fn detects_name_collision_duplicate_and_overlap() {
        let mut dup = profile(
            "4",
            "pdf-tools",
            "Extract text and tables from PDF documents. Use when the user shares a PDF.",
            "x",
        );
        dup.content_hash = "h-1".into();
        let set = vec![
            profile(
                "1",
                "pdf-extract",
                "Extract text and tables from PDF documents. Use when the user shares a PDF.",
                "x",
            ),
            profile(
                "2",
                "pdf-extract",
                "Something else entirely about commit messages",
                "x",
            ),
            profile(
                "3",
                "pdf-reader",
                "Extract text and tables from PDF documents when the user shares a PDF file.",
                "x",
            ),
            dup,
        ];
        let report = analyze(&set, &AnalysisOptions::default());
        let kinds: Vec<ConflictKind> = report.conflicts.iter().map(|c| c.kind).collect();
        assert!(kinds.contains(&ConflictKind::NameCollision), "{kinds:?}");
        assert!(kinds.contains(&ConflictKind::DuplicateContent), "{kinds:?}");
        assert!(
            kinds.contains(&ConflictKind::DescriptionOverlap),
            "{kinds:?}"
        );
        let overlap = report
            .conflicts
            .iter()
            .find(|c| c.kind == ConflictKind::DescriptionOverlap)
            .unwrap();
        assert!(overlap.similarity.unwrap() > 0.6);
        assert!(!overlap.shared_terms.is_empty());
        assert!(report.has_high());
        assert!(report.score > 0.0);
    }

    #[test]
    fn detects_budget_and_directives() {
        let long = "x".repeat(900);
        let set: Vec<AssetProfile> = (0..20)
            .map(|i| {
                profile(
                    &i.to_string(),
                    &format!("skill-{i}"),
                    &format!("{long} {i}"),
                    "",
                )
            })
            .collect();
        let report = analyze(
            &set,
            &AnalysisOptions {
                overlap_threshold: 0.99,
                budget_chars: Some(15_000),
                idf: None,
            },
        );
        assert!(report.conflicts.iter().any(
            |c| c.kind == ConflictKind::BudgetExceeded && c.severity == ConflictSeverity::High
        ));

        let set = vec![
            profile(
                "1",
                "a",
                "Formats python code with black when editing python files",
                "- Always run the formatter before committing",
            ),
            profile(
                "2",
                "b",
                "Reviews shell scripts for portability issues on linux",
                "- Never run the formatter before committing",
            ),
        ];
        let report = analyze(&set, &AnalysisOptions::default());
        assert!(
            report
                .conflicts
                .iter()
                .any(|c| c.kind == ConflictKind::ContradictoryDirective),
            "{report:?}"
        );
    }

    #[test]
    fn similarity_is_symmetric_and_bounded() {
        let docs = vec![
            tokens("extract pdf tables"),
            tokens("extract pdf text"),
            tokens("commit git messages"),
        ];
        let m = similarity_matrix(&docs, None);
        assert!((m[0][1] - m[1][0]).abs() < 1e-9);
        assert!(m[0][1] > m[0][2]);
        assert!(m.iter().flatten().all(|v| (0.0..=1.0001).contains(v)));
        // Background IDF makes a shared rare word count more than a shared common one.
        let idf = Idf::from_texts([
            "pdf forms",
            "pdf reports",
            "pdf tables",
            "pdf signatures",
            "pdf merge",
            "git commit",
            "git rebase",
            "extract zip",
            "docker build",
            "sql tuning",
        ]);
        let with_idf = similarity_matrix(&docs, Some(&idf));
        assert!(with_idf[0][1] > 0.0);
        assert!(idf.weight("extract") > idf.weight("pdf"));
    }

    #[test]
    fn tokenizer_stems_consistently() {
        assert_eq!(tokens("Extracting the tables"), vec!["extract", "tabl"]);
        assert_eq!(tokens("table"), vec!["tabl"]);
        assert_eq!(tokens("queries"), vec!["query"]);
        assert_eq!(stem("sharing"), stem("shared"));
        assert_eq!(stem("sharing"), stem("shares"));
        assert_eq!(stem("running"), "run");
        assert_eq!(stem("classes"), "class");
        assert_eq!(stem("status"), "status");
    }

    #[test]
    fn shared_terms_use_surface_forms() {
        let set = vec![
            profile(
                "1",
                "a",
                "Extracting tables from PDF documents when sharing a PDF",
                "x",
            ),
            profile(
                "2",
                "b",
                "Extract tables from PDF files when the user shares a PDF",
                "x",
            ),
        ];
        let report = analyze(&set, &AnalysisOptions::default());
        let overlap = report
            .conflicts
            .iter()
            .find(|c| c.kind == ConflictKind::DescriptionOverlap)
            .expect("overlap");
        assert!(
            overlap.shared_terms.iter().all(|t| [
                "extracting",
                "tables",
                "pdf",
                "sharing",
                "documents"
            ]
            .contains(&t.as_str())),
            "{:?}",
            overlap.shared_terms
        );
    }
}
