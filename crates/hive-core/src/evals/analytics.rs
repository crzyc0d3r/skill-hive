//! Pure functions that turn case results into a [`Benchmark`]: descriptive statistics, a
//! bootstrap confidence interval and Welch's t-test for the pass-rate difference, the verdict
//! rule, expectation patterns, analyst notes and the markdown report. Nothing here touches the
//! database or the file system, so every rule is unit-tested with hand-computed numbers.
//!
//! # Verdict rule
//!
//! For the treatment arm (`with_skill`, else the first non-baseline arm) against the baseline:
//!
//! * `Inconclusive` when either arm has fewer than 2 scored results;
//! * `HelpsALot` when the pass-rate delta is ≥ 0.25 and the 95 % CI lower bound is > 0;
//! * `Helps` when the delta is ≥ 0.10 and the CI lower bound is > −0.05;
//! * `HurtsALot` when the delta is ≤ −0.25 and the CI upper bound is < 0;
//! * `Hurts` when the delta is ≤ −0.10;
//! * `Neutral` otherwise.

use std::collections::BTreeMap;

use crate::model::{
    ArmActivity, ArmDelta, ArmKind, ArmSummary, Benchmark, CaseResult, CaseStatus, EvalRun,
    ExecutionTrace, ExpectationPattern, ExpectationRow, RunConfig, Stats, TriggerSummary, Verdict,
};

/// Bootstrap resamples for the confidence interval.
pub const BOOTSTRAP_RESAMPLES: usize = 2000;
/// Results per arm below which the notes ask for more repeats.
pub const SMALL_SAMPLE: u32 = 5;
/// Token ratio between arms that earns a note.
pub const TOKEN_RATIO_NOTE: f64 = 1.5;
/// Difference in IO share between arms that earns a note.
pub const IO_SHARE_NOTE: f64 = 0.15;

/// Mean, sample standard deviation (n − 1), min and max. All zero for an empty slice.
pub fn stats(values: &[f64]) -> Stats {
    let n = values.len();
    if n == 0 {
        return Stats::default();
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let variance = if n > 1 {
        values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n as f64 - 1.0)
    } else {
        0.0
    };
    Stats {
        n: n as u32,
        mean,
        stddev: variance.sqrt(),
        min: values.iter().copied().fold(f64::INFINITY, f64::min),
        max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    }
}

/// xorshift64* generator: tiny, deterministic, good enough for bootstrap resampling.
#[derive(Debug, Clone)]
pub struct XorShift64Star {
    state: u64,
}

impl XorShift64Star {
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform index in `0..n` (`n` must be > 0).
    pub fn below(&mut self, n: usize) -> usize {
        ((self.next_f64() * n as f64) as usize).min(n - 1)
    }
}

