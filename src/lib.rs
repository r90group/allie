use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const PRODUCT_LINE: &str = "Allie: accessibility evidence for every release.";
const NEXT_STEP: &str = "Next implementation target: allie run --manifest <flow.yml>";
const EVIDENCE_SCHEMA: &str = "allie.evidence.v0";
const WORKER_REQUEST_SCHEMA: &str = "allie.worker.request.v0";
const WORKER_RESPONSE_SCHEMA: &str = "allie.worker.response.v0";

#[derive(Debug)]
pub enum AllieError {
    Io {
        context: String,
        source: io::Error,
    },
    Json {
        context: String,
        source: serde_json::Error,
    },
    Yaml {
        context: String,
        source: serde_yaml::Error,
    },
    InvalidManifest(String),
    Worker(String),
}

impl Display for AllieError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::Json { context, source } => write!(f, "{context}: {source}"),
            Self::Yaml { context, source } => write!(f, "{context}: {source}"),
            Self::InvalidManifest(message) => write!(f, "invalid manifest: {message}"),
            Self::Worker(message) => write!(f, "worker failed: {message}"),
        }
    }
}

impl Error for AllieError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json { source, .. } => Some(source),
            Self::Yaml { source, .. } => Some(source),
            Self::InvalidManifest(_) | Self::Worker(_) => None,
        }
    }
}

type Result<T> = std::result::Result<T, AllieError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitClass {
    Success,
    BlockingFinding,
    InfrastructureFailure,
    Usage,
}

impl ExitClass {
    pub fn code(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::BlockingFinding => 1,
            Self::InfrastructureFailure => 2,
            Self::Usage => 64,
        }
    }

    fn packet_status(self) -> &'static str {
        match self {
            Self::Success => "pass",
            Self::BlockingFinding => "fail",
            Self::InfrastructureFailure | Self::Usage => "error",
        }
    }
}

#[derive(Debug)]
pub struct RunReceipt {
    pub run_id: String,
    pub exit_class: ExitClass,
    pub evidence_path: PathBuf,
    pub report_path: PathBuf,
}

#[derive(Debug)]
struct RunOptions {
    manifest_path: PathBuf,
    out_dir: PathBuf,
}

pub fn run_cli(args: impl IntoIterator<Item = String>) -> i32 {
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    run_cli_with_io(args, &mut stdout, &mut stderr)
}

pub fn run_cli_with_io(
    args: impl IntoIterator<Item = String>,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let args = args.into_iter().collect::<Vec<_>>();

    if args.is_empty() {
        let _ = writeln!(stdout, "{PRODUCT_LINE}");
        let _ = writeln!(stdout, "{NEXT_STEP}");
        let _ = writeln!(
            stdout,
            "Run: allie run --manifest examples/login-flow.yml --out .allie/runs/latest"
        );
        return ExitClass::Success.code();
    }

    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        print_usage(stdout);
        return ExitClass::Success.code();
    }

    match args.first().map(String::as_str) {
        Some("run") => match parse_run_options(&args[1..]) {
            Ok(options) => match run_v0(options) {
                Ok(receipt) => {
                    let _ = writeln!(stdout, "Allie evidence run: {}", receipt.run_id);
                    let _ = writeln!(stdout, "Evidence: {}", receipt.evidence_path.display());
                    let _ = writeln!(stdout, "Report: {}", receipt.report_path.display());
                    let _ = writeln!(stdout, "Status: {}", receipt.exit_class.packet_status());
                    receipt.exit_class.code()
                }
                Err(error) => {
                    let _ = writeln!(stderr, "allie: {error}");
                    ExitClass::InfrastructureFailure.code()
                }
            },
            Err(error) => {
                let _ = writeln!(stderr, "allie: {error}");
                print_usage(stderr);
                ExitClass::Usage.code()
            }
        },
        _ => {
            let _ = writeln!(stderr, "allie: unknown command");
            print_usage(stderr);
            ExitClass::Usage.code()
        }
    }
}

fn print_usage(writer: &mut dyn Write) {
    let _ = writeln!(
        writer,
        "Usage:\n  allie run --manifest <flow.yml> --out <output-dir>"
    );
}

fn parse_run_options(args: &[String]) -> std::result::Result<RunOptions, String> {
    let mut manifest_path = None;
    let mut out_dir = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--manifest" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--manifest requires a path".to_string())?;
                manifest_path = Some(PathBuf::from(value));
            }
            "--out" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--out requires a directory".to_string())?;
                out_dir = Some(PathBuf::from(value));
            }
            unexpected => return Err(format!("unexpected argument: {unexpected}")),
        }
        index += 1;
    }

    Ok(RunOptions {
        manifest_path: manifest_path.ok_or_else(|| "--manifest is required".to_string())?,
        out_dir: out_dir.ok_or_else(|| "--out is required".to_string())?,
    })
}

