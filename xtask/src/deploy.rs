use crate::flags;
use anyhow::Context;
use core::time::Duration;
use std::collections::HashMap;
use time::format_description::well_known::Iso8601;
use xshell::Shell;

const FILTER_BENCHES: &[&str] = &[
    "chain5",
    "chain100",
    "hourglass3",
    "hourglass6",
    "star10",
    "star30",
    "tree_k3d2",
    "tree_k3d3",
];

#[derive(Debug, serde::Deserialize)]
pub(crate) struct RawCommit {
    pub(crate) commit_title: String,
    pub(crate) commit_date: String,
    pub(crate) commit_sha: String,
    pub(crate) results: Vec<RawBench>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct RawBench {
    pub(crate) name: String,
    pub(crate) scenarios: Vec<RawScenario>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct RawScenario {
    pub(crate) name: String,
    #[expect(dead_code)]
    pub(crate) counts: RawCounts,
    pub(crate) timings: Vec<RawTimings>,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct RawCounts {
    #[expect(dead_code)]
    pub(crate) query: usize,
    #[expect(dead_code)]
    pub(crate) eval: usize,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct RawTimings {
    pub(crate) query: u64,
    #[expect(dead_code)]
    pub(crate) eval: u64,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct ChartData {
    pub(crate) commits: Vec<ChartCommit>,
    pub(crate) benches: HashMap<Bench, HashMap<Scenario, Sequence>>,
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct ChartCommit {
    pub(crate) sha: String,
    pub(crate) title: String,
}

type Bench = String;
type Scenario = String;
type Sequence = Vec<Mean>;
type Mean = Option<f64>;

impl flags::Deploy {
    pub(crate) fn run(self, sh: &Shell) -> anyhow::Result<()> {
        let Self { dry_run } = self;

        let mut raw_commits = read_raws(sh)?;
        raw_commits.sort_by_cached_key(|commit| {
            time::OffsetDateTime::parse(&commit.commit_date, &Iso8601::DEFAULT)
                .unwrap()
                .to_utc()
        });
        let benches_by_commit: Vec<HashMap<Bench, HashMap<Scenario, f64>>> =
            raw_commits.iter().map(commit_benches).collect();
        let data = serde_json::to_string(&ChartData {
            commits: chart_commits(&raw_commits),
            benches: transpose_benches(
                &benches_by_commit,
                &bench_names(&benches_by_commit),
                &scenario_names(&benches_by_commit),
            ),
        })?;

        if !dry_run {
            let index = sh.read_file("template/index.html")?;
            let script = sh.read_file("template/script.js")?;
            let index = index.replacen("[\"data\"]", &data, 1);
            let index = index.replacen("[\"script\"]", &script, 1);
            sh.write_file("out/index.html", index)?;
        }

        Ok(())
    }
}

fn chart_commits(raw_commits: &[RawCommit]) -> Vec<ChartCommit> {
    raw_commits
        .iter()
        .map(|commit| ChartCommit {
            sha: commit.commit_sha.chars().take(8).collect(),
            title: commit.commit_title.clone(),
        })
        .collect()
}

fn read_raws(sh: &Shell) -> anyhow::Result<Vec<RawCommit>> {
    sh.read_dir("raw")?
        .into_iter()
        .map(|path| {
            let text = sh
                .read_file(&path)
                .with_context(|| format!("cannot read {path:?}"))?;
            let json = serde_json::from_str::<RawCommit>(&text)
                .with_context(|| format!("cannot parse {path:?}"))?;
            Ok(json)
        })
        .collect()
}

fn commit_benches(commit: &RawCommit) -> HashMap<Bench, HashMap<Scenario, f64>> {
    commit.results.iter().map(bench_entry).collect()
}

fn bench_entry(bench: &RawBench) -> (Bench, HashMap<Scenario, f64>) {
    let name = bench.name.clone();
    let scenarios = bench.scenarios.iter().map(scenario_time).collect();
    (name, scenarios)
}

fn scenario_time(scenario: &RawScenario) -> (String, f64) {
    let timing_count = scenario.timings.len() as f64;
    let time_query = scenario
        .timings
        .iter()
        .map(|timings| partial_mean_micros(timing_count, timings))
        .sum();
    (scenario.name.clone(), time_query)
}

fn partial_mean_micros(timing_count: f64, timings: &RawTimings) -> f64 {
    let nanos = timings.query as f64;
    let micros = nanos / (Duration::from_micros(1).as_nanos() as f64);
    micros / timing_count
}

fn bench_names(benches_by_commit: &[HashMap<Bench, HashMap<Scenario, f64>>]) -> Vec<Bench> {
    let mut bench_names: Vec<Bench> = benches_by_commit
        .iter()
        .flat_map(|benches| benches.keys().cloned())
        .filter(|bench| !FILTER_BENCHES.contains(&bench.as_str()))
        .collect();
    bench_names.sort();
    bench_names.dedup();
    bench_names
}

fn scenario_names(benches_by_commit: &[HashMap<Bench, HashMap<Scenario, f64>>]) -> Vec<Scenario> {
    let mut scenario_names: Vec<Scenario> = benches_by_commit
        .iter()
        .flat_map(|benches| {
            benches
                .values()
                .flat_map(|scenarios| scenarios.keys().cloned())
        })
        .collect();
    scenario_names.sort();
    scenario_names.dedup();
    scenario_names
}

fn transpose_benches(
    benches_by_commit: &[HashMap<String, HashMap<String, f64>>],
    bench_names: &[String],
    scenario_names: &[String],
) -> HashMap<Bench, HashMap<Scenario, Sequence>> {
    bench_names
        .iter()
        .map(|bench_name| {
            let scenarios: HashMap<String, Sequence> = scenario_names
                .iter()
                .map(|scenario_name| {
                    let means: Sequence = benches_by_commit
                        .iter()
                        .map(|commit_benches| {
                            commit_benches
                                .get(bench_name)
                                .and_then(|scenarios_by_bench| {
                                    scenarios_by_bench.get(scenario_name)
                                })
                                .copied()
                        })
                        .collect();
                    (scenario_name.clone(), means)
                })
                .collect();
            (bench_name.clone(), scenarios)
        })
        .collect()
}