/// FNV-1a hash of a string: a stable seed per run id.
pub fn seed_from(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// 95 % percentile-bootstrap confidence interval of `mean(treatment) − mean(baseline)`.
/// Deterministic for a given seed. With an empty arm the interval collapses on the point
/// estimate.
pub fn bootstrap_ci_delta(
    treatment: &[f64],
    baseline: &[f64],
    resamples: usize,
    seed: u64,
) -> (f64, f64) {
    let point = mean(treatment) - mean(baseline);
    if treatment.is_empty() || baseline.is_empty() {
        return (point, point);
    }
    let mut rng = XorShift64Star::new(seed);
    let mut deltas = Vec::with_capacity(resamples.max(1));
    for _ in 0..resamples.max(1) {
        let t: f64 = (0..treatment.len())
            .map(|_| treatment[rng.below(treatment.len())])
            .sum::<f64>()
            / treatment.len() as f64;
        let b: f64 = (0..baseline.len())
            .map(|_| baseline[rng.below(baseline.len())])
            .sum::<f64>()
            / baseline.len() as f64;
        deltas.push(t - b);
    }
    deltas.sort_by(|a, b| a.total_cmp(b));
    let at = |q: f64| {
        let idx = ((deltas.len() - 1) as f64 * q).round() as usize;
        deltas[idx.min(deltas.len() - 1)]
    };
    (at(0.025), at(0.975))
}

/// Natural log of the gamma function (Lanczos approximation, g = 7).
pub fn ln_gamma(x: f64) -> f64 {
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = COEFFICIENTS[0];
    let t = x + 7.5;
    for (i, c) in COEFFICIENTS.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Continued fraction for the incomplete beta function (Lentz's method).
fn beta_continued_fraction(a: f64, b: f64, x: f64) -> f64 {
    const MAX_ITER: usize = 300;
    const EPS: f64 = 3e-14;
    const TINY: f64 = 1e-300;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=MAX_ITER {
        let m = m as f64;
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < EPS {
            break;
        }
    }
    h
}

/// Regularized incomplete beta function `I_x(a, b)`.
pub fn regularized_incomplete_beta(a: f64, b: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let front =
        (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_continued_fraction(a, b, x) / a
    } else {
        1.0 - front * beta_continued_fraction(b, a, 1.0 - x) / b
    }
}

/// Two-sided p-value of a Student t statistic with `df` degrees of freedom.
pub fn student_t_two_sided_p(t: f64, df: f64) -> f64 {
    if !t.is_finite() || df <= 0.0 {
        return f64::NAN;
    }
    let x = df / (df + t * t);
    regularized_incomplete_beta(df / 2.0, 0.5, x).clamp(0.0, 1.0)
}

/// Welch's unequal-variance t-test, two-sided. `None` when either sample has fewer than two
/// values or both samples have zero variance (the statistic is undefined).
pub fn welch_p_value(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() < 2 || b.len() < 2 {
        return None;
    }
    let (sa, sb) = (stats(a), stats(b));
    let (na, nb) = (a.len() as f64, b.len() as f64);
    let (va, vb) = (sa.stddev.powi(2) / na, sb.stddev.powi(2) / nb);
    let se2 = va + vb;
    if se2 <= 0.0 {
        return None;
    }
    let t = (sa.mean - sb.mean) / se2.sqrt();
    let df = se2.powi(2) / (va.powi(2) / (na - 1.0) + vb.powi(2) / (nb - 1.0));
    if !df.is_finite() || df <= 0.0 {
        return None;
    }
    let p = student_t_two_sided_p(t, df);
    p.is_finite().then_some(p)
}

/// The verdict rule (see the module docs).
pub fn verdict(
    delta: f64,
    ci_low: f64,
    ci_high: f64,
    n_treatment: usize,
    n_baseline: usize,
) -> Verdict {
    if n_treatment < 2 || n_baseline < 2 {
        Verdict::Inconclusive
    } else if delta >= 0.25 && ci_low > 0.0 {
        Verdict::HelpsALot
    } else if delta >= 0.10 && ci_low > -0.05 {
        Verdict::Helps
    } else if delta <= -0.25 && ci_high < 0.0 {
        Verdict::HurtsALot
    } else if delta <= -0.10 {
        Verdict::Hurts
    } else {
        Verdict::Neutral
    }
}

/// The baseline arm: `config.baseline_label` when it names an arm, else the first arm that is
/// not `WithSkill`, else the second arm, else the only arm.
pub fn baseline_label(config: &RunConfig) -> String {
    if let Some(label) = &config.baseline_label {
        if config.arms.iter().any(|a| &a.label == label) {
            return label.clone();
        }
    }
    if let Some(arm) = config
        .arms
        .iter()
        .find(|a| !matches!(a.kind, ArmKind::WithSkill))
    {
        return arm.label.clone();
    }
    config
        .arms
        .get(1)
        .or_else(|| config.arms.first())
        .map(|a| a.label.clone())
        .unwrap_or_else(|| crate::evals::WITHOUT_SKILL_LABEL.to_string())
}

/// The treatment arm: `with_skill` when present and not the baseline, else the first other arm.
pub fn treatment_label(config: &RunConfig, baseline: &str) -> String {
    let with = crate::evals::WITH_SKILL_LABEL;
    if config.arms.iter().any(|a| a.label == with) && with != baseline {
        return with.to_string();
    }
    config
        .arms
        .iter()
        .find(|a| a.label != baseline)
        .map(|a| a.label.clone())
        .unwrap_or_else(|| baseline.to_string())
}

/// The pass-rate value a result contributes: its pass rate when it was scored, 0 when the
/// execution failed or timed out, nothing when it was cancelled or scored nothing.
pub fn pass_rate_sample(result: &CaseResult) -> Option<f64> {
    match result.status {
        CaseStatus::Executed | CaseStatus::Graded if result.total > 0 => Some(result.pass_rate),
        CaseStatus::Failed | CaseStatus::Timeout => Some(0.0),
        _ => None,
    }
}

fn scored(result: &CaseResult) -> bool {
    matches!(result.status, CaseStatus::Executed | CaseStatus::Graded)
}

/// Pass-rate samples of one arm.
pub fn pass_rate_samples(results: &[CaseResult], arm: &str) -> Vec<f64> {
    results
        .iter()
        .filter(|r| r.arm == arm)
        .filter_map(pass_rate_sample)
        .collect()
}

/// Activity statistics over the traced, scored results of an arm; `None` without traces.
pub fn summarize_activity(done: &[&CaseResult]) -> Option<ArmActivity> {
    let traces: Vec<&ExecutionTrace> = done.iter().filter_map(|r| r.trace.as_ref()).collect();
    if traces.is_empty() {
        return None;
    }
    let of =
        |f: fn(&ExecutionTrace) -> f64| stats(&traces.iter().map(|t| f(t)).collect::<Vec<_>>());
    let opened = traces
        .iter()
        .filter(|t| t.skill_reads > 0 || t.skill_in_context)
        .count();
    Some(ArmActivity {
        traced: traces.len() as u32,
        turns: of(|t| t.turns as f64),
        tool_calls: of(|t| t.tool_calls as f64),
        reads: of(|t| t.reads as f64),
        searches: of(|t| t.searches as f64),
        writes: of(|t| t.writes as f64),
        commands: of(|t| t.commands as f64),
        web: of(|t| t.web as f64),
        tool_errors: of(|t| t.tool_errors as f64),
        files_read: of(|t| t.files_read_count as f64),
        files_written: of(|t| t.files_written_count as f64),
        thinking_tokens: of(|t| t.thinking_tokens as f64),
        text_tokens: of(|t| t.text_tokens as f64),
        tool_input_tokens: of(|t| t.tool_input_tokens as f64),
        tool_result_tokens: of(|t| t.tool_result_tokens as f64),
        io_share: of(|t| t.io_share),
        cache_read_tokens: of(|t| t.cache_read_tokens as f64),
        fresh_input_tokens: of(|t| (t.cache_creation_tokens + t.uncached_input_tokens) as f64),
        model_time_ms: stats(
            &traces
                .iter()
                .filter_map(|t| t.model_time_ms)
                .map(|v| v as f64)
                .collect::<Vec<_>>(),
        ),
        tool_time_ms: stats(
            &traces
                .iter()
                .filter_map(|t| t.tool_time_ms)
                .map(|v| v as f64)
                .collect::<Vec<_>>(),
        ),
        skill_opened: opened as f64 / traces.len() as f64,
    })
}

/// Summary statistics of one arm.
pub fn summarize_arm(label: &str, results: &[CaseResult]) -> ArmSummary {
    let in_arm: Vec<&CaseResult> = results
        .iter()
        .filter(|r| r.arm == label && !matches!(r.status, CaseStatus::Queued | CaseStatus::Running))
        .collect();
    let done: Vec<&CaseResult> = in_arm.iter().copied().filter(|r| scored(r)).collect();
    let costs: Vec<f64> = done.iter().filter_map(|r| r.cost_usd).collect();
    ArmSummary {
        activity: summarize_activity(&done),
        label: label.to_string(),
        runs: in_arm.len() as u32,
        errors: in_arm
            .iter()
            .filter(|r| matches!(r.status, CaseStatus::Failed | CaseStatus::Timeout))
            .count() as u32,
        pass_rate: stats(&pass_rate_samples(results, label)),
        tokens: stats(
            &done
                .iter()
                .map(|r| (r.input_tokens + r.output_tokens) as f64)
                .collect::<Vec<_>>(),
        ),
        duration_ms: stats(
            &done
                .iter()
                .map(|r| r.duration_ms as f64)
                .collect::<Vec<_>>(),
        ),
        tool_calls: stats(&done.iter().map(|r| r.tool_calls as f64).collect::<Vec<_>>()),
        cost_usd: (!costs.is_empty()).then(|| costs.iter().sum()),
    }
}

/// Classify an expectation from its pass fraction per arm.
pub fn classify(
    by_arm: &BTreeMap<String, f64>,
    treatment: &str,
    baseline: &str,
) -> ExpectationPattern {
    const EPS: f64 = 1e-9;
    if by_arm.is_empty() {
        return ExpectationPattern::Mixed;
    }
    let passes = |v: &f64| *v >= 1.0 - EPS;
    let fails = |v: &f64| *v <= EPS;
    if by_arm.values().all(passes) {
        return ExpectationPattern::AlwaysPasses;
    }
    if by_arm.values().all(fails) {
        return ExpectationPattern::AlwaysFails;
    }
    if by_arm.values().any(|v| !passes(v) && !fails(v)) {
        return ExpectationPattern::Flaky;
    }
    let only = |label: &str| {
        by_arm.get(label).is_some_and(passes)
            && by_arm
                .iter()
                .filter(|(k, _)| k.as_str() != label)
                .all(|(_, v)| fails(v))
    };
    if only(treatment) {
        ExpectationPattern::OnlyWithSkill
    } else if only(baseline) {
        ExpectationPattern::OnlyBaseline
    } else {
        ExpectationPattern::Mixed
    }
}

/// One row per (case, expectation) with the pass fraction per arm and its pattern.
pub fn expectation_rows(
    results: &[CaseResult],
    treatment: &str,
    baseline: &str,
) -> Vec<ExpectationRow> {
    type Key = (u32, String);
    let mut table: BTreeMap<Key, (String, BTreeMap<String, Vec<bool>>)> = BTreeMap::new();
    for r in results {
        if matches!(
            r.status,
            CaseStatus::Cancelled | CaseStatus::Queued | CaseStatus::Running
        ) {
            continue;
        }
        for e in &r.expectations {
            let entry = table
                .entry((r.case_id, e.text.clone()))
                .or_insert_with(|| (r.case_name.clone(), BTreeMap::new()));
            entry.1.entry(r.arm.clone()).or_default().push(e.passed);
        }
    }
    table
        .into_iter()
        .map(|((case_id, text), (case_name, arms))| {
            let by_arm: BTreeMap<String, f64> = arms
                .into_iter()
                .map(|(arm, passes)| {
                    let n = passes.len() as f64;
                    let p = passes.iter().filter(|p| **p).count() as f64;
                    (arm, if n > 0.0 { p / n } else { 0.0 })
                })
                .collect();
            let pattern = classify(&by_arm, treatment, baseline);
            ExpectationRow {
                case_id,
                case_name,
                text,
                by_arm,
                pattern,
            }
        })
        .collect()
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

/// Notes about how the arms worked, from their activity profiles: whether skill arms opened
/// the skill, and large differences in reads, thinking, tool errors and IO share.
pub fn activity_notes(arms: &[ArmSummary], config: &RunConfig, baseline: &str) -> Vec<String> {
    let mut notes = Vec::new();
    let base = arms
        .iter()
        .find(|a| a.label == baseline)
        .and_then(|a| a.activity.as_ref());
    for arm in arms {
        let Some(act) = &arm.activity else {
            continue;
        };
        let has_skill = config
            .arms
            .iter()
            .find(|a| a.label == arm.label)
            .is_some_and(|a| !matches!(a.kind, ArmKind::WithoutSkill));
        if has_skill {
            let opened = (act.skill_opened * act.traced as f64).round() as u32;
            if opened < act.traced {
                notes.push(format!(
                    "{} opened the skill in only {opened} of {} run(s): the other run(s) never read it, so its instructions could not help there",
                    arm.label, act.traced
                ));
            } else {
                notes.push(format!(
                    "{} opened the skill in every run ({} of {})",
                    arm.label, opened, act.traced
                ));
            }
        }
        if act.tool_errors.mean >= 1.0 {
            notes.push(format!(
                "{} hit {:.1} tool error(s) per run on average",
                arm.label, act.tool_errors.mean
            ));
        }
        if arm.label == baseline {
            continue;
        }
        let Some(base) = base else {
            continue;
        };
        if base.reads.mean > 0.0 && act.reads.mean / base.reads.mean >= TOKEN_RATIO_NOTE {
            notes.push(format!(
                "{} reads {:.1}× as many files as {baseline} ({:.1} vs {:.1} per run)",
                arm.label,
                act.reads.mean / base.reads.mean,
                act.reads.mean,
                base.reads.mean
            ));
        }
        if base.thinking_tokens.mean > 0.0
            && act.thinking_tokens.mean / base.thinking_tokens.mean >= TOKEN_RATIO_NOTE
        {
            notes.push(format!(
                "{} thinks {:.1}× as much as {baseline} ({} vs {} tokens per run)",
                arm.label,
                act.thinking_tokens.mean / base.thinking_tokens.mean,
                num(act.thinking_tokens.mean),
                num(base.thinking_tokens.mean)
            ));
        } else if act.thinking_tokens.mean > 0.0
            && base.thinking_tokens.mean / act.thinking_tokens.mean >= TOKEN_RATIO_NOTE
        {
            notes.push(format!(
                "{} thinks {:.1}× as much as {} ({} vs {} tokens per run)",
                baseline,
                base.thinking_tokens.mean / act.thinking_tokens.mean,
                arm.label,
                num(base.thinking_tokens.mean),
                num(act.thinking_tokens.mean)
            ));
        }
        let io_diff = act.io_share.mean - base.io_share.mean;
        if io_diff.abs() >= IO_SHARE_NOTE {
            notes.push(format!(
                "{} spends {} of its traffic on tool IO vs {} for {baseline}: {}",
                arm.label,
                pct(act.io_share.mean),
                pct(base.io_share.mean),
                if io_diff > 0.0 {
                    "more of its tokens move data through tools, fewer reason or explain"
                } else {
                    "more of its tokens reason or explain, fewer move data through tools"
                }
            ));
        }
    }
    notes
}

/// Analyst notes: what the aggregates hide.
pub fn analyst_notes(
    arms: &[ArmSummary],
    rows: &[ExpectationRow],
    results: &[CaseResult],
    baseline: &str,
) -> Vec<String> {
    let mut notes = Vec::new();
    let total = rows.len();
    if total > 0 {
        let count = |p: ExpectationPattern| rows.iter().filter(|r| r.pattern == p).count();
        let always = count(ExpectationPattern::AlwaysPasses);
        if always > 0 {
            notes.push(format!(
                "{always} of {total} expectations pass in every arm and do not measure the skill"
            ));
        }
        let never = count(ExpectationPattern::AlwaysFails);
        if never > 0 {
            notes.push(format!(
                "{never} of {total} expectations fail in every arm: broken expectations, or beyond what the model can do"
            ));
        }
        let flaky = count(ExpectationPattern::Flaky);
        if flaky > 0 {
            notes.push(format!(
                "{flaky} of {total} expectations are flaky: the same case passes and fails across repeats"
            ));
        }
        let only_baseline = count(ExpectationPattern::OnlyBaseline);
        if only_baseline > 0 {
            notes.push(format!(
                "{only_baseline} of {total} expectations pass only in the baseline: the skill may be getting in the way"
            ));
        }
    }
    let base = arms.iter().find(|a| a.label == baseline);
    for arm in arms {
        if arm.label != baseline {
            if let Some(base) = base {
                if base.tokens.mean > 0.0 && arm.tokens.n > 0 {
                    let ratio = arm.tokens.mean / base.tokens.mean;
                    if ratio >= TOKEN_RATIO_NOTE {
                        notes.push(format!(
                            "{} uses {ratio:.1}× the tokens of {baseline}",
                            arm.label
                        ));
                    } else if ratio <= 1.0 / TOKEN_RATIO_NOTE {
                        notes.push(format!(
                            "{} uses {ratio:.1}× the tokens of {baseline} (fewer)",
                            arm.label
                        ));
                    }
                }
                if let (Some(a), Some(b)) = (arm.cost_usd, base.cost_usd) {
                    if b > 0.0 && a / b >= TOKEN_RATIO_NOTE {
                        notes.push(format!(
                            "{} costs {:.1}× the baseline (${a:.3} vs ${b:.3})",
                            arm.label,
                            a / b
                        ));
                    }
                }
            }
        }
        if arm.errors > 0 {
            notes.push(format!(
                "{} of {} executions failed or timed out in {}",
                arm.errors, arm.runs, arm.label
            ));
        }
    }
    let mut durations: Vec<f64> = results
        .iter()
        .filter(|r| scored(r))
        .map(|r| r.duration_ms as f64)
        .collect();
    let med = median(&mut durations);
    if med > 0.0 {
        for r in results.iter().filter(|r| scored(r)) {
            let d = r.duration_ms as f64;
            if d > 3.0 * med && d > 5_000.0 {
                notes.push(format!(
                    "case `{}` ({}, repeat {}) took {:.1} s, {:.1}× the median",
                    r.case_name,
                    r.arm,
                    r.repeat,
                    d / 1000.0,
                    d / med
                ));
            }
        }
    }
    if let Some(min_n) = arms.iter().map(|a| a.pass_rate.n).min() {
        if min_n > 0 && min_n < SMALL_SAMPLE {
            notes.push(format!(
                "only {min_n} scored result(s) in the smallest arm: repeat the run to tighten the confidence interval"
            ));
        }
    }
    if results.iter().any(|r| {
        r.grader_notes
            .as_deref()
            .is_some_and(|n| n.starts_with(crate::evals::grader::NO_GRADER_NOTE))
    }) {
        notes.push(
            "no grader configured: only deterministic checks were scored; expectations without checks were skipped"
                .into(),
        );
    }
    notes
}

/// Build the benchmark for a run from its case results.
pub fn aggregate(
    run: &EvalRun,
    results: &[CaseResult],
    trigger: Option<TriggerSummary>,
    executor: &str,
    grader: Option<&str>,
) -> Benchmark {
    let config = &run.config;
    let baseline = baseline_label(config);
    let treatment = treatment_label(config, &baseline);
    let arms: Vec<ArmSummary> = config
        .arms
        .iter()
        .map(|a| summarize_arm(&a.label, results))
        .collect();
    let base_summary = arms.iter().find(|a| a.label == baseline).cloned();
    let base_samples = pass_rate_samples(results, &baseline);
    let seed = seed_from(&run.id);
    let deltas: Vec<ArmDelta> = arms
        .iter()
        .filter(|a| a.label != baseline)
        .enumerate()
        .map(|(i, arm)| {
            let samples = pass_rate_samples(results, &arm.label);
            let delta = mean(&samples) - mean(&base_samples);
            let (ci_low, ci_high) = bootstrap_ci_delta(
                &samples,
                &base_samples,
                BOOTSTRAP_RESAMPLES,
                seed.wrapping_add(i as u64),
            );
            let (tokens_delta, duration_delta_ms) = match &base_summary {
                Some(b) => (
                    arm.tokens.mean - b.tokens.mean,
                    arm.duration_ms.mean - b.duration_ms.mean,
                ),
                None => (0.0, 0.0),
            };
            ArmDelta {
                arm: arm.label.clone(),
                baseline: baseline.clone(),
                pass_rate_delta: delta,
                ci_low,
                ci_high,
                p_value: welch_p_value(&samples, &base_samples),
                tokens_delta,
                duration_delta_ms,
                verdict: verdict(delta, ci_low, ci_high, samples.len(), base_samples.len()),
            }
        })
        .collect();
    let expectations = expectation_rows(results, &treatment, &baseline);
    let verdict = deltas
        .iter()
        .find(|d| d.arm == treatment)
        .or(deltas.first())
        .map(|d| d.verdict)
        .unwrap_or(Verdict::Inconclusive);
    let mut notes = analyst_notes(&arms, &expectations, results, &baseline);
    notes.extend(activity_notes(&arms, config, &baseline));
    Benchmark {
        skill_name: run.skill_name.clone(),
        skill_hash: run.skill_hash.clone(),
        executor: executor.to_string(),
        grader: grader.map(str::to_string),
        arms,
        baseline,
        deltas,
        expectations,
        trigger,
        verdict,
        notes,
        generated_at: crate::util::now_ms(),
    }
}

fn pct(x: f64) -> String {
    format!("{:.0}%", x * 100.0)
}

fn pp(x: f64) -> String {
    format!("{:+.0} pp", x * 100.0)
}

fn num(x: f64) -> String {
    let whole = x.round().abs() as u64;
    let digits = whole.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    if x < -0.5 { format!("-{out}") } else { out }
}

fn secs(ms: f64) -> String {
    format!("{:.1} s", ms / 1000.0)
}

fn money(x: Option<f64>) -> String {
    x.map(|v| format!("${v:.3}")).unwrap_or_else(|| "—".into())
}

fn pattern_label(p: ExpectationPattern) -> &'static str {
    match p {
        ExpectationPattern::AlwaysPasses => "always passes",
        ExpectationPattern::AlwaysFails => "always fails",
        ExpectationPattern::OnlyWithSkill => "only with skill",
        ExpectationPattern::OnlyBaseline => "only baseline",
        ExpectationPattern::Flaky => "flaky",
        ExpectationPattern::Mixed => "mixed",
    }
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

/// Render a benchmark as a markdown report.
pub fn benchmark_markdown(b: &Benchmark) -> String {
    let mut md = String::new();
    md.push_str(&format!("# Benchmark: {}\n\n", b.skill_name));
    let headline = b
        .deltas
        .iter()
        .find(|d| d.verdict == b.verdict)
        .map(|d| {
            format!(
                " — {} vs {}: {} pass rate (95% CI {} … {}{})",
                d.arm,
                d.baseline,
                pp(d.pass_rate_delta),
                pp(d.ci_low),
                pp(d.ci_high),
                d.p_value
                    .map(|p| format!(", p = {p:.3}"))
                    .unwrap_or_default()
            )
        })
        .unwrap_or_default();
    md.push_str(&format!("**Verdict: {}**{headline}\n\n", b.verdict.label()));
    md.push_str(&format!(
        "Executor: {} · Grader: {} · Skill hash: `{}` · Generated: {}\n\n",
        b.executor,
        b.grader.as_deref().unwrap_or("none (checks only)"),
        crate::hash::short(&b.skill_hash),
        crate::util::iso_from_ms(b.generated_at)
    ));

    md.push_str("## Arms\n\n");
    md.push_str(
        "| Arm | Runs | Errors | Pass rate | ± sd | Tokens | Duration | Tool calls | Cost |\n",
    );
    md.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for a in &b.arms {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {:.1} | {} |\n",
            cell(&a.label),
            a.runs,
            a.errors,
            pct(a.pass_rate.mean),
            pct(a.pass_rate.stddev),
            num(a.tokens.mean),
            secs(a.duration_ms.mean),
            a.tool_calls.mean,
            money(a.cost_usd)
        ));
    }
    md.push('\n');

    if !b.deltas.is_empty() {
        md.push_str(&format!("## Deltas vs {}\n\n", b.baseline));
        md.push_str("| Arm | Δ pass rate | 95% CI | p-value | Δ tokens | Δ duration | Verdict |\n");
        md.push_str("|---|---:|---|---:|---:|---:|---|\n");
        for d in &b.deltas {
            md.push_str(&format!(
                "| {} | {} | {} … {} | {} | {} | {} | {} |\n",
                cell(&d.arm),
                pp(d.pass_rate_delta),
                pp(d.ci_low),
                pp(d.ci_high),
                d.p_value
                    .map(|p| format!("{p:.3}"))
                    .unwrap_or_else(|| "—".into()),
                num(d.tokens_delta),
                secs(d.duration_delta_ms),
                d.verdict.label()
            ));
        }
        md.push('\n');
    }

    if b.arms.iter().any(|a| a.activity.is_some()) {
        md.push_str("## Activity\n\n");
        md.push_str(
            "Mean per run over the traced executions. Token splits are estimated from characters; cache figures and turns are as reported.\n\n",
        );
        let labels: Vec<&str> = b.arms.iter().map(|a| a.label.as_str()).collect();
        md.push_str(&format!("| Metric | {} |\n", labels.join(" | ")));
        md.push_str(&format!("|---|{}\n", "---:|".repeat(labels.len())));
        type Row<'a> = (&'a str, Box<dyn Fn(&ArmActivity) -> String + 'a>);
        let rows: Vec<Row> = vec![
            ("Traced runs", Box::new(|a| a.traced.to_string())),
            ("Model turns", Box::new(|a| format!("{:.1}", a.turns.mean))),
            (
                "Tool calls",
                Box::new(|a| format!("{:.1}", a.tool_calls.mean)),
            ),
            ("File reads", Box::new(|a| format!("{:.1}", a.reads.mean))),
            ("Searches", Box::new(|a| format!("{:.1}", a.searches.mean))),
            ("Writes", Box::new(|a| format!("{:.1}", a.writes.mean))),
            ("Commands", Box::new(|a| format!("{:.1}", a.commands.mean))),
            ("Web", Box::new(|a| format!("{:.1}", a.web.mean))),
            (
                "Tool errors",
                Box::new(|a| format!("{:.1}", a.tool_errors.mean)),
            ),
            (
                "Distinct files read",
                Box::new(|a| format!("{:.1}", a.files_read.mean)),
            ),
            (
                "Distinct files written",
                Box::new(|a| format!("{:.1}", a.files_written.mean)),
            ),
            ("Opened the skill", Box::new(|a| pct(a.skill_opened))),
            (
                "Thinking tokens (est.)",
                Box::new(|a| num(a.thinking_tokens.mean)),
            ),
            ("Text tokens (est.)", Box::new(|a| num(a.text_tokens.mean))),
            (
                "Tool-argument tokens (est.)",
                Box::new(|a| num(a.tool_input_tokens.mean)),
            ),
            (
                "Tool-result tokens (est.)",
                Box::new(|a| num(a.tool_result_tokens.mean)),
            ),
            ("IO share", Box::new(|a| pct(a.io_share.mean))),
            (
                "Cache-read tokens",
                Box::new(|a| num(a.cache_read_tokens.mean)),
            ),
            (
                "Fresh input tokens",
                Box::new(|a| num(a.fresh_input_tokens.mean)),
            ),
            (
                "Model time",
                Box::new(|a| {
                    if a.model_time_ms.n > 0 {
                        secs(a.model_time_ms.mean)
                    } else {
                        "—".into()
                    }
                }),
            ),
            (
                "Tool time",
                Box::new(|a| {
                    if a.tool_time_ms.n > 0 {
                        secs(a.tool_time_ms.mean)
                    } else {
                        "—".into()
                    }
                }),
            ),
        ];
        for (name, f) in rows {
            let cells: Vec<String> = b
                .arms
                .iter()
                .map(|a| a.activity.as_ref().map(&f).unwrap_or_else(|| "—".into()))
                .collect();
            md.push_str(&format!("| {name} | {} |\n", cells.join(" | ")));
        }
        md.push('\n');
    }

    if !b.expectations.is_empty() {
        md.push_str("## Expectations\n\n");
        let arm_labels: Vec<&str> = b.arms.iter().map(|a| a.label.as_str()).collect();
        md.push_str(&format!(
            "| Case | Expectation | {} | Pattern |\n",
            arm_labels.join(" | ")
        ));
        md.push_str(&format!(
            "|---|---|{}---|\n",
            "---:|".repeat(arm_labels.len())
        ));
        for row in &b.expectations {
            let cells: Vec<String> = arm_labels
                .iter()
                .map(|l| {
                    row.by_arm
                        .get(*l)
                        .map(|v| pct(*v))
                        .unwrap_or_else(|| "—".into())
                })
                .collect();
            md.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                cell(&row.case_name),
                cell(&row.text),
                cells.join(" | "),
                pattern_label(row.pattern)
            ));
        }
        md.push('\n');
    }

    if let Some(t) = &b.trigger {
        md.push_str("## Trigger\n\n");
        md.push_str(&format!(
            "{} of {} queries routed correctly · precision {} · recall {} · F1 {:.2}\n\n",
            t.correct,
            t.total,
            pct(t.precision),
            pct(t.recall),
            t.f1
        ));
        md.push_str(
            "| Query | Should trigger | Triggered | Chosen | OK |\n|---|---|---|---|---|\n",
        );
        for r in &t.results {
            md.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                cell(&r.query),
                if r.should_trigger { "yes" } else { "no" },
                if r.triggered { "yes" } else { "no" },
                cell(r.chosen.as_deref().unwrap_or("—")),
                if r.correct { "✓" } else { "✗" }
            ));
        }
        md.push('\n');
    }

    if !b.notes.is_empty() {
        md.push_str("## Notes\n\n");
        for n in &b.notes {
            md.push_str(&format!("- {n}\n"));
        }
        md.push('\n');
    }
    md
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ArmSpec, ExpectationResult, ExpectationSource, RunProgress, RunStatus};

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn stats_on_known_vectors() {
        let s = stats(&[2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        assert_eq!(s.n, 8);
        assert!(approx(s.mean, 5.0, 1e-12));
        // sample stddev: sum of squares 32 / 7 → sqrt(4.5714) = 2.138
        assert!(approx(s.stddev, 2.138_089_935, 1e-6));
        assert_eq!((s.min, s.max), (2.0, 9.0));
        let one = stats(&[3.5]);
        assert_eq!((one.n, one.mean, one.stddev), (1, 3.5, 0.0));
        assert_eq!(stats(&[]), Stats::default());
    }

    #[test]
    fn prng_is_deterministic_and_in_range() {
        let mut a = XorShift64Star::new(42);
        let mut b = XorShift64Star::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
            let f = a.next_f64();
            assert!((0.0..1.0).contains(&f));
            assert!(a.below(7) < 7);
            b.next_f64();
            b.below(7);
        }
        assert_ne!(
            XorShift64Star::new(1).next_u64(),
            XorShift64Star::new(2).next_u64()
        );
        assert_ne!(seed_from("run-a"), seed_from("run-b"));
    }

    #[test]
    fn bootstrap_ci_contains_point_and_is_deterministic() {
        let t = [1.0, 1.0, 0.5, 1.0, 0.75, 1.0];
        let b = [0.25, 0.5, 0.0, 0.5, 0.25, 0.0];
        let point = mean(&t) - mean(&b);
        let (lo, hi) = bootstrap_ci_delta(&t, &b, 2000, 7);
        assert!(lo <= point && point <= hi, "{lo} {point} {hi}");
        assert!(
            lo > 0.0,
            "clear separation gives a positive lower bound: {lo}"
        );
        assert_eq!((lo, hi), bootstrap_ci_delta(&t, &b, 2000, 7));
        // A different seed draws different resamples (the percentiles may or may not move).
        let mut a = XorShift64Star::new(7);
        let mut b2 = XorShift64Star::new(8);
        assert_ne!(a.next_u64(), b2.next_u64());
        // constant arms collapse on the point estimate
        assert_eq!(
            bootstrap_ci_delta(&[1.0, 1.0], &[0.0, 0.0], 500, 1),
            (1.0, 1.0)
        );
        assert_eq!(bootstrap_ci_delta(&[0.5], &[], 500, 1), (0.5, 0.5));
    }

    #[test]
    fn incomplete_beta_and_t_distribution() {
        // I_x(1,1) = x; I_x(2,1) = x²
        assert!(approx(
            regularized_incomplete_beta(1.0, 1.0, 0.3),
            0.3,
            1e-10
        ));
        assert!(approx(
            regularized_incomplete_beta(2.0, 1.0, 0.5),
            0.25,
            1e-10
        ));
        assert!(approx(ln_gamma(5.0), (24.0f64).ln(), 1e-10));
        assert!(approx(
            ln_gamma(0.5),
            std::f64::consts::PI.sqrt().ln(),
            1e-10
        ));
        // t = 1, df = 8 → two-sided p = 0.3466
        assert!(approx(student_t_two_sided_p(1.0, 8.0), 0.3466, 5e-4));
        // t = 2.0, df = 10 → 0.0734; large df approaches the normal: t = 1.96 → 0.05
        assert!(approx(student_t_two_sided_p(2.0, 10.0), 0.0734, 5e-4));
        assert!(approx(student_t_two_sided_p(1.96, 1e6), 0.05, 1e-3));
        assert_eq!(student_t_two_sided_p(0.0, 5.0), 1.0);
    }

    #[test]
    fn welch_matches_textbook_values() {
        // Equal variances, shifted by one: t = −1, df = 8, p = 0.3466
        let p = welch_p_value(&[1.0, 2.0, 3.0, 4.0, 5.0], &[2.0, 3.0, 4.0, 5.0, 6.0]).unwrap();
        assert!(approx(p, 0.3466, 1e-3), "{p}");
        // Wikipedia "Welch's t-test", example 1: t = −2.46, ν ≈ 24.98, p = 0.021
        let a1 = [
            27.5, 21.0, 19.0, 23.6, 17.0, 17.9, 16.9, 20.1, 21.9, 22.6, 23.1, 19.6, 19.0, 21.7,
            21.4,
        ];
        let a2 = [
            27.1, 22.0, 20.8, 23.4, 23.4, 23.5, 25.8, 22.0, 24.8, 20.2, 21.9, 22.1, 22.9, 20.5,
            24.4,
        ];
        let p = welch_p_value(&a1, &a2).unwrap();
        assert!(approx(p, 0.0214, 1e-3), "{p}");
        // Example 2 (unequal sizes and variances): p ≈ 0.149
        let b1 = [17.2, 20.9, 22.6, 18.1, 21.7, 21.4, 23.5, 24.2, 14.7, 21.8];
        let b2 = [
            21.5, 22.8, 21.0, 23.0, 21.6, 23.6, 22.5, 20.7, 23.4, 21.8, 20.7, 21.7, 21.5, 22.5,
            23.6, 21.5, 22.5, 23.5, 21.5, 21.8,
        ];
        let p = welch_p_value(&b1, &b2).unwrap();
        assert!(approx(p, 0.1488, 1e-3), "{p}");
        // Undefined cases
        assert!(welch_p_value(&[1.0], &[1.0, 2.0]).is_none());
        assert!(welch_p_value(&[1.0, 2.0], &[]).is_none());
        assert!(welch_p_value(&[1.0, 1.0], &[0.0, 0.0]).is_none());
        // One arm with variance is fine
        assert!(welch_p_value(&[1.0, 1.0, 1.0], &[0.0, 0.5, 1.0]).is_some());
    }

    #[test]
    fn verdict_thresholds() {
        assert_eq!(verdict(0.5, 0.2, 0.8, 1, 5), Verdict::Inconclusive);
        assert_eq!(verdict(0.5, 0.2, 0.8, 5, 1), Verdict::Inconclusive);
        assert_eq!(verdict(0.25, 0.01, 0.5, 2, 2), Verdict::HelpsALot);
        assert_eq!(
            verdict(0.25, 0.0, 0.5, 2, 2),
            Verdict::Helps,
            "ci_low must be > 0 for a lot"
        );
        assert_eq!(verdict(0.10, -0.04, 0.3, 2, 2), Verdict::Helps);
        assert_eq!(verdict(0.10, -0.05, 0.3, 2, 2), Verdict::Neutral);
        assert_eq!(verdict(0.09, 0.05, 0.3, 2, 2), Verdict::Neutral);
        assert_eq!(verdict(-0.25, -0.5, -0.01, 2, 2), Verdict::HurtsALot);
        assert_eq!(verdict(-0.25, -0.5, 0.0, 2, 2), Verdict::Hurts);
        assert_eq!(verdict(-0.10, -0.5, 0.2, 2, 2), Verdict::Hurts);
        assert_eq!(verdict(-0.09, -0.5, 0.2, 2, 2), Verdict::Neutral);
        assert_eq!(verdict(0.0, -0.1, 0.1, 10, 10), Verdict::Neutral);
    }

    fn by_arm(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn expectation_patterns() {
        let (t, b) = ("with_skill", "without_skill");
        assert_eq!(
            classify(&by_arm(&[(t, 1.0), (b, 1.0)]), t, b),
            ExpectationPattern::AlwaysPasses
        );
        assert_eq!(
            classify(&by_arm(&[(t, 0.0), (b, 0.0)]), t, b),
            ExpectationPattern::AlwaysFails
        );
        assert_eq!(
            classify(&by_arm(&[(t, 1.0), (b, 0.0)]), t, b),
            ExpectationPattern::OnlyWithSkill
        );
        assert_eq!(
            classify(&by_arm(&[(t, 0.0), (b, 1.0)]), t, b),
            ExpectationPattern::OnlyBaseline
        );
        assert_eq!(
            classify(&by_arm(&[(t, 1.0), (b, 0.5)]), t, b),
            ExpectationPattern::Flaky
        );
        assert_eq!(
            classify(&by_arm(&[(t, 1.0), (b, 0.0), ("variant", 1.0)]), t, b),
            ExpectationPattern::Mixed
        );
        assert_eq!(
            classify(&by_arm(&[("variant", 1.0), (b, 0.0)]), t, b),
            ExpectationPattern::Mixed
        );
        assert_eq!(classify(&BTreeMap::new(), t, b), ExpectationPattern::Mixed);
    }

    fn result(
        case_id: u32,
        arm: &str,
        repeat: u32,
        passes: &[bool],
        tokens: u64,
        duration: u64,
    ) -> CaseResult {
        let passed = passes.iter().filter(|p| **p).count() as u32;
        CaseResult {
            id: format!("{case_id}-{arm}-{repeat}"),
            run_id: "r".into(),
            case_id,
            case_name: format!("case-{case_id}"),
            arm: arm.into(),
            repeat,
            status: CaseStatus::Graded,
            expectations: passes
                .iter()
                .enumerate()
                .map(|(i, p)| ExpectationResult {
                    text: format!("E{i}"),
                    passed: *p,
                    evidence: String::new(),
                    source: ExpectationSource::Check,
                })
                .collect(),
            passed,
            failed: passes.len() as u32 - passed,
            total: passes.len() as u32,
            pass_rate: passed as f64 / passes.len() as f64,
            input_tokens: tokens,
            output_tokens: 0,
            duration_ms: duration,
            tool_calls: 2,
            cost_usd: Some(0.01),
            output_excerpt: String::new(),
            sandbox_path: String::new(),
            transcript_path: None,
            grader_notes: None,
            error: None,
            trace: None,
            grader_input_tokens: 0,
            grader_output_tokens: 0,
        }
    }

    fn traced(
        mut r: CaseResult,
        reads: u32,
        skill_reads: u32,
        thinking: u64,
        io: f64,
    ) -> CaseResult {
        r.trace = Some(ExecutionTrace {
            turns: 4,
            tool_calls: reads + 1,
            reads,
            writes: 1,
            skill_reads,
            files_read_count: reads,
            files_written_count: 1,
            thinking_tokens: thinking,
            text_tokens: 100,
            tool_input_tokens: 50,
            tool_result_tokens: 300,
            io_share: io,
            cache_read_tokens: 1000,
            cache_creation_tokens: 200,
            uncached_input_tokens: 20,
            tool_errors: if reads > 2 { 1 } else { 0 },
            model_time_ms: Some(1500),
            ..Default::default()
        });
        r
    }

    fn run() -> EvalRun {
        EvalRun {
            id: "run-1".into(),
            item_id: Some("item".into()),
            skill_name: "pdf-tools".into(),
            skill_hash: "abcdef1234567890".into(),
            config: RunConfig {
                item_id: "item".into(),
                arms: vec![
                    ArmSpec {
                        label: "with_skill".into(),
                        kind: ArmKind::WithSkill,
                    },
                    ArmSpec {
                        label: "without_skill".into(),
                        kind: ArmKind::WithoutSkill,
                    },
                ],
                case_ids: vec![],
                repeats: 2,
                concurrency: 1,
                executor_provider_id: "p".into(),
                grader_provider_id: None,
                timeout_secs: 60,
                include_trigger: false,
                baseline_label: None,
            },
            status: RunStatus::Running,
            progress: RunProgress::default(),
            workspace: "/w".into(),
            summary: None,
            error: None,
            created_at: 0,
            started_at: None,
            finished_at: None,
        }
    }

    #[test]
    fn aggregate_and_markdown() {
        let results = vec![
            result(1, "with_skill", 1, &[true, true], 2000, 3000),
            result(1, "with_skill", 2, &[true, true], 2200, 3100),
            result(1, "without_skill", 1, &[true, false], 1000, 2000),
            result(1, "without_skill", 2, &[true, false], 900, 2100),
            result(2, "with_skill", 1, &[true, true], 2100, 2900),
            result(2, "with_skill", 2, &[true, false], 2000, 40_000),
            result(2, "without_skill", 1, &[false, false], 1000, 2000),
            result(2, "without_skill", 2, &[false, false], 1100, 2200),
        ];
        let mut failed = result(2, "without_skill", 3, &[false], 0, 0);
        failed.status = CaseStatus::Timeout;
        failed.total = 0;
        let mut all = results.clone();
        all.push(failed);
        let b = aggregate(&run(), &all, None, "scripted", None);
        assert_eq!(b.baseline, "without_skill");
        assert_eq!(b.arms.len(), 2);
        let with = &b.arms[0];
        let without = &b.arms[1];
        assert_eq!(with.runs, 4);
        assert!(approx(with.pass_rate.mean, 0.875, 1e-9));
        assert_eq!(without.runs, 5, "four scored results plus the timeout");
        assert_eq!(without.errors, 1);
        // (0.5 + 0.5 + 0 + 0 + 0 (timeout)) / 5 = 0.2
        assert!(
            approx(without.pass_rate.mean, 0.2, 1e-9),
            "{}",
            without.pass_rate.mean
        );
        assert_eq!(without.pass_rate.n, 5);
        assert_eq!(with.cost_usd, Some(0.04));
        assert_eq!(b.deltas.len(), 1);
        let d = &b.deltas[0];
        assert!(approx(d.pass_rate_delta, 0.675, 1e-9));
        assert!(d.ci_low <= d.pass_rate_delta && d.pass_rate_delta <= d.ci_high);
        assert!(d.p_value.unwrap() < 0.05);
        assert!(d.tokens_delta > 1000.0);
        assert_eq!(b.verdict, Verdict::HelpsALot);
        assert_eq!(d.verdict, b.verdict);

        let patterns: Vec<ExpectationPattern> = b.expectations.iter().map(|r| r.pattern).collect();
        assert_eq!(b.expectations.len(), 4);
        assert!(patterns.contains(&ExpectationPattern::AlwaysPasses)); // case 1 E0
        assert!(patterns.contains(&ExpectationPattern::OnlyWithSkill)); // case 1 E1, case 2 E0
        assert!(patterns.contains(&ExpectationPattern::Flaky)); // case 2 E1
        assert!(
            b.notes
                .iter()
                .any(|n| n.contains("1 of 4 expectations pass in every arm")),
            "{:?}",
            b.notes
        );
        assert!(
            b.notes
                .iter()
                .any(|n| n.contains("× the tokens of without_skill")),
            "{:?}",
            b.notes
        );
        assert!(
            b.notes
                .iter()
                .any(|n| n.contains("failed or timed out in without_skill")),
            "{:?}",
            b.notes
        );
        assert!(
            b.notes.iter().any(|n| n.contains("took 40.0 s")),
            "{:?}",
            b.notes
        );
        assert!(
            b.notes.iter().any(|n| n.contains("repeat the run")),
            "{:?}",
            b.notes
        );

        let md = benchmark_markdown(&b);
        assert!(md.starts_with("# Benchmark: pdf-tools"));
        assert!(md.contains("| Arm | Runs | Errors | Pass rate |"));
        assert!(md.contains("| with_skill | 4 | 0 | 88% |"), "{md}");
        assert!(md.contains("| without_skill | 5 | 1 | 20% |"), "{md}");
        assert!(md.contains("## Deltas vs without_skill"));
        assert!(md.contains("Helps a lot"));
        assert!(md.contains("## Expectations"));
        assert!(md.contains("only with skill"));
        assert!(md.contains("## Notes"));
        assert!(!md.contains("## Trigger"));
    }

    #[test]
    fn aggregate_edge_cases() {
        let b = aggregate(&run(), &[], None, "x", Some("g"));
        assert_eq!(b.verdict, Verdict::Inconclusive);
        assert_eq!(b.deltas[0].p_value, None);
        assert_eq!(b.grader.as_deref(), Some("g"));
        assert!(b.expectations.is_empty());
        let md = benchmark_markdown(&b);
        assert!(md.contains("Inconclusive"));

        // Explicit baseline label and a variant arm.
        let mut r = run();
        r.config.arms.push(ArmSpec {
            label: "v2".into(),
            kind: ArmKind::Variant {
                path: "/tmp/v2".into(),
            },
        });
        r.config.baseline_label = Some("v2".into());
        assert_eq!(baseline_label(&r.config), "v2");
        assert_eq!(treatment_label(&r.config, "v2"), "with_skill");
        r.config.baseline_label = Some("nope".into());
        assert_eq!(baseline_label(&r.config), "without_skill");
        r.config.arms.remove(1);
        r.config.baseline_label = None;
        assert_eq!(baseline_label(&r.config), "v2");
        let results = vec![
            result(1, "with_skill", 1, &[true], 10, 10),
            result(1, "with_skill", 2, &[true], 10, 10),
            result(1, "v2", 1, &[true], 10, 10),
            result(1, "v2", 2, &[true], 10, 10),
        ];
        let b = aggregate(&r, &results, None, "x", None);
        assert_eq!(b.baseline, "v2");
        assert_eq!(b.deltas.len(), 1);
        assert_eq!(b.verdict, Verdict::Neutral);
        assert_eq!(b.expectations[0].pattern, ExpectationPattern::AlwaysPasses);

        let t = TriggerSummary {
            total: 2,
            correct: 1,
            precision: 0.5,
            recall: 1.0,
            f1: 0.667,
            results: vec![],
        };
        let b = aggregate(&r, &results, Some(t), "x", None);
        assert!(benchmark_markdown(&b).contains("## Trigger"));
        assert_eq!(num(1234567.0), "1,234,567");
        assert_eq!(num(999.0), "999");
        assert_eq!(num(-1500.0), "-1,500");
    }

    #[test]
    fn activity_is_aggregated_and_noted() {
        // Without traces there is no activity section and no activity note.
        let plain = aggregate(
            &run(),
            &[result(1, "with_skill", 1, &[true], 10, 10)],
            None,
            "x",
            None,
        );
        assert!(plain.arms.iter().all(|a| a.activity.is_none()));
        assert!(!benchmark_markdown(&plain).contains("## Activity"));

        let results = vec![
            traced(
                result(1, "with_skill", 1, &[true, true], 2000, 3000),
                3,
                1,
                900,
                0.6,
            ),
            traced(
                result(1, "with_skill", 2, &[true, true], 2200, 3100),
                3,
                0,
                700,
                0.5,
            ),
            traced(
                result(1, "without_skill", 1, &[true, false], 1000, 2000),
                1,
                0,
                300,
                0.2,
            ),
            traced(
                result(1, "without_skill", 2, &[true, false], 1, 2100),
                1,
                0,
                300,
                0.3,
            ),
            // An untraced result is ignored by the activity statistics but still scored.
            result(2, "with_skill", 1, &[true, true], 2100, 2900),
        ];
        let b = aggregate(&run(), &results, None, "x", None);
        let with = b.arms[0].activity.as_ref().expect("with_skill activity");
        let without = b.arms[1].activity.as_ref().expect("baseline activity");
        assert_eq!((with.traced, without.traced), (2, 2));
        assert!(approx(with.reads.mean, 3.0, 1e-9));
        assert!(approx(with.skill_opened, 0.5, 1e-9));
        assert!(approx(without.skill_opened, 0.0, 1e-9));
        assert!(approx(with.thinking_tokens.mean, 800.0, 1e-9));
        assert!(approx(with.io_share.mean, 0.55, 1e-9));
        assert!(approx(with.fresh_input_tokens.mean, 220.0, 1e-9));
        assert_eq!(with.model_time_ms.n, 2);
        assert_eq!(with.tool_time_ms.n, 0, "no timed tool steps in the fixture");
        assert!(approx(with.tool_errors.mean, 1.0, 1e-9));

        let notes = b.notes.join("\n");
        assert!(
            notes.contains("with_skill opened the skill in only 1 of 2 run(s)"),
            "{notes}"
        );
        assert!(!notes.contains("without_skill opened"), "{notes}");
        assert!(
            notes.contains("with_skill reads 3.0× as many files"),
            "{notes}"
        );
        assert!(
            notes.contains("with_skill thinks 2.7× as much as without_skill"),
            "{notes}"
        );
        assert!(
            notes.contains("with_skill spends 55% of its traffic on tool IO vs 25%"),
            "{notes}"
        );
        assert!(
            notes.contains("with_skill hit 1.0 tool error(s)"),
            "{notes}"
        );

        let md = benchmark_markdown(&b);
        assert!(md.contains("## Activity"), "{md}");
        assert!(
            md.contains("| Metric | with_skill | without_skill |"),
            "{md}"
        );
        assert!(md.contains("| Opened the skill | 50% | 0% |"), "{md}");
        assert!(md.contains("| File reads | 3.0 | 1.0 |"), "{md}");
        assert!(
            md.contains("| Thinking tokens (est.) | 800 | 300 |"),
            "{md}"
        );
        assert!(md.contains("| Tool time | — | — |"), "{md}");

        // A skill arm that always opened the skill gets the positive note.
        let results = vec![
            traced(result(1, "with_skill", 1, &[true], 1, 1), 1, 1, 10, 0.1),
            traced(result(1, "with_skill", 2, &[true], 1, 1), 1, 2, 10, 0.1),
            traced(result(1, "without_skill", 1, &[true], 1, 1), 1, 0, 10, 0.1),
            traced(result(1, "without_skill", 2, &[true], 1, 1), 1, 0, 10, 0.1),
        ];
        let b = aggregate(&run(), &results, None, "x", None);
        assert!(
            b.notes
                .iter()
                .any(|n| n == "with_skill opened the skill in every run (2 of 2)"),
            "{:?}",
            b.notes
        );
        assert!(
            !b.notes.iter().any(|n| n.contains("reads")),
            "{:?}",
            b.notes
        );
    }
}