fn run_v0(options: RunOptions) -> Result<RunReceipt> {
    let started_at = now_utc();
    let manifest = FlowManifest::load(&options.manifest_path)?;
    manifest.validate()?;
    fs::create_dir_all(&options.out_dir).map_err(|source| AllieError::Io {
        context: format!("create output directory {}", options.out_dir.display()),
        source,
    })?;

    let run_id = new_run_id();
    let request_path = options.out_dir.join("worker-request.json");
    let response_path = options.out_dir.join("worker-response.json");
    let request = WorkerRequest::from_manifest(
        &run_id,
        &manifest,
        &options.manifest_path,
        &options.out_dir.join("artifacts"),
    )?;
    write_json_pretty(&request_path, &request)?;

    invoke_worker(&request_path, &response_path)?;

    let response_text = fs::read_to_string(&response_path).map_err(|source| AllieError::Io {
        context: format!("read worker response {}", response_path.display()),
        source,
    })?;
    let response = serde_json::from_str::<WorkerResponse>(&response_text).map_err(|source| {
        AllieError::Json {
            context: format!("parse worker response {}", response_path.display()),
            source,
        }
    })?;
    response.validate()?;

    write_packet_and_report(
        &manifest,
        &options.manifest_path,
        &options.out_dir,
        response,
        started_at,
        now_utc(),
        run_id,
    )
}

