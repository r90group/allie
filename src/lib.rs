use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use wait_timeout::ChildExt;

const PRODUCT_LINE: &str = "Allie: accessibility evidence for every release.";
const NEXT_STEP: &str = "Next implementation target: allie run --manifest <flow.yml>";
const EVIDENCE_SCHEMA: &str = "allie.evidence.v0";
const WORKER_REQUEST_SCHEMA: &str = "allie.worker.request.v0";
const WORKER_RESPONSE_SCHEMA: &str = "allie.worker.response.v0";
const DEFAULT_WORKER_TIMEOUT_MS: u64 = 30_000;
const WCAG22_AA_PROFILE_JSON: &str = include_str!("../profiles/wcag22-aa.json");

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

#[derive(Debug)]
struct ReleaseOptions {
    packet_path: PathBuf,
    out_dir: PathBuf,
    changed_surfaces: Vec<String>,
    stale_after_days: i64,
}

#[derive(Debug)]
struct ReleaseReceipt {
    status: String,
    exit_class: ExitClass,
    summary_path: PathBuf,
    check_path: PathBuf,
    report_path: PathBuf,
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
        Some("release") => match parse_release_options(&args[1..]) {
            Ok(options) => match run_release(options) {
                Ok(receipt) => {
                    let _ = writeln!(
                        stdout,
                        "Release summary: {}",
                        receipt.summary_path.display()
                    );
                    let _ = writeln!(stdout, "GitHub check: {}", receipt.check_path.display());
                    let _ = writeln!(stdout, "Release report: {}", receipt.report_path.display());
                    let _ = writeln!(stdout, "Status: {}", receipt.status);
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
        "Usage:\n  allie run --manifest <flow.yml> --out <output-dir>\n  allie release --packet <evidence.json> --out <output-dir> [--changed-surface <id>] [--stale-after-days <days>]"
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

fn parse_release_options(args: &[String]) -> std::result::Result<ReleaseOptions, String> {
    let mut packet_path = None;
    let mut out_dir = None;
    let mut changed_surfaces = Vec::new();
    let mut stale_after_days = 7;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--packet" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--packet requires a path".to_string())?;
                packet_path = Some(PathBuf::from(value));
            }
            "--out" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--out requires a directory".to_string())?;
                out_dir = Some(PathBuf::from(value));
            }
            "--changed-surface" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--changed-surface requires an id".to_string())?;
                changed_surfaces.push(value.to_string());
            }
            "--stale-after-days" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--stale-after-days requires a number".to_string())?;
                stale_after_days = value
                    .parse::<i64>()
                    .map_err(|_| "--stale-after-days must be an integer".to_string())?;
            }
            unexpected => return Err(format!("unexpected argument: {unexpected}")),
        }
        index += 1;
    }

    Ok(ReleaseOptions {
        packet_path: packet_path.ok_or_else(|| "--packet is required".to_string())?,
        out_dir: out_dir.ok_or_else(|| "--out is required".to_string())?,
        changed_surfaces,
        stale_after_days,
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
    let mut run_failures = manifest.preflight_failures();
    let response = if run_failures.is_empty() {
        let request = WorkerRequest::from_manifest(
            &run_id,
            &manifest,
            &options.manifest_path,
            &options.out_dir.join("artifacts"),
        )?;
        write_json_pretty(&request_path, &request)?;

        match invoke_worker(
            &request_path,
            &response_path,
            manifest.policy.worker_timeout_ms,
        ) {
            Ok(()) => read_worker_response(&response_path),
            Err(failure) => {
                let message = failure.message.clone();
                run_failures.push(failure);
                Ok(WorkerResponse::error(message))
            }
        }?
    } else {
        WorkerResponse::error(
            run_failures
                .iter()
                .map(|failure| failure.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        )
    };
    response.validate()?;

    write_packet_and_report(
        &manifest,
        &options.manifest_path,
        &options.out_dir,
        response,
        run_failures,
        started_at,
        now_utc(),
        run_id,
    )
}

fn run_release(options: ReleaseOptions) -> Result<ReleaseReceipt> {
    fs::create_dir_all(&options.out_dir).map_err(|source| AllieError::Io {
        context: format!(
            "create release output directory {}",
            options.out_dir.display()
        ),
        source,
    })?;

    let packet = read_release_packet(&options.packet_path)?;

    let projection = project_release_decision(&packet, &options);
    let summary_path = options.out_dir.join("release-summary.json");
    let check_path = options.out_dir.join("github-check.json");
    let report_path = options.out_dir.join("release-report.html");
    write_json_pretty(&summary_path, &projection.summary)?;
    write_json_pretty(&check_path, &projection.github_check)?;
    write_string(&report_path, &render_release_report(&projection.summary))?;

    Ok(ReleaseReceipt {
        status: projection.summary["status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        exit_class: projection.exit_class,
        summary_path,
        check_path,
        report_path,
    })
}

struct ReleaseProjection {
    summary: serde_json::Value,
    github_check: serde_json::Value,
    exit_class: ExitClass,
}

fn project_release_decision(
    packet: &serde_json::Value,
    options: &ReleaseOptions,
) -> ReleaseProjection {
    let deterministic_failures = packet["summary"]["deterministic_failures"]
        .as_u64()
        .unwrap_or_default();
    let scripted_failures = packet["summary"]["scripted_failures"]
        .as_u64()
        .unwrap_or_default();
    let infrastructure_failures = packet["summary"]["infrastructure_failures"]
        .as_u64()
        .unwrap_or_default();
    let packet_status = packet["summary"]["status"].as_str().unwrap_or("error");
    let evidence_artifacts = packet["artifacts"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|artifact| artifact["type"].as_str())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let verdicts = packet["verdicts"].as_array().cloned().unwrap_or_default();
    let review_needed = verdicts
        .iter()
        .filter(|verdict| verdict["status"].as_str() == Some("needs_review"))
        .filter_map(|verdict| verdict["obligation"].as_str())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let not_tested = verdicts
        .iter()
        .filter(|verdict| verdict["status"].as_str() == Some("not_tested"))
        .filter_map(|verdict| verdict["obligation"].as_str())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let model_findings_non_blocking = packet["findings"]
        .as_array()
        .map(|findings| {
            findings
                .iter()
                .filter(|finding| finding["evidence_class"].as_str() == Some("agentic"))
                .count()
        })
        .unwrap_or_default();

    let captured_states = string_set_at(&packet["coverage"]["states_captured"]);
    let discovered_surfaces = string_set_at(&packet["coverage"]["surfaces_discovered"]);
    let missing_required_evidence = options
        .changed_surfaces
        .iter()
        .filter(|surface| {
            !captured_states.contains(surface.as_str())
                && !discovered_surfaces.contains(surface.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();

    let stale_evidence = packet_is_stale(packet, options.stale_after_days);
    let expired_waivers = expired_touched_waivers(packet, &options.changed_surfaces);
    let invalid_waivers = invalid_touched_waivers(packet, &options.changed_surfaces);
    let has_blocker = packet_status == "fail"
        || packet_status == "error"
        || deterministic_failures > 0
        || scripted_failures > 0
        || infrastructure_failures > 0
        || !missing_required_evidence.is_empty()
        || !expired_waivers.is_empty()
        || !invalid_waivers.is_empty();
    let status = if has_blocker {
        "blocked"
    } else if stale_evidence
        || !review_needed.is_empty()
        || !not_tested.is_empty()
        || model_findings_non_blocking > 0
    {
        "needs_review"
    } else {
        "approved"
    };
    let conclusion = if has_blocker {
        "failure"
    } else if status == "needs_review" {
        "neutral"
    } else {
        "success"
    };
    let exit_class = if has_blocker {
        ExitClass::BlockingFinding
    } else {
        ExitClass::Success
    };

    let summary = serde_json::json!({
        "schema": "allie.release-decision.v0",
        "status": status,
        "packet_path": options.packet_path.to_string_lossy(),
        "packet_run_id": packet["run"]["id"].as_str().unwrap_or("unknown"),
        "changed_surfaces": options.changed_surfaces,
        "blocking": {
            "deterministic_failures": deterministic_failures,
            "scripted_failures": scripted_failures,
            "infrastructure_failures": infrastructure_failures,
            "missing_required_evidence": missing_required_evidence,
            "expired_waivers": expired_waivers,
            "invalid_waivers": invalid_waivers
        },
        "review": {
            "stale_evidence": stale_evidence
        },
        "review_needed_obligations": review_needed,
        "not_tested_obligations": not_tested,
        "model_findings_non_blocking": model_findings_non_blocking,
        "evidence_artifacts": evidence_artifacts,
        "policy": {
            "model_status": packet["policy"]["model_status"].clone(),
            "model_provider_allowlist": packet["policy"]["model_provider_allowlist"].clone(),
            "zdr_required": packet["policy"]["zdr_required"].clone()
        }
    });
    let summary_text = release_summary_text(&summary);
    let github_check = serde_json::json!({
        "name": "Allie accessibility evidence",
        "conclusion": conclusion,
        "output": {
            "title": format!("Allie release decision: {status}"),
            "summary": summary_text,
            "text": summary_text
        }
    });

    ReleaseProjection {
        summary,
        github_check,
        exit_class,
    }
}

fn string_set_at(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn packet_is_stale(packet: &serde_json::Value, stale_after_days: i64) -> bool {
    let Some(finished_at) = packet["run"]["finished_at"].as_str() else {
        return true;
    };
    let Ok(finished_at) = DateTime::parse_from_rfc3339(finished_at) else {
        return true;
    };
    let age = Utc::now().signed_duration_since(finished_at.with_timezone(&Utc));
    age.num_days() > stale_after_days
}

fn expired_touched_waivers(
    packet: &serde_json::Value,
    changed_surfaces: &[String],
) -> Vec<serde_json::Value> {
    let changed = changed_surfaces.iter().cloned().collect::<BTreeSet<_>>();
    packet["waivers"]
        .as_array()
        .map(|waivers| {
            waivers
                .iter()
                .filter(|waiver| waiver_is_expired_for_changed_surface(waiver, &changed))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn waiver_is_expired_for_changed_surface(
    waiver: &serde_json::Value,
    changed_surfaces: &BTreeSet<String>,
) -> bool {
    if !waiver_touches_changed_surface(waiver, changed_surfaces) {
        return false;
    }
    let Some(expires_at) = waiver["expires_at"].as_str() else {
        return false;
    };
    let Ok(expires_at) = DateTime::parse_from_rfc3339(expires_at) else {
        return true;
    };
    if expires_at.with_timezone(&Utc) >= Utc::now() {
        return false;
    }
    true
}

fn invalid_touched_waivers(
    packet: &serde_json::Value,
    changed_surfaces: &[String],
) -> Vec<serde_json::Value> {
    let changed = changed_surfaces.iter().cloned().collect::<BTreeSet<_>>();
    packet["waivers"]
        .as_array()
        .map(|waivers| {
            waivers
                .iter()
                .filter(|waiver| {
                    waiver_touches_changed_surface(waiver, &changed)
                        && !waiver_has_required_release_metadata(waiver)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn waiver_touches_changed_surface(
    waiver: &serde_json::Value,
    changed_surfaces: &BTreeSet<String>,
) -> bool {
    if changed_surfaces.is_empty() {
        return true;
    }
    let Some(surface) = waiver["surface"].as_str() else {
        return true;
    };
    surface.trim().is_empty() || changed_surfaces.contains(surface)
}

fn waiver_has_required_release_metadata(waiver: &serde_json::Value) -> bool {
    let Some(surface) = waiver["surface"].as_str() else {
        return false;
    };
    if surface.trim().is_empty() {
        return false;
    }
    let Some(status) = waiver["status"].as_str() else {
        return false;
    };
    if !matches!(status, "waived" | "risk_accepted") {
        return false;
    }
    let Some(expires_at) = waiver["expires_at"].as_str() else {
        return false;
    };
    if DateTime::parse_from_rfc3339(expires_at).is_err() {
        return false;
    }
    let provenance_ok = waiver["provenance"]
        .as_str()
        .map(|value| !value.trim().is_empty())
        .or_else(|| {
            waiver["provenance"]
                .as_object()
                .map(|value| !value.is_empty())
        })
        .unwrap_or(false);
    let packet_ref_ok = waiver["packet_ref"]
        .as_str()
        .map(|value| !value.trim().is_empty())
        .or_else(|| {
            waiver["packet_refs"].as_array().map(|values| {
                values
                    .iter()
                    .any(|value| value.as_str().is_some_and(|item| !item.trim().is_empty()))
            })
        })
        .unwrap_or(false);

    provenance_ok && packet_ref_ok
}

fn release_summary_text(summary: &serde_json::Value) -> String {
    format!(
        "status={} deterministic_failures={} scripted_failures={} infrastructure_failures={} review_needed={} not_tested={}",
        summary["status"].as_str().unwrap_or("unknown"),
        summary["blocking"]["deterministic_failures"]
            .as_u64()
            .unwrap_or_default(),
        summary["blocking"]["scripted_failures"]
            .as_u64()
            .unwrap_or_default(),
        summary["blocking"]["infrastructure_failures"]
            .as_u64()
            .unwrap_or_default(),
        summary["review_needed_obligations"]
            .as_array()
            .map(|items| items.len())
            .unwrap_or_default(),
        summary["not_tested_obligations"]
            .as_array()
            .map(|items| items.len())
            .unwrap_or_default()
    )
}

fn render_release_report(summary: &serde_json::Value) -> String {
    let text = escape_html(&release_summary_text(summary));
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Allie Release Decision</title>
  <style>
    body {{ margin: 0; font: 16px/1.5 ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; color: #151719; background: #f5f7fa; }}
    main {{ width: min(100% - 40px, 900px); margin: 0 auto; padding: 40px 0; }}
    section {{ background: #fff; border: 1px solid #d7dde5; padding: 20px; margin-top: 18px; }}
    h1 {{ margin: 0; font-size: 42px; line-height: 1.05; letter-spacing: 0; }}
    h2 {{ margin: 0 0 10px; font-size: 13px; text-transform: uppercase; letter-spacing: 0.08em; color: #58616c; }}
  </style>
</head>
<body>
  <main>
    <h1>Allie release decision: {status}</h1>
    <section>
      <h2>Evidence Projection</h2>
      <p>{text}</p>
      <p>This is a projection of evidence packets, not a legal compliance guarantee and not a global score.</p>
    </section>
  </main>
</body>
</html>
"#,
        status = escape_html(summary["status"].as_str().unwrap_or("unknown")),
        text = text
    )
}

fn read_release_packet(packet_path: &Path) -> Result<serde_json::Value> {
    let packet_text = fs::read_to_string(packet_path).map_err(|source| AllieError::Io {
        context: format!("read evidence packet {}", packet_path.display()),
        source,
    })?;
    let packet = serde_json::from_str::<EvidencePacket>(&packet_text).map_err(|source| {
        AllieError::Json {
            context: format!("parse evidence packet {}", packet_path.display()),
            source,
        }
    })?;
    validate_release_packet(&packet)?;
    serde_json::to_value(packet).map_err(|source| AllieError::Json {
        context: format!("normalize evidence packet {}", packet_path.display()),
        source,
    })
}

fn validate_release_packet(packet: &EvidencePacket) -> Result<()> {
    if packet.schema != EVIDENCE_SCHEMA {
        return Err(AllieError::InvalidManifest(format!(
            "invalid evidence packet schema {}; expected {EVIDENCE_SCHEMA}",
            packet.schema
        )));
    }

    if !matches!(packet.summary.status.as_str(), "pass" | "fail" | "error") {
        return Err(AllieError::InvalidManifest(format!(
            "invalid evidence packet status {}; expected pass, fail, or error",
            packet.summary.status
        )));
    }

    Ok(())
}

fn read_worker_response(response_path: &Path) -> Result<WorkerResponse> {
    let response_text = match fs::read_to_string(response_path) {
        Ok(text) => text,
        Err(source) => {
            return Ok(WorkerResponse::error(format!(
                "worker partial-write: read response {}: {source}",
                response_path.display()
            )));
        }
    };

    match serde_json::from_str::<WorkerResponse>(&response_text) {
        Ok(response) => Ok(response),
        Err(source) => Ok(WorkerResponse::error(format!(
            "worker partial-write: parse response {}: {source}",
            response_path.display()
        ))),
    }
}

fn invoke_worker(
    request_path: &Path,
    response_path: &Path,
    timeout_ms: u64,
) -> std::result::Result<(), RunFailure> {
    let worker_script = std::env::var_os("ALLIE_BROWSER_WORKER")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("workers/browser/run.mjs")
        });

    if !worker_script.exists() {
        return Err(RunFailure::new(
            "worker-missing",
            "worker-adapter",
            format!("worker script not found at {}", worker_script.display()),
        ));
    }

    let mut child = Command::new("node")
        .arg(&worker_script)
        .arg("--request")
        .arg(request_path)
        .arg("--response")
        .arg(response_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| {
            RunFailure::new(
                "worker-spawn-failed",
                "worker-adapter",
                format!("spawn worker {}: {source}", worker_script.display()),
            )
        })?;

    match child
        .wait_timeout(Duration::from_millis(timeout_ms))
        .map_err(|source| {
            RunFailure::new(
                "worker-wait-failed",
                "worker-adapter",
                format!("wait for worker {}: {source}", worker_script.display()),
            )
        })? {
        Some(status) => {
            let output = child.wait_with_output().map_err(|source| {
                RunFailure::new(
                    "worker-output-failed",
                    "worker-adapter",
                    format!(
                        "collect worker output {}: {source}",
                        worker_script.display()
                    ),
                )
            })?;
            if !status.success() {
                return Err(RunFailure::new(
                    "worker-crash",
                    "worker-adapter",
                    format!(
                        "{}\n{}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    ),
                ));
            }
        }
        None => {
            let _ = child.kill();
            let output = child.wait_with_output().map_err(|source| {
                RunFailure::new(
                    "worker-timeout",
                    "worker-adapter",
                    format!("worker timed out after {timeout_ms} ms and output collection failed: {source}"),
                )
            })?;
            return Err(RunFailure::new(
                "worker-timeout",
                "worker-adapter",
                format!(
                    "worker timed out after {timeout_ms} ms\n{}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }
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
    #[serde(default)]
    credentials: CredentialConfig,
    target: ManifestTarget,
    policy: ManifestPolicy,
    #[serde(default)]
    artifacts: ArtifactPolicy,
    #[serde(default)]
    model: ModelPolicy,
    #[serde(default)]
    known_nondeterminism: Vec<String>,
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
        require_name("credential profile", &self.auth_profile_name())?;
        self.credentials.validate()?;
        self.model.validate()?;
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

    fn auth_profile_name(&self) -> String {
        self.credentials
            .profile
            .clone()
            .or_else(|| self.auth_profile.clone())
            .unwrap_or_else(|| "none".to_string())
    }

    fn credential_metadata(&self) -> CredentialProviderMetadata {
        let status = if self.credentials.provider == "env" {
            match self.credentials.env.as_deref() {
                Some(env_name) if std::env::var_os(env_name).is_some() => "available",
                Some(_) if self.credentials.required => "missing",
                Some(_) => "not_required",
                None => "misconfigured",
            }
        } else {
            "not_required"
        };

        CredentialProviderMetadata {
            provider: self.credentials.provider.clone(),
            env: self.credentials.env.clone(),
            required: self.credentials.required,
            status: status.to_string(),
        }
    }

    fn preflight_failures(&self) -> Vec<RunFailure> {
        let mut failures = Vec::new();

        if self.credentials.provider == "env" {
            match self.credentials.env.as_deref() {
                Some(env_name)
                    if self.credentials.required && std::env::var_os(env_name).is_none() =>
                {
                    failures.push(RunFailure::new(
                        "missing-credential",
                        "credential-provider",
                        format!(
                            "credential profile {} requires env {} but it is not set",
                            self.auth_profile_name(),
                            env_name
                        ),
                    ));
                }
                Some(_) | None => {}
            }
        }

        if self.model.enabled && self.model.provider_allowlist.is_empty() {
            failures.push(RunFailure::new(
                "model-policy-incomplete",
                "model-policy",
                "model calls are enabled but provider_allowlist is empty".to_string(),
            ));
        }

        failures
    }
}

fn require_name(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(AllieError::InvalidManifest(format!("{label} is required")));
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CredentialConfig {
    profile: Option<String>,
    provider: String,
    env: Option<String>,
    required: bool,
}

impl Default for CredentialConfig {
    fn default() -> Self {
        Self {
            profile: None,
            provider: "none".to_string(),
            env: None,
            required: false,
        }
    }
}

impl CredentialConfig {
    fn validate(&self) -> Result<()> {
        match self.provider.as_str() {
            "none" => Ok(()),
            "env" => {
                if self.env.as_deref().unwrap_or_default().trim().is_empty() {
                    return Err(AllieError::InvalidManifest(
                        "env credential provider requires credentials.env".to_string(),
                    ));
                }
                Ok(())
            }
            provider => Err(AllieError::InvalidManifest(format!(
                "unsupported credential provider {provider}"
            ))),
        }
    }
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
    #[serde(default = "default_worker_timeout_ms")]
    worker_timeout_ms: u64,
}

fn default_worker_timeout_ms() -> u64 {
    DEFAULT_WORKER_TIMEOUT_MS
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ArtifactPolicy {
    redaction_status: String,
    retention_class: String,
}

impl Default for ArtifactPolicy {
    fn default() -> Self {
        Self {
            redaction_status: "not_redacted_local_fixture".to_string(),
            retention_class: "local_ephemeral".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ModelPolicy {
    enabled: bool,
    provider_allowlist: Vec<String>,
    zdr_required: bool,
}

impl Default for ModelPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            provider_allowlist: Vec::new(),
            zdr_required: true,
        }
    }
}

impl ModelPolicy {
    fn validate(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        Ok(())
    }
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
    #[serde(default)]
    nondeterminism: Vec<String>,
}

impl WorkerResponse {
    fn validate(&self) -> Result<()> {
        if self.schema != WORKER_RESPONSE_SCHEMA {
            return Err(AllieError::Worker(format!(
                "unexpected worker response schema {}",
                self.schema
            )));
        }
        Ok(())
    }
}

impl WorkerResponse {
    fn error(message: String) -> Self {
        Self {
            schema: WORKER_RESPONSE_SCHEMA.to_string(),
            status: WorkerRunStatus::Error,
            actual_base_url: None,
            states: Vec::new(),
            errors: vec![message],
            nondeterminism: Vec::new(),
        }
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
#[serde(deny_unknown_fields)]
struct EvidencePacket {
    schema: String,
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
#[serde(deny_unknown_fields)]
struct PacketSummary {
    status: String,
    exit_code: i32,
    deterministic_failures: usize,
    scripted_failures: usize,
    infrastructure_failures: usize,
    states_captured: usize,
    failure_class: Option<String>,
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
    credential_provider: CredentialProviderMetadata,
    flow_manifest: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CredentialProviderMetadata {
    provider: String,
    env: Option<String>,
    required: bool,
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PolicyMetadata {
    profile: String,
    blocking_classes: Vec<String>,
    worker_timeout_ms: u64,
    model_provider_allowlist: Vec<String>,
    model_status: String,
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

#[derive(Clone, Debug)]
struct RunFailure {
    kind: String,
    source: String,
    message: String,
}

impl RunFailure {
    fn new(kind: &str, source: &str, message: String) -> Self {
        Self {
            kind: kind.to_string(),
            source: source.to_string(),
            message,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactMetadata {
    id: String,
    #[serde(rename = "type")]
    artifact_type: String,
    path: String,
    hash: String,
    redaction_status: String,
    retention_class: String,
    unavailable_reason: Option<String>,
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
#[serde(deny_unknown_fields)]
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
    run_failures: Vec<RunFailure>,
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
    let contract_failures = if matches!(response.status, WorkerRunStatus::Error) {
        Vec::new()
    } else {
        response_contract_failures(manifest, &response)
    };
    let exit_class = exit_class_for_response(&response, &contract_failures, &run_failures);
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
    let response_error_count = if run_failures.is_empty() {
        response.errors.len()
    } else {
        0
    };
    let infrastructure_failures =
        run_failures.len() + response_error_count + response.nondeterminism.len();

    let mut artifacts = worker_artifacts(out_dir, &response, &manifest.artifacts, finished_at)?;
    let findings = findings_from_response(
        &response,
        &artifacts,
        &contract_failures,
        &run_failures,
        &manifest.policy.profile,
        &replay_command,
    );
    let verdicts = verdicts_from_findings(manifest, &response, &findings);
    let failure_class = failure_class_for(exit_class, &response, &contract_failures, &run_failures);
    let mut packet = EvidencePacket {
        schema: EVIDENCE_SCHEMA.to_string(),
        summary: PacketSummary {
            status: exit_class.packet_status().to_string(),
            exit_code: exit_class.code(),
            deterministic_failures,
            scripted_failures,
            infrastructure_failures,
            states_captured: response.states.len(),
            failure_class,
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
            auth_profile: manifest.auth_profile_name(),
            credential_provider: manifest.credential_metadata(),
            flow_manifest: manifest_path.to_string_lossy().to_string(),
        },
        policy: PolicyMetadata {
            profile: manifest.policy.profile.clone(),
            blocking_classes: manifest.policy.blocking_classes.clone(),
            worker_timeout_ms: manifest.policy.worker_timeout_ms,
            model_provider_allowlist: manifest.model.provider_allowlist.clone(),
            model_status: if manifest.model.enabled {
                "enabled".to_string()
            } else {
                "disabled".to_string()
            },
            zdr_required: manifest.model.zdr_required,
            redaction_profile: manifest.artifacts.redaction_status.clone(),
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
            credential_profile: manifest.auth_profile_name(),
            browser: manifest.browser.clone(),
            seed_data: vec!["checked-in fixture fixtures/login".to_string()],
            known_nondeterminism: manifest.known_nondeterminism.clone(),
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
        &manifest.artifacts,
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
    run_failures: &[RunFailure],
) -> ExitClass {
    if matches!(response.status, WorkerRunStatus::Error)
        || !run_failures.is_empty()
        || !response.errors.is_empty()
        || !response.nondeterminism.is_empty()
    {
        ExitClass::InfrastructureFailure
    } else if response
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

fn failure_class_for(
    exit_class: ExitClass,
    response: &WorkerResponse,
    contract_failures: &[ContractFailure],
    run_failures: &[RunFailure],
) -> Option<String> {
    if let Some(failure) = run_failures.first() {
        return Some(failure.kind.clone());
    }
    if !response.nondeterminism.is_empty() {
        return Some("nondeterminism".to_string());
    }
    if matches!(response.status, WorkerRunStatus::Error) || !response.errors.is_empty() {
        return Some("worker-error".to_string());
    }
    if !contract_failures.is_empty() {
        return Some("required-evidence-missing".to_string());
    }
    match exit_class {
        ExitClass::BlockingFinding => Some("blocking-finding".to_string()),
        ExitClass::InfrastructureFailure => Some("infrastructure-failure".to_string()),
        ExitClass::Success | ExitClass::Usage => None,
    }
}

fn worker_artifacts(
    out_dir: &Path,
    response: &WorkerResponse,
    artifact_policy: &ArtifactPolicy,
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
                artifact_policy,
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
                artifact_policy,
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
    artifact_policy: &ArtifactPolicy,
    timestamp: DateTime<Utc>,
) -> Result<ArtifactMetadata> {
    Ok(ArtifactMetadata {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        path: path_relative_to(out_dir, path),
        hash: format!("sha256:{}", sha256_file(path)?),
        redaction_status: artifact_policy.redaction_status.clone(),
        retention_class: artifact_policy.retention_class.clone(),
        unavailable_reason: None,
        related_flow_state,
        creation_tool: creation_tool.to_string(),
        timestamp: timestamp.to_rfc3339(),
    })
}

fn findings_from_response(
    response: &WorkerResponse,
    artifacts: &[ArtifactMetadata],
    contract_failures: &[ContractFailure],
    run_failures: &[RunFailure],
    policy_profile: &str,
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
                        standard_obligation: obligation_from_tags(policy_profile, &violation.tags),
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

    for (index, failure) in run_failures.iter().enumerate() {
        findings.push(Finding {
            id: format!("{}-{}", failure.kind, index + 1),
            title: "Run preflight failed".to_string(),
            description: failure.message.clone(),
            evidence_class: "infrastructure".to_string(),
            standard_obligation: failure.kind.clone(),
            severity: "blocking".to_string(),
            status: "fail".to_string(),
            confidence: "script_observed".to_string(),
            source: failure.source.clone(),
            affected_route: "run".to_string(),
            affected_state: "run".to_string(),
            artifact_refs: Vec::new(),
            suggested_remediation:
                "Fix the run configuration or environment, then rerun the replay command."
                    .to_string(),
            replay_command: replay_command.to_string(),
        });
    }

    if run_failures.is_empty() {
        for (index, message) in response.errors.iter().enumerate() {
            findings.push(Finding {
                id: format!("worker-error-{}", index + 1),
                title: "Worker failed before producing complete evidence".to_string(),
                description: message.clone(),
                evidence_class: "infrastructure".to_string(),
                standard_obligation: "worker-error".to_string(),
                severity: "blocking".to_string(),
                status: "fail".to_string(),
                confidence: "script_observed".to_string(),
                source: "browser-worker".to_string(),
                affected_route: "run".to_string(),
                affected_state: "run".to_string(),
                artifact_refs: Vec::new(),
                suggested_remediation:
                    "Inspect worker-request.json and worker stderr, then rerun the replay command."
                        .to_string(),
                replay_command: replay_command.to_string(),
            });
        }
    }

    for (index, message) in response.nondeterminism.iter().enumerate() {
        findings.push(Finding {
            id: format!("nondeterminism-{}", index + 1),
            title: "Run was marked nondeterministic".to_string(),
            description: message.clone(),
            evidence_class: "infrastructure".to_string(),
            standard_obligation: "nondeterminism".to_string(),
            severity: "blocking".to_string(),
            status: "fail".to_string(),
            confidence: "script_observed".to_string(),
            source: "browser-worker".to_string(),
            affected_route: "run".to_string(),
            affected_state: "run".to_string(),
            artifact_refs: Vec::new(),
            suggested_remediation:
                "Stabilize the fixture or mark known nondeterminism in the manifest before release use."
                    .to_string(),
            replay_command: replay_command.to_string(),
        });
    }

    findings
}

fn obligation_from_tags(policy_profile: &str, tags: &[String]) -> String {
    if policy_profile != "wcag22-aa" {
        return tags
            .iter()
            .find(|tag| tag.starts_with("wcag"))
            .cloned()
            .unwrap_or_else(|| format!("{policy_profile}:unmapped-axe-rule"));
    }

    let profile = wcag22_profile();
    let Some(map) = profile
        .get("axe_tag_map")
        .and_then(|value| value.as_object())
    else {
        return "wcag22-aa:unmapped-axe-rule".to_string();
    };

    let mut candidates = tags.iter().collect::<Vec<_>>();
    candidates.sort_by_key(|tag| std::cmp::Reverse(tag.len()));
    for tag in candidates {
        if let Some(obligation) = map
            .get(tag)
            .and_then(|value| value.get("obligation"))
            .and_then(|value| value.as_str())
        {
            return obligation.to_string();
        }
    }

    "wcag22-aa:unmapped-axe-rule".to_string()
}

fn deterministic_pass_obligation(policy_profile: &str) -> String {
    if policy_profile != "wcag22-aa" {
        return format!("{policy_profile}:deterministic-machine-checks");
    }

    wcag22_profile()
        .get("deterministic_pass_obligation")
        .and_then(|value| value.get("obligation"))
        .and_then(|value| value.as_str())
        .unwrap_or("wcag22-aa:deterministic-axe-rules")
        .to_string()
}

fn scripted_profile_obligations(policy_profile: &str) -> Vec<String> {
    profile_obligation_list(policy_profile, "scripted_obligations")
}

fn human_review_profile_obligations(policy_profile: &str) -> Vec<String> {
    profile_obligation_list(policy_profile, "human_review_obligations")
}

fn profile_obligation_list(policy_profile: &str, key: &str) -> Vec<String> {
    if policy_profile != "wcag22-aa" {
        return Vec::new();
    }

    wcag22_profile()
        .get(key)
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("obligation").and_then(|value| value.as_str()))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn wcag22_profile() -> serde_json::Value {
    serde_json::from_str(WCAG22_AA_PROFILE_JSON).expect("embedded wcag22-aa profile is valid JSON")
}

fn verdicts_from_findings(
    manifest: &FlowManifest,
    response: &WorkerResponse,
    findings: &[Finding],
) -> Vec<Verdict> {
    let mut verdicts = Vec::new();

    if findings.is_empty() {
        verdicts.push(Verdict {
            obligation: deterministic_pass_obligation(&manifest.policy.profile),
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
        });
    } else {
        verdicts.extend(findings.iter().map(|finding| Verdict {
            obligation: finding.standard_obligation.clone(),
            status: "fail".to_string(),
            confidence: finding.confidence.clone(),
            evidence_class: finding.evidence_class.clone(),
            source: finding.source.clone(),
            affected_states: vec![finding.affected_state.clone()],
            finding_refs: vec![finding.id.clone()],
        }));
    }

    let captured_states = response
        .states
        .iter()
        .map(|state| state.id.clone())
        .collect::<Vec<_>>();

    verdicts.extend(
        scripted_profile_obligations(&manifest.policy.profile)
            .into_iter()
            .map(|obligation| Verdict {
                obligation,
                status: "not_tested".to_string(),
                confidence: "script_observed".to_string(),
                evidence_class: "scripted".to_string(),
                source: "allie-obligation-profile".to_string(),
                affected_states: captured_states.clone(),
                finding_refs: Vec::new(),
            }),
    );

    verdicts.extend(
        human_review_profile_obligations(&manifest.policy.profile)
            .into_iter()
            .map(|obligation| Verdict {
                obligation,
                status: "needs_review".to_string(),
                confidence: "script_observed".to_string(),
                evidence_class: "human".to_string(),
                source: "allie-obligation-profile".to_string(),
                affected_states: captured_states.clone(),
                finding_refs: Vec::new(),
            }),
    );

    verdicts
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
        obligations.insert(deterministic_pass_obligation(&manifest.policy.profile));
    } else {
        for finding in findings {
            obligations.insert(finding.standard_obligation.clone());
        }
    }

    let not_tested = scripted_profile_obligations(&manifest.policy.profile);
    let needs_review = human_review_profile_obligations(&manifest.policy.profile);
    for obligation in not_tested.iter().chain(needs_review.iter()) {
        obligations.insert(obligation.clone());
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
        obligations_not_tested: not_tested,
        obligations_requiring_human_review: needs_review,
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
                "<li><a href=\"{}\">{}</a> <span>{}</span><br><span>redaction: {}; retention: {}; unavailable: {}</span></li>",
                escape_html(&artifact.path),
                escape_html(&artifact.id),
                escape_html(&artifact.hash),
                escape_html(&artifact.redaction_status),
                escape_html(&artifact.retention_class),
                escape_html(artifact.unavailable_reason.as_deref().unwrap_or("none"))
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

    let verdicts = packet
        .verdicts
        .iter()
        .map(|verdict| {
            format!(
                "<li><strong>{}</strong> <span>{}</span><br><span>confidence: {}; evidence: {}</span></li>",
                escape_html(&verdict.obligation),
                escape_html(&verdict.status),
                escape_html(&verdict.confidence),
                escape_html(&verdict.evidence_class)
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
      <h2>Verdicts</h2>
      <ul>{verdicts}</ul>
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
        verdicts = verdicts,
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
            Vec::new(),
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
        assert!(packet.contains("\"retention_class\": \"local_ephemeral\""));
        assert!(packet.contains("\"infrastructure_failures\": 0"));
        assert!(packet.contains("\"title\": \"Allie Fixture Login\""));
        assert!(packet.contains("wcag22-aa:deterministic-axe-rules"));
        assert!(packet.contains("\"status\": \"not_tested\""));
        assert!(packet.contains("\"status\": \"needs_review\""));
        assert!(packet.contains("cargo run --locked -- run --manifest examples/login-flow.yml"));

        let report = fs::read_to_string(receipt.report_path).unwrap();
        assert!(report.contains("Allie evidence status"));
        assert!(report.contains("No deterministic axe failures"));
        assert!(report.contains("wcag22-aa:2.1.1-keyboard-traversal"));
        assert!(!report.to_lowercase().contains("compliance score"));
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
            exit_class_for_response(&response, &[], &[]),
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
            Vec::new(),
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
    fn missing_required_credentials_write_error_packet_without_secret_values() {
        let temp = tempdir().unwrap();
        let out_dir = temp.path().join("latest");
        let mut manifest = FlowManifest::load(Path::new("examples/login-flow.yml")).unwrap();
        manifest.credentials = CredentialConfig {
            profile: Some("staging-secret-profile".to_string()),
            provider: "env".to_string(),
            env: Some("ALLIE_TEST_DO_NOT_SET_SECRET".to_string()),
            required: true,
        };
        let failures = manifest.preflight_failures();
        let response = WorkerResponse::error(
            failures
                .iter()
                .map(|failure| failure.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        );

        let receipt = write_packet_and_report(
            &manifest,
            Path::new("examples/login-flow.yml"),
            &out_dir,
            response,
            failures,
            Utc::now(),
            Utc::now(),
            "run-missing-credential".to_string(),
        )
        .unwrap();

        assert_eq!(receipt.exit_class, ExitClass::InfrastructureFailure);
        let packet = fs::read_to_string(receipt.evidence_path).unwrap();
        assert!(packet.contains("\"failure_class\": \"missing-credential\""));
        assert!(packet.contains("\"auth_profile\": \"staging-secret-profile\""));
        assert!(packet.contains("\"env\": \"ALLIE_TEST_DO_NOT_SET_SECRET\""));
        assert!(!packet.contains("\"standard_obligation\": \"worker-error\""));
        assert!(!packet.contains("super-secret-value"));

        let report = fs::read_to_string(receipt.report_path).unwrap();
        assert!(report.contains("Run preflight failed"));
        assert!(!report.contains("super-secret-value"));
    }

    #[test]
    fn model_policy_enabled_without_allowlist_fails_closed() {
        let temp = tempdir().unwrap();
        let out_dir = temp.path().join("latest");
        let mut manifest = FlowManifest::load(Path::new("examples/login-flow.yml")).unwrap();
        manifest.model.enabled = true;
        manifest.model.provider_allowlist = Vec::new();
        let failures = manifest.preflight_failures();
        let response = WorkerResponse::error(
            failures
                .iter()
                .map(|failure| failure.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        );

        let receipt = write_packet_and_report(
            &manifest,
            Path::new("examples/login-flow.yml"),
            &out_dir,
            response,
            failures,
            Utc::now(),
            Utc::now(),
            "run-model-policy".to_string(),
        )
        .unwrap();

        assert_eq!(receipt.exit_class, ExitClass::InfrastructureFailure);
        let packet = fs::read_to_string(receipt.evidence_path).unwrap();
        assert!(packet.contains("\"failure_class\": \"model-policy-incomplete\""));
        assert!(packet.contains("\"model_status\": \"enabled\""));
        assert!(!packet.contains("\"standard_obligation\": \"worker-error\""));
    }

    #[test]
    fn worker_error_and_partial_write_responses_map_to_infrastructure_failure() {
        let temp = tempdir().unwrap();
        let out_dir = temp.path().join("latest");
        let manifest = FlowManifest::load(Path::new("examples/login-flow.yml")).unwrap();
        let response = WorkerResponse::error(
            "worker partial-write: parse response .allie/run/worker-response.json".to_string(),
        );

        let receipt = write_packet_and_report(
            &manifest,
            Path::new("examples/login-flow.yml"),
            &out_dir,
            response,
            Vec::new(),
            Utc::now(),
            Utc::now(),
            "run-partial-write".to_string(),
        )
        .unwrap();

        assert_eq!(receipt.exit_class, ExitClass::InfrastructureFailure);
        let packet = fs::read_to_string(receipt.evidence_path).unwrap();
        assert!(packet.contains("\"status\": \"error\""));
        assert!(packet.contains("\"failure_class\": \"worker-error\""));
        assert!(packet.contains("worker partial-write"));
    }

    #[test]
    fn worker_timeout_and_crash_kinds_are_stable_packet_failure_classes() {
        let temp = tempdir().unwrap();
        let out_dir = temp.path().join("latest");
        let manifest = FlowManifest::load(Path::new("examples/login-flow.yml")).unwrap();
        let failure = RunFailure::new(
            "worker-timeout",
            "worker-adapter",
            "worker timed out after 1 ms".to_string(),
        );
        let response = WorkerResponse::error(failure.message.clone());

        let receipt = write_packet_and_report(
            &manifest,
            Path::new("examples/login-flow.yml"),
            &out_dir,
            response,
            vec![failure],
            Utc::now(),
            Utc::now(),
            "run-timeout".to_string(),
        )
        .unwrap();

        assert_eq!(receipt.exit_class, ExitClass::InfrastructureFailure);
        let packet = fs::read_to_string(receipt.evidence_path).unwrap();
        assert!(packet.contains("\"failure_class\": \"worker-timeout\""));
        assert!(packet.contains("\"infrastructure_failures\": 1"));
    }

    #[test]
    fn nondeterminism_marks_packet_error_instead_of_release_pass() {
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
        let mut response = passing_worker_response();
        response
            .nondeterminism
            .push("route state changed between capture attempts".to_string());

        let receipt = write_packet_and_report(
            &manifest,
            Path::new("examples/login-flow.yml"),
            &out_dir,
            response,
            Vec::new(),
            Utc::now(),
            Utc::now(),
            "run-nondeterminism".to_string(),
        )
        .unwrap();

        assert_eq!(receipt.exit_class, ExitClass::InfrastructureFailure);
        let packet = fs::read_to_string(receipt.evidence_path).unwrap();
        assert!(packet.contains("\"failure_class\": \"nondeterminism\""));
        assert!(packet.contains("route state changed between capture attempts"));
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
        assert!(
            parsed["properties"]["waivers"]["items"]["anyOf"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value["required"][0] == "packet_ref")
        );
    }

    #[test]
    fn wcag22_profile_maps_axe_tags_to_versioned_obligations() {
        let profile: serde_json::Value = serde_json::from_str(WCAG22_AA_PROFILE_JSON).unwrap();

        assert_eq!(profile["id"], "wcag22-aa");
        assert_eq!(
            obligation_from_tags("wcag22-aa", &["wcag2aa".to_string(), "wcag143".to_string()]),
            "wcag22-aa:1.4.3-contrast-minimum"
        );
    }

    #[test]
    fn standards_verdicts_preserve_residual_review_without_legal_claims() {
        let manifest = FlowManifest::load(Path::new("examples/login-flow.yml")).unwrap();
        let response = passing_worker_response();
        let verdicts = verdicts_from_findings(&manifest, &response, &[]);

        assert!(verdicts.iter().any(|verdict| verdict.status == "pass"
            && verdict.obligation == "wcag22-aa:deterministic-axe-rules"));
        assert!(verdicts.iter().any(|verdict| verdict.status == "not_tested"
            && verdict.obligation == "wcag22-aa:2.1.1-keyboard-traversal"));
        assert!(
            verdicts
                .iter()
                .any(|verdict| verdict.status == "needs_review"
                    && verdict.obligation == "wcag22-aa:human-assistive-technology-review")
        );
        assert!(
            !verdicts
                .iter()
                .any(|verdict| verdict.obligation == "legal-compliance")
        );
    }

    #[test]
    fn release_cli_writes_neutral_check_for_residual_review_packet() {
        let temp = tempdir().unwrap();
        let packet_path = write_passing_evidence_packet(&temp.path().join("run"));
        let out_dir = temp.path().join("release");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let code = run_cli_with_io(
            vec![
                "release".to_string(),
                "--packet".to_string(),
                packet_path.to_string_lossy().to_string(),
                "--out".to_string(),
                out_dir.to_string_lossy().to_string(),
                "--changed-surface".to_string(),
                "login-form".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 0);
        assert!(stderr.is_empty());
        let stdout = String::from_utf8(stdout).unwrap();
        assert!(stdout.contains("Status: needs_review"));
        assert!(out_dir.join("release-summary.json").exists());
        assert!(out_dir.join("github-check.json").exists());
        assert!(out_dir.join("release-report.html").exists());

        let check = fs::read_to_string(out_dir.join("github-check.json")).unwrap();
        let check: serde_json::Value = serde_json::from_str(&check).unwrap();
        assert_eq!(check["conclusion"], "neutral");
        assert!(
            check["output"]["summary"]
                .as_str()
                .unwrap()
                .contains("status=needs_review")
        );

        let report = fs::read_to_string(out_dir.join("release-report.html")).unwrap();
        assert!(report.contains("Allie release decision: needs_review"));
        assert!(report.contains("not a legal compliance guarantee"));
    }

    #[test]
    fn release_cli_rejects_invalid_packet_status_before_projection() {
        let temp = tempdir().unwrap();
        let source_packet_path = write_passing_evidence_packet(&temp.path().join("run"));
        let mut packet: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(source_packet_path).unwrap()).unwrap();
        packet["summary"]["status"] = serde_json::json!("approved");
        let packet_path = temp.path().join("invalid-evidence.json");
        let out_dir = temp.path().join("release");
        write_json_pretty(&packet_path, &packet).unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let code = run_cli_with_io(
            vec![
                "release".to_string(),
                "--packet".to_string(),
                packet_path.to_string_lossy().to_string(),
                "--out".to_string(),
                out_dir.to_string_lossy().to_string(),
                "--changed-surface".to_string(),
                "login-form".to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 2);
        assert!(String::from_utf8(stdout).unwrap().is_empty());
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("invalid evidence packet status approved"));
        assert!(!out_dir.join("release-summary.json").exists());
        assert!(!out_dir.join("github-check.json").exists());
    }

    #[test]
    fn release_cli_rejects_invalid_packet_schema_before_projection() {
        let temp = tempdir().unwrap();
        let source_packet_path = write_passing_evidence_packet(&temp.path().join("run"));
        let mut packet: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(source_packet_path).unwrap()).unwrap();
        packet["schema"] = serde_json::json!("allie.evidence.future");
        let packet_path = temp.path().join("wrong-schema-evidence.json");
        let out_dir = temp.path().join("release");
        write_json_pretty(&packet_path, &packet).unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let code = run_cli_with_io(
            vec![
                "release".to_string(),
                "--packet".to_string(),
                packet_path.to_string_lossy().to_string(),
                "--out".to_string(),
                out_dir.to_string_lossy().to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 2);
        assert!(String::from_utf8(stdout).unwrap().is_empty());
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("invalid evidence packet schema allie.evidence.future"));
        assert!(!out_dir.join("release-summary.json").exists());
    }

    #[test]
    fn release_cli_rejects_schema_unknown_fields_before_projection() {
        let temp = tempdir().unwrap();
        let source_packet_path = write_passing_evidence_packet(&temp.path().join("run"));
        let mut packet: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(source_packet_path).unwrap()).unwrap();
        packet["summary"]["unexpected"] = serde_json::json!(true);
        let packet_path = temp.path().join("unknown-field-evidence.json");
        let out_dir = temp.path().join("release");
        write_json_pretty(&packet_path, &packet).unwrap();
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        let code = run_cli_with_io(
            vec![
                "release".to_string(),
                "--packet".to_string(),
                packet_path.to_string_lossy().to_string(),
                "--out".to_string(),
                out_dir.to_string_lossy().to_string(),
            ],
            &mut stdout,
            &mut stderr,
        );

        assert_eq!(code, 2);
        assert!(String::from_utf8(stdout).unwrap().is_empty());
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("unknown field"));
        assert!(!out_dir.join("release-summary.json").exists());
    }

    #[test]
    fn release_projection_blocks_packet_failures() {
        let mut packet = minimal_release_packet();
        packet["summary"]["status"] = serde_json::json!("fail");
        packet["summary"]["deterministic_failures"] = serde_json::json!(1);

        let projection = project_release_decision(&packet, &release_options(vec![]));

        assert_eq!(projection.exit_class, ExitClass::BlockingFinding);
        assert_eq!(projection.summary["status"], "blocked");
        assert_eq!(projection.github_check["conclusion"], "failure");
    }

    #[test]
    fn release_projection_does_not_block_model_only_findings() {
        let mut packet = minimal_release_packet();
        packet["findings"] = serde_json::json!([
            {
                "id": "agentic-1",
                "title": "Possible label ambiguity",
                "evidence_class": "agentic",
                "confidence": "agent_inferred"
            }
        ]);

        let projection = project_release_decision(&packet, &release_options(vec!["login-form"]));

        assert_eq!(projection.exit_class, ExitClass::Success);
        assert_eq!(projection.summary["status"], "needs_review");
        assert_eq!(projection.github_check["conclusion"], "neutral");
        assert_eq!(projection.summary["model_findings_non_blocking"], 1);
    }

    #[test]
    fn release_projection_blocks_missing_changed_surface_evidence() {
        let packet = minimal_release_packet();

        let projection = project_release_decision(&packet, &release_options(vec!["settings"]));

        assert_eq!(projection.exit_class, ExitClass::BlockingFinding);
        assert_eq!(projection.summary["status"], "blocked");
        assert_eq!(
            projection.summary["blocking"]["missing_required_evidence"][0],
            "settings"
        );
    }

    #[test]
    fn release_projection_blocks_expired_touched_waivers() {
        let mut packet = minimal_release_packet();
        packet["waivers"] = serde_json::json!([
            {
                "id": "waiver-1",
                "surface": "login-form",
                "status": "risk_accepted",
                "provenance": {"actor": "accessibility-lead"},
                "expires_at": (Utc::now() - chrono::Duration::days(1)).to_rfc3339(),
                "packet_ref": "run-release"
            }
        ]);

        let projection = project_release_decision(&packet, &release_options(vec!["login-form"]));

        assert_eq!(projection.exit_class, ExitClass::BlockingFinding);
        assert_eq!(projection.summary["status"], "blocked");
        assert_eq!(
            projection.summary["blocking"]["expired_waivers"][0]["id"],
            "waiver-1"
        );
    }

    #[test]
    fn release_projection_blocks_invalid_touched_waivers() {
        let mut packet = minimal_release_packet();
        packet["waivers"] = serde_json::json!([
            {
                "id": "waiver-2",
                "surface": "login-form",
                "status": "waived",
                "expires_at": (Utc::now() + chrono::Duration::days(3)).to_rfc3339()
            }
        ]);

        let projection = project_release_decision(&packet, &release_options(vec!["login-form"]));

        assert_eq!(projection.exit_class, ExitClass::BlockingFinding);
        assert_eq!(projection.summary["status"], "blocked");
        assert_eq!(
            projection.summary["blocking"]["invalid_waivers"][0]["id"],
            "waiver-2"
        );
    }

    #[test]
    fn release_projection_routes_stale_evidence_to_review() {
        let mut packet = minimal_release_packet();
        packet["run"]["finished_at"] =
            serde_json::json!((Utc::now() - chrono::Duration::days(30)).to_rfc3339());

        let projection = project_release_decision(&packet, &release_options(vec!["login-form"]));

        assert_eq!(projection.exit_class, ExitClass::Success);
        assert_eq!(projection.summary["status"], "needs_review");
        assert_eq!(projection.github_check["conclusion"], "neutral");
        assert_eq!(projection.summary["review"]["stale_evidence"], true);
    }

    fn release_options(changed_surfaces: Vec<&str>) -> ReleaseOptions {
        ReleaseOptions {
            packet_path: PathBuf::from("evidence.json"),
            out_dir: PathBuf::from("release"),
            changed_surfaces: changed_surfaces
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            stale_after_days: 7,
        }
    }

    fn minimal_release_packet() -> serde_json::Value {
        serde_json::json!({
            "schema": "allie.evidence.v0",
            "summary": {
                "status": "pass",
                "exit_code": 0,
                "deterministic_failures": 0,
                "scripted_failures": 0,
                "infrastructure_failures": 0,
                "states_captured": 1,
                "failure_class": null
            },
            "run": {
                "id": "run-release",
                "finished_at": Utc::now().to_rfc3339()
            },
            "coverage": {
                "states_captured": ["login-form"],
                "surfaces_discovered": ["Allie Fixture"]
            },
            "artifacts": [
                {"type": "axe_json"},
                {"type": "screenshot"},
                {"type": "html_report"}
            ],
            "findings": [],
            "verdicts": [],
            "waivers": [],
            "policy": {
                "model_status": "disabled",
                "model_provider_allowlist": [],
                "zdr_required": true
            }
        })
    }

    fn write_passing_evidence_packet(out_dir: &Path) -> PathBuf {
        let artifacts_dir = out_dir.join("artifacts");
        fs::create_dir_all(&artifacts_dir).unwrap();
        fs::write(
            artifacts_dir.join("axe-login-form.json"),
            br#"{"violations":[]}"#,
        )
        .unwrap();
        fs::write(artifacts_dir.join("login-form.png"), b"fake-png").unwrap();

        let manifest = FlowManifest::load(Path::new("examples/login-flow.yml")).unwrap();
        write_packet_and_report(
            &manifest,
            Path::new("examples/login-flow.yml"),
            out_dir,
            passing_worker_response(),
            Vec::new(),
            Utc::now(),
            Utc::now(),
            "run-release-cli".to_string(),
        )
        .unwrap()
        .evidence_path
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
            nondeterminism: Vec::new(),
        }
    }
}