fn invoke_worker(request_path: &Path, response_path: &Path) -> Result<()> {
    let worker_script = std::env::var_os("ALLIE_BROWSER_WORKER")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("workers/browser/run.mjs")
        });

    if !worker_script.exists() {
        return Err(AllieError::Worker(format!(
            "worker script not found at {}",
            worker_script.display()
        )));
    }

    let output = Command::new("node")
        .arg(&worker_script)
        .arg("--request")
        .arg(request_path)
        .arg("--response")
        .arg(response_path)
        .output()
        .map_err(|source| AllieError::Io {
            context: format!("spawn worker {}", worker_script.display()),
            source,
        })?;

    if !output.status.success() {
        return Err(AllieError::Worker(format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct FlowManifest {
    id: String,
    name: String,
    app_name: String,
    environment: String,
    auth_profile: Option<String>,
    target: ManifestTarget,
    policy: ManifestPolicy,
    browser: BrowserSettings,
    flow: ManifestFlow,
}

impl FlowManifest {
    fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).map_err(|source| AllieError::Io {
            context: format!("read manifest {}", path.display()),
            source,
        })?;
        serde_yaml::from_str(&text).map_err(|source| AllieError::Yaml {
            context: format!("parse manifest {}", path.display()),
            source,
        })
    }

    fn validate(&self) -> Result<()> {
        require_name("manifest id", &self.id)?;
        require_name("flow id", &self.flow.id)?;
        require_name("policy profile", &self.policy.profile)?;
        if self.flow.states.is_empty() {
            return Err(AllieError::InvalidManifest(
                "flow.states must contain at least one state".to_string(),
            ));
        }

        if self.target.kind == "local_fixture" {
            if self.target.fixture_dir.is_none() {
                return Err(AllieError::InvalidManifest(
                    "local_fixture target requires fixture_dir".to_string(),
                ));
            }
        } else if self.target.base_url.is_none() {
            return Err(AllieError::InvalidManifest(
                "non-fixture target requires base_url".to_string(),
            ));
        }

        for state in &self.flow.states {
            require_name("state id", &state.id)?;
            if !state.path.starts_with('/') {
                return Err(AllieError::InvalidManifest(format!(
                    "state {} path must start with /",
                    state.id
                )));
            }
        }

        Ok(())
    }
}

fn require_name(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(AllieError::InvalidManifest(format!("{label} is required")));
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ManifestTarget {
    kind: String,
    fixture_dir: Option<PathBuf>,
    base_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ManifestPolicy {
    profile: String,
    blocking_classes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct BrowserSettings {
    viewport: Viewport,
    color_scheme: String,
    reduced_motion: String,
    locale: String,
    zoom: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Viewport {
    width: u32,
    height: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ManifestFlow {
    id: String,
    description: String,
    states: Vec<ManifestState>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ManifestState {
    id: String,
    path: String,
    description: String,
    required: bool,
    axe: bool,
    screenshot: bool,
}

#[derive(Debug, Serialize)]
struct WorkerRequest {
    schema: &'static str,
    run_id: String,
    manifest_id: String,
    target: WorkerTarget,
    browser: BrowserSettings,
    states: Vec<ManifestState>,
    artifacts_dir: String,
}

impl WorkerRequest {
    fn from_manifest(
        run_id: &str,
        manifest: &FlowManifest,
        manifest_path: &Path,
        artifacts_dir: &Path,
    ) -> Result<Self> {
        let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
        let target = WorkerTarget {
            kind: manifest.target.kind.clone(),
            fixture_dir: manifest
                .target
                .fixture_dir
                .as_ref()
                .map(|path| normalize_relative(manifest_dir, path)),
            base_url: manifest.target.base_url.clone(),
        };

        Ok(Self {
            schema: WORKER_REQUEST_SCHEMA,
            run_id: run_id.to_string(),
            manifest_id: manifest.id.clone(),
            target,
            browser: manifest.browser.clone(),
            states: manifest.flow.states.clone(),
            artifacts_dir: artifacts_dir.to_string_lossy().to_string(),
        })
    }
}

fn normalize_relative(base: &Path, path: &Path) -> String {
    if path.is_absolute() {
        path.to_string_lossy().to_string()
    } else {
        base.join(path).to_string_lossy().to_string()
    }
}

#[derive(Debug, Serialize)]
struct WorkerTarget {
    kind: String,
    fixture_dir: Option<String>,
    base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorkerResponse {
    schema: String,
    status: WorkerRunStatus,
    actual_base_url: Option<String>,
    #[serde(default)]
    states: Vec<WorkerStateResult>,
    #[serde(default)]
    errors: Vec<String>,
}

impl WorkerResponse {
    fn validate(&self) -> Result<()> {
        if self.schema != WORKER_RESPONSE_SCHEMA {
            return Err(AllieError::Worker(format!(
                "unexpected worker response schema {}",
                self.schema
            )));
        }
        if matches!(self.status, WorkerRunStatus::Error) {
            return Err(AllieError::Worker(self.errors.join("; ")));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum WorkerRunStatus {
    Passed,
    Failed,
    Error,
}

#[derive(Debug, Deserialize)]
struct WorkerStateResult {
    id: String,
    route: String,
    url: String,
    title: String,
    http_status: Option<u16>,
    screenshot_path: Option<String>,
    axe_json_path: Option<String>,
    #[serde(default)]
    axe_violations: Vec<AxeViolation>,
    #[serde(default)]
    console_errors: Vec<String>,
    #[serde(default)]
    network_errors: Vec<String>,
    #[serde(default)]
    state_errors: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct AxeViolation {
    id: String,
    impact: Option<String>,
    help: Option<String>,
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    nodes: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct EvidencePacket {
    schema: &'static str,
    summary: PacketSummary,
    run: RunMetadata,
    target: TargetMetadata,
    policy: PolicyMetadata,
    coverage: Coverage,
    artifacts: Vec<ArtifactMetadata>,
    findings: Vec<Finding>,
    verdicts: Vec<Verdict>,
    waivers: Vec<serde_json::Value>,
    review: Vec<serde_json::Value>,
    replay: Replay,
}

#[derive(Debug, Serialize, Deserialize)]
struct PacketSummary {
    status: String,
    exit_code: i32,
    deterministic_failures: usize,
    scripted_failures: usize,
    states_captured: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct RunMetadata {
    id: String,
    started_at: String,
    finished_at: String,
    allie_version: String,
    git_sha: String,
    git_branch: String,
    ci_provider: Option<String>,
    actor: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TargetMetadata {
    base_url: Option<String>,
    environment: String,
    app_name: String,
    auth_profile: String,
    flow_manifest: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PolicyMetadata {
    profile: String,
    blocking_classes: Vec<String>,
    model_provider_allowlist: Vec<String>,
    zdr_required: bool,
    redaction_profile: String,
    budget: PolicyBudget,
}

#[derive(Debug, Serialize, Deserialize)]
struct PolicyBudget {
    model_calls: u32,
    max_states: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Coverage {
    routes_visited: Vec<String>,
    surfaces_discovered: Vec<String>,
    flows_exercised: Vec<String>,
    states_captured: Vec<String>,
    state_metadata: Vec<StateMetadata>,
    standards_obligations_evaluated: Vec<String>,
    obligations_not_tested: Vec<String>,
    obligations_requiring_human_review: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StateMetadata {
    id: String,
    route: String,
    url: String,
    title: String,
    http_status: Option<u16>,
    console_errors: Vec<String>,
    network_errors: Vec<String>,
    state_errors: Vec<String>,
}

#[derive(Debug)]
struct ContractFailure {
    state_id: String,
    route: String,
    message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ArtifactMetadata {
    id: String,
    #[serde(rename = "type")]
    artifact_type: String,
    path: String,
    hash: String,
    redaction_status: String,
    related_flow_state: Option<String>,
    creation_tool: String,
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Finding {
    id: String,
    title: String,
    description: String,
    evidence_class: String,
    standard_obligation: String,
    severity: String,
    status: String,
    confidence: String,
    source: String,
    affected_route: String,
    affected_state: String,
    artifact_refs: Vec<String>,
    suggested_remediation: String,
    replay_command: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Verdict {
    obligation: String,
    status: String,
    confidence: String,
    evidence_class: String,
    source: String,
    affected_states: Vec<String>,
    finding_refs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Replay {
    command: String,
    manifest_path: String,
    environment_requirements: Vec<String>,
    credential_profile: String,
    browser: BrowserSettings,
    seed_data: Vec<String>,
    known_nondeterminism: Vec<String>,
}

fn write_packet_and_report(
    manifest: &FlowManifest,
    manifest_path: &Path,
    out_dir: &Path,
    response: WorkerResponse,
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    run_id: String,
) -> Result<RunReceipt> {
    fs::create_dir_all(out_dir).map_err(|source| AllieError::Io {
        context: format!("create output directory {}", out_dir.display()),
        source,
    })?;

    let replay_command = format!(
        "cargo run --locked -- run --manifest {} --out {}",
        manifest_path.display(),
        out_dir.display()
    );
    let contract_failures = response_contract_failures(manifest, &response);
    let exit_class = exit_class_for_response(&response, &contract_failures);
    let deterministic_failures = response
        .states
        .iter()
        .map(|state| state.axe_violations.len())
        .sum::<usize>();
    let scripted_failures = response
        .states
        .iter()
        .map(|state| state.state_errors.len())
        .sum::<usize>()
        + contract_failures.len();

    let mut artifacts = worker_artifacts(out_dir, &response, finished_at)?;
    let findings =
        findings_from_response(&response, &artifacts, &contract_failures, &replay_command);
    let verdicts = verdicts_from_findings(manifest, &response, &findings);
    let mut packet = EvidencePacket {
        schema: EVIDENCE_SCHEMA,
        summary: PacketSummary {
            status: exit_class.packet_status().to_string(),
            exit_code: exit_class.code(),
            deterministic_failures,
            scripted_failures,
            states_captured: response.states.len(),
        },
        run: RunMetadata {
            id: run_id.clone(),
            started_at: started_at.to_rfc3339(),
            finished_at: finished_at.to_rfc3339(),
            allie_version: env!("CARGO_PKG_VERSION").to_string(),
            git_sha: git_metadata(&["rev-parse", "--short", "HEAD"]).unwrap_or_default(),
            git_branch: git_metadata(&["branch", "--show-current"]).unwrap_or_default(),
            ci_provider: std::env::var("CI").ok().map(|_| "generic-ci".to_string()),
            actor: std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
        },
        target: TargetMetadata {
            base_url: response
                .actual_base_url
                .clone()
                .or_else(|| manifest.target.base_url.clone()),
            environment: manifest.environment.clone(),
            app_name: manifest.app_name.clone(),
            auth_profile: manifest
                .auth_profile
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            flow_manifest: manifest_path.to_string_lossy().to_string(),
        },
        policy: PolicyMetadata {
            profile: manifest.policy.profile.clone(),
            blocking_classes: manifest.policy.blocking_classes.clone(),
            model_provider_allowlist: Vec::new(),
            zdr_required: true,
            redaction_profile: "local-fixture-none".to_string(),
            budget: PolicyBudget {
                model_calls: 0,
                max_states: manifest.flow.states.len(),
            },
        },
        coverage: coverage_from_response(manifest, &response, &findings),
        artifacts: Vec::new(),
        findings,
        verdicts,
        waivers: Vec::new(),
        review: Vec::new(),
        replay: Replay {
            command: replay_command,
            manifest_path: manifest_path.to_string_lossy().to_string(),
            environment_requirements: vec![
                "npm install".to_string(),
                "npx playwright install chromium".to_string(),
            ],
            credential_profile: manifest
                .auth_profile
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            browser: manifest.browser.clone(),
            seed_data: vec!["checked-in fixture fixtures/login".to_string()],
            known_nondeterminism: Vec::new(),
        },
    };

    packet.artifacts = artifacts.clone();
    let report_path = out_dir.join("report.html");
    write_string(&report_path, &render_report(&packet))?;

    artifacts.push(artifact_for_path(
        "report-html",
        "html_report",
        out_dir,
        &report_path,
        None,
        "allie-report-writer",
        finished_at,
    )?);
    packet.artifacts = artifacts;
    let evidence_path = out_dir.join("evidence.json");
    write_json_pretty(&evidence_path, &packet)?;

    Ok(RunReceipt {
        run_id,
        exit_class,
        evidence_path,
        report_path,
    })
}

fn exit_class_for_response(
    response: &WorkerResponse,
    contract_failures: &[ContractFailure],
) -> ExitClass {
    if response
        .states
        .iter()
        .any(|state| !state.axe_violations.is_empty() || !state.state_errors.is_empty())
        || !contract_failures.is_empty()
        || matches!(response.status, WorkerRunStatus::Failed)
    {
        ExitClass::BlockingFinding
    } else {
        ExitClass::Success
    }
}

fn worker_artifacts(
    out_dir: &Path,
    response: &WorkerResponse,
    timestamp: DateTime<Utc>,
) -> Result<Vec<ArtifactMetadata>> {
    let mut artifacts = Vec::new();
    for state in &response.states {
        if let Some(path) = &state.axe_json_path {
            artifacts.push(artifact_for_path(
                &format!("axe-json-{}", state.id),
                "axe_json",
                out_dir,
                &out_dir.join(path),
                Some(state.id.clone()),
                "playwright-axe-worker",
                timestamp,
            )?);
        }
        if let Some(path) = &state.screenshot_path {
            artifacts.push(artifact_for_path(
                &format!("screenshot-{}", state.id),
                "screenshot",
                out_dir,
                &out_dir.join(path),
                Some(state.id.clone()),
                "playwright-axe-worker",
                timestamp,
            )?);
        }
    }
    Ok(artifacts)
}

fn artifact_for_path(
    id: &str,
    artifact_type: &str,
    out_dir: &Path,
    path: &Path,
    related_flow_state: Option<String>,
    creation_tool: &str,
    timestamp: DateTime<Utc>,
) -> Result<ArtifactMetadata> {
    Ok(ArtifactMetadata {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        path: path_relative_to(out_dir, path),
        hash: format!("sha256:{}", sha256_file(path)?),
        redaction_status: "not_redacted_local_fixture".to_string(),
        related_flow_state,
        creation_tool: creation_tool.to_string(),
        timestamp: timestamp.to_rfc3339(),
    })
}

fn findings_from_response(
    response: &WorkerResponse,
    artifacts: &[ArtifactMetadata],
    contract_failures: &[ContractFailure],
    replay_command: &str,
) -> Vec<Finding> {
    let mut findings = response
        .states
        .iter()
        .flat_map(|state| {
            state
                .axe_violations
                .iter()
                .enumerate()
                .map(move |(index, violation)| {
                    let refs = artifacts
                        .iter()
                        .filter(|artifact| artifact.related_flow_state.as_deref() == Some(&state.id))
                        .map(|artifact| artifact.id.clone())
                        .collect::<Vec<_>>();
                    Finding {
                        id: format!("{}-axe-{}-{}", state.id, violation.id, index + 1),
                        title: violation
                            .help
                            .clone()
                            .unwrap_or_else(|| violation.id.clone()),
                        description: violation.description.clone().unwrap_or_else(|| {
                            format!("axe-core reported {} affected node(s)", violation.nodes)
                        }),
                        evidence_class: "deterministic".to_string(),
                        standard_obligation: obligation_from_tags(&violation.tags),
                        severity: violation
                            .impact
                            .clone()
                            .unwrap_or_else(|| "unknown".to_string()),
                        status: "fail".to_string(),
                        confidence: "machine_proven".to_string(),
                        source: "axe-core".to_string(),
                        affected_route: state.route.clone(),
                        affected_state: state.id.clone(),
                        artifact_refs: refs,
                        suggested_remediation: format!(
                            "Review axe rule {} in the linked raw axe JSON and rerun the replay command.",
                            violation.id
                        ),
                        replay_command: replay_command.to_string(),
                    }
                })
        })
        .collect::<Vec<_>>();

    for state in &response.states {
        for (index, message) in state.state_errors.iter().enumerate() {
            findings.push(Finding {
                id: format!("{}-state-error-{}", state.id, index + 1),
                title: "Required route state failed".to_string(),
                description: message.clone(),
                evidence_class: "scripted".to_string(),
                standard_obligation: "required-route-state".to_string(),
                severity: "blocking".to_string(),
                status: "fail".to_string(),
                confidence: "script_observed".to_string(),
                source: "playwright-worker".to_string(),
                affected_route: state.route.clone(),
                affected_state: state.id.clone(),
                artifact_refs: Vec::new(),
                suggested_remediation:
                    "Fix the route or manifest path, then rerun the replay command.".to_string(),
                replay_command: replay_command.to_string(),
            });
        }
    }

    for (index, failure) in contract_failures.iter().enumerate() {
        findings.push(Finding {
            id: format!("{}-contract-failure-{}", failure.state_id, index + 1),
            title: "Required evidence artifact missing".to_string(),
            description: failure.message.clone(),
            evidence_class: "scripted".to_string(),
            standard_obligation: "required-evidence-artifact".to_string(),
            severity: "blocking".to_string(),
            status: "fail".to_string(),
            confidence: "script_observed".to_string(),
            source: "allie-evidence-contract".to_string(),
            affected_route: failure.route.clone(),
            affected_state: failure.state_id.clone(),
            artifact_refs: Vec::new(),
            suggested_remediation:
                "Fix the worker response or manifest requirements, then rerun the replay command."
                    .to_string(),
            replay_command: replay_command.to_string(),
        });
    }

    findings
}

fn obligation_from_tags(tags: &[String]) -> String {
    tags.iter()
        .find(|tag| tag.starts_with("wcag"))
        .cloned()
        .unwrap_or_else(|| "wcag-review".to_string())
}

fn verdicts_from_findings(
    manifest: &FlowManifest,
    response: &WorkerResponse,
    findings: &[Finding],
) -> Vec<Verdict> {
    if findings.is_empty() {
        return vec![Verdict {
            obligation: manifest.policy.profile.clone(),
            status: "pass".to_string(),
            confidence: "machine_proven".to_string(),
            evidence_class: "deterministic".to_string(),
            source: "axe-core".to_string(),
            affected_states: response
                .states
                .iter()
                .map(|state| state.id.clone())
                .collect(),
            finding_refs: Vec::new(),
        }];
    }

    findings
        .iter()
        .map(|finding| Verdict {
            obligation: finding.standard_obligation.clone(),
            status: "fail".to_string(),
            confidence: finding.confidence.clone(),
            evidence_class: finding.evidence_class.clone(),
            source: finding.source.clone(),
            affected_states: vec![finding.affected_state.clone()],
            finding_refs: vec![finding.id.clone()],
        })
        .collect()
}

fn coverage_from_response(
    manifest: &FlowManifest,
    response: &WorkerResponse,
    findings: &[Finding],
) -> Coverage {
    let mut routes = BTreeSet::new();
    let mut states = BTreeSet::new();
    let mut obligations = BTreeSet::new();

    for state in &response.states {
        routes.insert(state.route.clone());
        states.insert(state.id.clone());
    }

    if findings.is_empty() {
        obligations.insert(manifest.policy.profile.clone());
    } else {
        for finding in findings {
            obligations.insert(finding.standard_obligation.clone());
        }
    }

    Coverage {
        routes_visited: routes.into_iter().collect(),
        surfaces_discovered: vec![manifest.app_name.clone()],
        flows_exercised: vec![manifest.flow.id.clone()],
        states_captured: states.into_iter().collect(),
        state_metadata: response
            .states
            .iter()
            .map(|state| StateMetadata {
                id: state.id.clone(),
                route: state.route.clone(),
                url: state.url.clone(),
                title: state.title.clone(),
                http_status: state.http_status,
                console_errors: state.console_errors.clone(),
                network_errors: state.network_errors.clone(),
                state_errors: state.state_errors.clone(),
            })
            .collect(),
        standards_obligations_evaluated: obligations.into_iter().collect(),
        obligations_not_tested: vec![
            "keyboard traversal".to_string(),
            "zoom and reflow".to_string(),
            "human equivalent access review".to_string(),
        ],
        obligations_requiring_human_review: vec![
            "content meaning and cognitive load".to_string(),
            "manual assistive technology review".to_string(),
        ],
    }
}

fn render_report(packet: &EvidencePacket) -> String {
    let findings = if packet.findings.is_empty() {
        "<p>No deterministic axe failures were reported for the captured states.</p>".to_string()
    } else {
        let items = packet
            .findings
            .iter()
            .map(|finding| {
                format!(
                    "<li><strong>{}</strong><br><span>{}</span><br><code>{}</code></li>",
                    escape_html(&finding.title),
                    escape_html(&finding.description),
                    escape_html(&finding.affected_state)
                )
            })
            .collect::<Vec<_>>()
            .join("");
        format!("<ul>{items}</ul>")
    };

    let artifacts = packet
        .artifacts
        .iter()
        .map(|artifact| {
            format!(
                "<li><a href=\"{}\">{}</a> <span>{}</span></li>",
                escape_html(&artifact.path),
                escape_html(&artifact.id),
                escape_html(&artifact.hash)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let state_metadata = packet
        .coverage
        .state_metadata
        .iter()
        .map(|state| {
            format!(
                "<li><strong>{}</strong> <span>{}</span><br><code>{}</code><br><span>HTTP status: {}; console errors: {}; network errors: {}; state errors: {}</span></li>",
                escape_html(&state.id),
                escape_html(&state.title),
                escape_html(&state.url),
                state.http_status
                    .map(|status| status.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                state.console_errors.len(),
                state.network_errors.len(),
                state.state_errors.len()
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Allie Evidence Report {run_id}</title>
  <style>
    :root {{ color-scheme: light; --ink: #151719; --muted: #58616c; --line: #d7dde5; --wash: #f5f7fa; --panel: #ffffff; --accent: #1f5eff; }}
    * {{ box-sizing: border-box; }}
    body {{ margin: 0; color: var(--ink); background: var(--wash); font: 16px/1.5 ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
    main {{ width: min(100% - 40px, 980px); margin: 0 auto; padding: 40px 0; }}
    h1 {{ font-size: 44px; line-height: 1.05; margin: 0 0 10px; letter-spacing: 0; }}
    h2 {{ font-size: 13px; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); margin: 0 0 12px; }}
    p {{ margin: 0; }}
    p + p {{ margin-top: 8px; }}
    code {{ font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; background: #edf1f6; padding: 0.08em 0.28em; border-radius: 4px; }}
    section {{ background: var(--panel); border: 1px solid var(--line); padding: 20px; margin-top: 18px; }}
    .summary {{ display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 1px; background: var(--line); border: 1px solid var(--line); margin-top: 22px; }}
    .summary div {{ background: var(--panel); padding: 16px; }}
    .label {{ color: var(--muted); font-size: 13px; text-transform: uppercase; letter-spacing: 0.08em; }}
    .value {{ font-size: 24px; font-weight: 700; margin-top: 4px; }}
    a {{ color: var(--accent); }}
    li + li {{ margin-top: 8px; }}
    @media (max-width: 760px) {{ main {{ width: min(100% - 24px, 980px); }} .summary {{ grid-template-columns: 1fr; }} }}
  </style>
</head>
<body>
  <main>
    <p class="label">Allie evidence status, not a legal compliance guarantee</p>
    <h1>{app_name}</h1>
    <p>Run <code>{run_id}</code> exercised <code>{flow_manifest}</code> with policy profile <code>{policy}</code>.</p>
    <div class="summary" aria-label="Run summary">
      <div><p class="label">Status</p><p class="value">{status}</p></div>
      <div><p class="label">Exit</p><p class="value">{exit_code}</p></div>
      <div><p class="label">States</p><p class="value">{states}</p></div>
      <div><p class="label">Deterministic Failures</p><p class="value">{failures}</p></div>
    </div>
    <section>
      <h2>Replay</h2>
      <p><code>{replay}</code></p>
    </section>
    <section>
      <h2>Captured States</h2>
      <ul>{state_metadata}</ul>
    </section>
    <section>
      <h2>Findings</h2>
      {findings}
    </section>
    <section>
      <h2>Artifacts</h2>
      <ul>{artifacts}</ul>
    </section>
    <section>
      <h2>Residual Review Needs</h2>
      <p>{review_needs}</p>
    </section>
  </main>
</body>
</html>
"#,
        run_id = escape_html(&packet.run.id),
        app_name = escape_html(&packet.target.app_name),
        flow_manifest = escape_html(&packet.target.flow_manifest),
        policy = escape_html(&packet.policy.profile),
        status = escape_html(&packet.summary.status),
        exit_code = packet.summary.exit_code,
        states = packet.summary.states_captured,
        failures = packet.summary.deterministic_failures,
        replay = escape_html(&packet.replay.command),
        state_metadata = state_metadata,
        findings = findings,
        artifacts = artifacts,
        review_needs = escape_html(
            &packet
                .coverage
                .obligations_requiring_human_review
                .join(", ")
        ),
    )
}

fn response_contract_failures(
    manifest: &FlowManifest,
    response: &WorkerResponse,
) -> Vec<ContractFailure> {
    let mut failures = Vec::new();

    for expected in &manifest.flow.states {
        let Some(actual) = response.states.iter().find(|state| state.id == expected.id) else {
            failures.push(ContractFailure {
                state_id: expected.id.clone(),
                route: expected.path.clone(),
                message: format!("required state {} was not captured", expected.id),
            });
            continue;
        };

        if expected.required && expected.axe && actual.axe_json_path.is_none() {
            failures.push(ContractFailure {
                state_id: expected.id.clone(),
                route: expected.path.clone(),
                message: format!(
                    "required state {} did not include raw axe JSON",
                    expected.id
                ),
            });
        }

        if expected.required && expected.screenshot && actual.screenshot_path.is_none() {
            failures.push(ContractFailure {
                state_id: expected.id.clone(),
                route: expected.path.clone(),
                message: format!(
                    "required state {} did not include a screenshot",
                    expected.id
                ),
            });
        }
    }

    failures
}

fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value).map_err(|source| AllieError::Json {
        context: format!("serialize json {}", path.display()),
        source,
    })?;
    write_string(path, &(json + "\n"))
}

fn write_string(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AllieError::Io {
            context: format!("create directory {}", parent.display()),
            source,
        })?;
    }
    fs::write(path, contents).map_err(|source| AllieError::Io {
        context: format!("write {}", path.display()),
        source,
    })
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|source| AllieError::Io {
        context: format!("read artifact {}", path.display()),
        source,
    })?;
    let digest = Sha256::digest(&bytes);
    Ok(hex_lower(&digest))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn path_relative_to(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

fn new_run_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    format!("run-{millis}")
}

fn git_metadata(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn placeholder_cli_points_to_v0_command() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let code = run_cli_with_io(Vec::<String>::new(), &mut stdout, &mut stderr);

        assert_eq!(code, 0);
        let output = String::from_utf8(stdout).unwrap();
        assert!(output.contains("accessibility evidence"));
        assert!(output.contains("allie run --manifest"));
        assert!(stderr.is_empty());
    }

    #[test]
    fn example_manifest_validates_the_checked_in_fixture() {
        let manifest = FlowManifest::load(Path::new("examples/login-flow.yml")).unwrap();

        manifest.validate().unwrap();

        assert_eq!(manifest.id, "login-flow");
        assert_eq!(manifest.policy.profile, "wcag22-aa");
        assert_eq!(manifest.flow.states[0].id, "login-form");
    }

    #[test]
    fn packet_and_report_capture_worker_artifacts_and_replay() {
        let temp = tempdir().unwrap();
        let out_dir = temp.path().join("latest");
        let artifacts_dir = out_dir.join("artifacts");
        fs::create_dir_all(&artifacts_dir).unwrap();
        fs::write(
            artifacts_dir.join("axe-login-form.json"),
            br#"{"violations":[]}"#,
        )
        .unwrap();
        fs::write(artifacts_dir.join("login-form.png"), b"fake-png").unwrap();

        let manifest = FlowManifest::load(Path::new("examples/login-flow.yml")).unwrap();
        let response = passing_worker_response();
        let receipt = write_packet_and_report(
            &manifest,
            Path::new("examples/login-flow.yml"),
            &out_dir,
            response,
            Utc::now(),
            Utc::now(),
            "run-test".to_string(),
        )
        .unwrap();

        assert_eq!(receipt.exit_class, ExitClass::Success);
        assert!(receipt.evidence_path.exists());
        assert!(receipt.report_path.exists());

        let packet = fs::read_to_string(receipt.evidence_path).unwrap();
        assert!(packet.contains("\"schema\": \"allie.evidence.v0\""));
        assert!(packet.contains("sha256:"));
        assert!(packet.contains("\"title\": \"Allie Fixture Login\""));
        assert!(packet.contains("cargo run --locked -- run --manifest examples/login-flow.yml"));

        let report = fs::read_to_string(receipt.report_path).unwrap();
        assert!(report.contains("Allie evidence status"));
        assert!(report.contains("No deterministic axe failures"));
    }

    #[test]
    fn deterministic_axe_violations_return_blocking_exit_class() {
        let mut response = passing_worker_response();
        response.status = WorkerRunStatus::Failed;
        response.states[0].axe_violations.push(AxeViolation {
            id: "color-contrast".to_string(),
            impact: Some("serious".to_string()),
            help: Some("Elements must meet minimum color contrast ratio thresholds".to_string()),
            description: Some("axe reported contrast failure".to_string()),
            tags: vec!["wcag143".to_string()],
            nodes: 1,
        });

        assert_eq!(
            exit_class_for_response(&response, &[]),
            ExitClass::BlockingFinding
        );
    }

    #[test]
    fn missing_required_worker_artifacts_block_success_packets() {
        let temp = tempdir().unwrap();
        let out_dir = temp.path().join("latest");
        let artifacts_dir = out_dir.join("artifacts");
        fs::create_dir_all(&artifacts_dir).unwrap();
        fs::write(
            artifacts_dir.join("axe-login-form.json"),
            br#"{"violations":[]}"#,
        )
        .unwrap();

        let manifest = FlowManifest::load(Path::new("examples/login-flow.yml")).unwrap();
        let mut response = passing_worker_response();
        response.states[0].screenshot_path = None;
        let receipt = write_packet_and_report(
            &manifest,
            Path::new("examples/login-flow.yml"),
            &out_dir,
            response,
            Utc::now(),
            Utc::now(),
            "run-missing-artifact".to_string(),
        )
        .unwrap();

        assert_eq!(receipt.exit_class, ExitClass::BlockingFinding);
        let packet = fs::read_to_string(receipt.evidence_path).unwrap();
        assert!(packet.contains("\"status\": \"fail\""));
        assert!(packet.contains("did not include a screenshot"));
    }

    #[test]
    fn evidence_schema_is_formal_v0_schema() {
        let schema = fs::read_to_string("schemas/allie.evidence.v0.schema.json").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&schema).unwrap();

        assert_eq!(parsed["properties"]["schema"]["const"], "allie.evidence.v0");
        assert!(
            parsed["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "replay")
        );
    }

    fn passing_worker_response() -> WorkerResponse {
        WorkerResponse {
            schema: WORKER_RESPONSE_SCHEMA.to_string(),
            status: WorkerRunStatus::Passed,
            actual_base_url: Some("http://127.0.0.1:49152".to_string()),
            states: vec![WorkerStateResult {
                id: "login-form".to_string(),
                route: "/".to_string(),
                url: "http://127.0.0.1:49152/".to_string(),
                title: "Allie Fixture Login".to_string(),
                http_status: Some(200),
                screenshot_path: Some("artifacts/login-form.png".to_string()),
                axe_json_path: Some("artifacts/axe-login-form.json".to_string()),
                axe_violations: Vec::new(),
                console_errors: Vec::new(),
                network_errors: Vec::new(),
                state_errors: Vec::new(),
            }],
            errors: Vec::new(),
        }
    }
}
