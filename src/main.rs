use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const CATALOG: &str = include_str!("../capabilities/catalog.json");
const SKILL: &str = include_str!("../skills/filmmap/SKILL.md");
const TOOL_MANIFEST: &str = include_str!("../tools/manifest.json");

#[derive(Parser)]
#[command(
    name = "filmmap",
    version,
    about = "Build evidence-backed media indexes for editing agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a FilmMap workspace with a starter profile and JSONL index.
    Init { path: Option<PathBuf> },
    /// List the stable capability vocabulary.
    Capability {
        #[command(subcommand)]
        command: CapabilityCommands,
    },
    /// Inspect or validate a machine-specific capability profile.
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },
    /// Compare a requirement with a profile and report executable/missing work.
    Plan {
        #[arg(long)]
        requirement: PathBuf,
        #[arg(long)]
        profile: PathBuf,
    },
    /// Add a media file to the index with a content hash and basic file evidence.
    Artifact {
        #[command(subcommand)]
        command: ArtifactCommands,
    },
    /// Append an explicit observation to a JSONL evidence index.
    Observe {
        #[command(subcommand)]
        command: ObserveCommands,
    },
    /// Build a deterministic, portable JSON index from JSONL inputs.
    Synthesize {
        source: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    /// Check JSONL records and stable IDs for basic integrity.
    Validate { index: PathBuf },
    /// Search indexed records by case-insensitive text and optional time range.
    Query {
        index: PathBuf,
        query: String,
        #[arg(long)]
        from: Option<f64>,
        #[arg(long)]
        to: Option<f64>,
        #[arg(long)]
        from_ms: Option<u64>,
        #[arg(long)]
        to_ms: Option<u64>,
        #[arg(long)]
        asset_id: Option<String>,
        #[arg(long)]
        kind: Option<String>,
    },
    /// Install the bundled Agent skill or extension scripts.
    Install {
        #[command(subcommand)]
        command: InstallCommands,
    },
}
#[derive(Subcommand)]
enum CapabilityCommands {
    List,
}
#[derive(Subcommand)]
enum ProfileCommands {
    Detect {
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Validate {
        path: PathBuf,
    },
}
#[derive(Subcommand)]
enum ArtifactCommands {
    Add {
        index: PathBuf,
        path: PathBuf,
        #[arg(long, default_value = "video")]
        kind: String,
        #[arg(long)]
        parent_asset: Option<String>,
        #[arg(long)]
        time_ms: Option<u64>,
        #[arg(long)]
        mime_type: Option<String>,
    },
}
#[derive(Subcommand)]
enum ObserveCommands {
    Add {
        index: PathBuf,
        artifact_id: String,
        #[arg(long)]
        capability: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        class: Option<String>,
        #[arg(long)]
        at: Option<f64>,
        #[arg(long)]
        at_ms: Option<u64>,
        #[arg(long)]
        start_ms: Option<u64>,
        #[arg(long)]
        end_ms: Option<u64>,
        #[arg(long)]
        value: Option<String>,
        #[arg(long)]
        value_json: Option<String>,
        #[arg(long, default_value = "agent")]
        source: String,
        #[arg(long)]
        producer_version: Option<String>,
        #[arg(long, default_value = "candidate")]
        status: String,
        #[arg(long)]
        confidence: Option<f64>,
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
    },
}
#[derive(Subcommand)]
enum InstallCommands {
    Skill {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    Tools {
        #[arg(long)]
        dir: Option<PathBuf>,
    },
}

#[derive(Serialize, Deserialize)]
struct Profile {
    schema_version: String,
    profile_id: String,
    #[serde(default)]
    generated_at: String,
    #[serde(default)]
    generated_at_unix_ms: u128,
    capabilities: Vec<CapabilityState>,
}
#[derive(Serialize, Deserialize)]
struct CapabilityState {
    id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    executable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_processing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    precision: Option<Value>,
}

fn main() -> Result<()> {
    run(Cli::parse())
}
fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init { path } => init(path.unwrap_or_else(|| PathBuf::from("."))),
        Commands::Capability {
            command: CapabilityCommands::List,
        } => {
            println!("{CATALOG}");
            Ok(())
        }
        Commands::Profile { command } => match command {
            ProfileCommands::Detect { output } => {
                let p = detect_profile();
                let text = serde_json::to_string_pretty(&p)?;
                if let Some(path) = output {
                    fs::write(&path, &text).with_context(|| format!("write {}", path.display()))?;
                } else {
                    println!("{text}");
                }
                Ok(())
            }
            ProfileCommands::Validate { path } => {
                let p: Profile = serde_json::from_slice(
                    &fs::read(&path).with_context(|| format!("read {}", path.display()))?,
                )?;
                validate_profile(&p)?;
                println!(
                    "valid profile: {} ({} capabilities)",
                    p.profile_id,
                    p.capabilities.len()
                );
                Ok(())
            }
        },
        Commands::Plan {
            requirement,
            profile,
        } => plan(&requirement, &profile),
        Commands::Artifact {
            command:
                ArtifactCommands::Add {
                    index,
                    path,
                    kind,
                    parent_asset,
                    time_ms,
                    mime_type,
                },
        } => add_artifact(
            &index,
            &path,
            &kind,
            parent_asset.as_deref(),
            time_ms,
            mime_type.as_deref(),
        ),
        Commands::Observe {
            command:
                ObserveCommands::Add {
                    index,
                    artifact_id,
                    kind,
                    capability,
                    class,
                    at,
                    at_ms,
                    start_ms,
                    end_ms,
                    value,
                    value_json,
                    source,
                    producer_version,
                    status,
                    confidence,
                    evidence_refs,
                },
        } => add_observation(AddObservation {
            index,
            artifact_id,
            kind: kind.or(capability).context("provide --kind")?,
            class: class.unwrap_or_else(|| "interpretation".to_string()),
            at,
            at_ms,
            start_ms,
            end_ms,
            value,
            value_json,
            source,
            producer_version,
            status,
            confidence,
            evidence_refs,
        }),
        Commands::Synthesize { source, out } => synthesize(&source, &out),
        Commands::Validate { index } => validate_index(&index),
        Commands::Query {
            index,
            query,
            from,
            to,
            from_ms,
            to_ms,
            asset_id,
            kind,
        } => query_index(
            &index,
            &query,
            QueryOptions {
                from,
                to,
                from_ms,
                to_ms,
                asset_id,
                kind,
            },
        ),
        Commands::Install {
            command: InstallCommands::Skill { dir },
        } => install_text(dir.unwrap_or_else(default_skill_dir), "SKILL.md", SKILL),
        Commands::Install {
            command: InstallCommands::Tools { dir },
        } => install_tools(dir.unwrap_or_else(default_tools_dir)),
    }
}
fn init(root: PathBuf) -> Result<()> {
    fs::create_dir_all(&root)?;
    for (name, value) in [("profile.json", serde_json::to_string_pretty(&detect_profile())?), ("index.jsonl", String::new()), ("filmmap.json", "{\n  \"schema_version\": \"filmmap/v1\",\n  \"index\": \"index.jsonl\",\n  \"profile\": \"profile.json\"\n}\n".to_string())] {
        let path = root.join(name); if !path.exists() { fs::write(path, value)?; }
    }
    println!("initialized FilmMap workspace at {}", root.display());
    Ok(())
}
fn detect_profile() -> Profile {
    let entries: Vec<Value> = serde_json::from_str(CATALOG).unwrap_or_default();
    let capabilities = entries
        .iter()
        .map(|e| {
            let id = e["id"].as_str().unwrap_or_default().to_string();
            let exe = e["executable"].as_str();
            if let Some(exe) = exe {
                if let Some(path) = find_on_path(exe) {
                    CapabilityState {
                        id,
                        status: "available".into(),
                        provider: Some(exe.to_string()),
                        executable: Some(path),
                        version: None,
                        note: None,
                        external_processing: Some(false),
                        precision: None,
                    }
                } else {
                    CapabilityState {
                        id,
                        status: "unavailable".into(),
                        provider: None,
                        executable: None,
                        version: None,
                        note: Some(format!("executable {exe} not found on PATH")),
                        external_processing: Some(false),
                        precision: None,
                    }
                }
            } else {
                CapabilityState {
                    id,
                    status: "unverified".into(),
                    provider: None,
                    executable: None,
                    version: None,
                    note: Some(
                        "provided by an external agent or service; not probed by FilmMap".into(),
                    ),
                    external_processing: None,
                    precision: None,
                }
            }
        })
        .collect();
    Profile {
        schema_version: "filmmap/profile/v1".into(),
        profile_id: "local-auto-detect".into(),
        generated_at: "legacy-compatible".into(),
        generated_at_unix_ms: now_unix_ms(),
        capabilities,
    }
}
fn find_on_path(name: &str) -> Option<String> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|p| p.join(name))
        .find(|p| is_executable(p))
        .map(|p| p.display().to_string())
}
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}
#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}
fn validate_profile(p: &Profile) -> Result<()> {
    if p.schema_version != "filmmap/profile/v1" {
        bail!("unsupported schema_version: {}", p.schema_version);
    }
    let catalog: Vec<Value> = serde_json::from_str(CATALOG)?;
    let valid: Vec<&str> = catalog.iter().filter_map(|v| v["id"].as_str()).collect();
    let mut seen = std::collections::HashSet::new();
    for c in &p.capabilities {
        if !valid.contains(&c.id.as_str()) {
            bail!("unknown capability id: {}", c.id);
        }
        if !seen.insert(&c.id) {
            bail!("duplicate capability profile entry: {}", c.id);
        }
        if !["available", "unavailable", "unverified", "failed"].contains(&c.status.as_str()) {
            bail!("invalid status for {}: {}", c.id, c.status);
        }
        if c.status == "available" && c.provider.is_none() && c.executable.is_none() {
            bail!(
                "available capability requires provider or executable: {}",
                c.id
            );
        }
        if c.status == "unavailable" && c.note.as_deref().unwrap_or_default().is_empty() {
            bail!("unavailable capability requires a reason: {}", c.id);
        }
    }
    Ok(())
}
fn plan(req_path: &Path, profile_path: &Path) -> Result<()> {
    let req: Value = serde_json::from_slice(&fs::read(req_path)?)?;
    let p: Profile = serde_json::from_slice(&fs::read(profile_path)?)?;
    validate_profile(&p)?;
    let required: Vec<String> = req["required_capabilities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .chain(
            req["targets"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|target| target["level"] == "required")
                .filter_map(|target| target["capability"].as_str().map(str::to_string)),
        )
        .collect();
    let optional: Vec<String> = req["optional_capabilities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .chain(
            req["targets"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|target| target["level"] == "optional")
                .filter_map(|target| target["capability"].as_str().map(str::to_string)),
        )
        .collect();
    let if_available: Vec<String> = req["if_available_capabilities"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .chain(
            req["targets"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|target| target["level"] == "if_available")
                .filter_map(|target| target["capability"].as_str().map(str::to_string)),
        )
        .collect();
    if required.is_empty() && optional.is_empty() && if_available.is_empty() {
        bail!(
            "requirement must declare required_capabilities, optional_capabilities, if_available_capabilities, or targets"
        );
    }
    let classify = |ids: &[String]| -> (Vec<String>, Vec<String>, Vec<String>) {
        let mut satisfied = Vec::new();
        let mut potentially = Vec::new();
        let mut missing = Vec::new();
        for id in ids {
            match p
                .capabilities
                .iter()
                .find(|c| c.id == *id)
                .map(|c| c.status.as_str())
            {
                Some("available") => satisfied.push(id.clone()),
                Some("unverified") => potentially.push(id.clone()),
                _ => missing.push(id.clone()),
            }
        }
        (satisfied, potentially, missing)
    };
    let (satisfied, potentially_satisfied, missing) = classify(&required);
    let (optional_satisfied, optional_potential, optional_missing) = classify(&optional);
    let (conditional_satisfied, conditional_potential, conditional_missing) =
        classify(&if_available);
    let mut constraint_checks = Vec::new();
    let mut constraint_blocked = false;
    let mut constraint_unverified = false;
    for target in req["targets"].as_array().into_iter().flatten() {
        let capability = target["capability"].as_str().unwrap_or_default();
        let state = p.capabilities.iter().find(|c| c.id == capability);
        let required_target = target["level"] == "required";
        if let Some(max_error) = target["target_precision_ms"].as_f64() {
            let actual = state
                .and_then(|c| c.precision.as_ref())
                .and_then(|v| v["max_error_ms"].as_f64());
            let status = match actual {
                Some(actual) if actual <= max_error => "satisfied",
                Some(_) => "missing",
                None => "unverified",
            };
            if required_target && status == "missing" {
                constraint_blocked = true;
            }
            if required_target && status == "unverified" {
                constraint_unverified = true;
            }
            constraint_checks.push(json!({"capability":capability,"constraint":"target_precision_ms","requested":max_error,"observed":actual,"status":status}));
        }
    }
    if req["constraints"]["external_processing"] == false {
        for id in required
            .iter()
            .chain(optional.iter())
            .chain(if_available.iter())
        {
            if let Some(state) = p.capabilities.iter().find(|c| c.id == *id) {
                match state.external_processing {
                    Some(true) => {
                        if required.contains(id) {
                            constraint_blocked = true;
                        }
                        constraint_checks.push(json!({"capability":id,"constraint":"external_processing","requested":false,"observed":true,"status":"missing"}));
                    }
                    None if state.status != "unavailable" => {
                        if required.contains(id) {
                            constraint_unverified = true;
                        }
                        constraint_checks.push(json!({"capability":id,"constraint":"external_processing","requested":false,"observed":null,"status":"unverified"}));
                    }
                    _ => {}
                }
            }
        }
    }
    let executable = missing.is_empty()
        && potentially_satisfied.is_empty()
        && !constraint_blocked
        && !constraint_unverified;
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "requirement_id":req["id"], "status": if executable {"satisfied"} else if !missing.is_empty() {"missing"} else {"potentially_satisfied"},
            "required":{"satisfied":satisfied,"potentially_satisfied":potentially_satisfied,"missing":missing},
            "optional":{"satisfied":optional_satisfied,"potentially_satisfied":optional_potential,"missing":optional_missing},
            "if_available":{"satisfied":conditional_satisfied,"potentially_satisfied":conditional_potential,"missing":conditional_missing},
            "constraint_checks":constraint_checks,
            "executable":executable
        }))?
    );
    Ok(())
}
struct AddObservation {
    index: PathBuf,
    artifact_id: String,
    kind: String,
    class: String,
    at: Option<f64>,
    at_ms: Option<u64>,
    start_ms: Option<u64>,
    end_ms: Option<u64>,
    value: Option<String>,
    value_json: Option<String>,
    source: String,
    producer_version: Option<String>,
    status: String,
    confidence: Option<f64>,
    evidence_refs: Vec<String>,
}

fn add_observation(input: AddObservation) -> Result<()> {
    let AddObservation {
        index,
        artifact_id,
        kind,
        class,
        at,
        at_ms,
        start_ms,
        end_ms,
        value,
        value_json,
        source,
        producer_version,
        status,
        confidence,
        evidence_refs,
    } = input;
    if !["fact", "measurement", "interpretation"].contains(&class.as_str()) {
        bail!("invalid observation class: {class}");
    }
    if !["accepted", "candidate", "rejected", "conflicted"].contains(&status.as_str()) {
        bail!("invalid observation status: {status}");
    }
    if let Some(c) = confidence
        && (!c.is_finite() || !(0.0..=1.0).contains(&c))
    {
        bail!("confidence must be in 0..=1");
    }
    if let Some(seconds) = at
        && (!seconds.is_finite() || seconds < 0.0)
    {
        bail!("--at must be a finite non-negative source time in seconds");
    }
    let time_fields = usize::from(at.is_some())
        + usize::from(at_ms.is_some())
        + usize::from(start_ms.is_some())
        + usize::from(end_ms.is_some());
    let time = if time_fields == 0 {
        Value::Null
    } else if start_ms.is_some() && end_ms.is_some() && at.is_none() && at_ms.is_none() {
        let (start, end) = (start_ms.unwrap(), end_ms.unwrap());
        if end <= start {
            bail!("time range must satisfy end_ms > start_ms");
        }
        json!({"start_ms":start,"end_ms":end})
    } else if start_ms.is_none() && end_ms.is_none() && at.is_some() != at_ms.is_some() {
        let time_ms = at_ms.unwrap_or_else(|| (at.unwrap() * 1000.0).round() as u64);
        json!({"at_ms":time_ms})
    } else {
        bail!("time must be a single at/at_ms or a complete start_ms/end_ms range");
    };
    let value = match (value, value_json) {
        (Some(_), Some(_)) => bail!("provide either --value or --value-json, not both"),
        (Some(text), None) => json!({"text":text}),
        (None, Some(raw)) => {
            serde_json::from_str(&raw).context("--value-json must be valid JSON")?
        }
        (None, None) => bail!("provide --value or --value-json"),
    };
    let mut refs = evidence_refs;
    refs.sort();
    refs.dedup();
    validate_observation_refs(&index, &artifact_id, &refs)?;
    let producer_type = if source.starts_with("human") || source.starts_with("manual") {
        "human"
    } else {
        "agent"
    };
    let mut record = json!({
        "record_type":"observation", "protocol":"filmmap.observation", "version":"0.1.0",
        "asset_id":artifact_id, "class":class, "kind":kind,
        "time":time, "value":value,
        "producer":{"type":producer_type,"id":source,"version":producer_version},
        "evidence_refs":refs, "evidence_state":if refs.is_empty(){"none"}else{"available"},
        "status":status, "confidence":confidence,
        "created_at_unix_ms":now_unix_ms()
    });
    let id_input = record.clone();
    let mut id_input = id_input;
    id_input
        .as_object_mut()
        .unwrap()
        .remove("created_at_unix_ms");
    let digest = Sha256::digest(serde_json::to_vec(&id_input)?);
    record["id"] = json!(format!("obs_sha256:{digest:x}"));
    append_idempotent(&index, &record)
}

fn validate_observation_refs(index: &Path, asset_id: &str, refs: &[String]) -> Result<()> {
    let data =
        fs::read_to_string(index).with_context(|| format!("read index {}", index.display()))?;
    let mut ids = std::collections::HashSet::new();
    for line in data.lines() {
        let record: Value = serde_json::from_str(line)?;
        if record["record_type"] == "artifact"
            && let Some(id) = record["artifact_id"].as_str()
        {
            ids.insert(id.to_string());
        }
    }
    if !ids.contains(asset_id) {
        bail!("unknown asset_id: {asset_id}");
    }
    for reference in refs {
        if !ids.contains(reference) {
            bail!("unknown evidence artifact: {reference}");
        }
    }
    Ok(())
}

fn append_idempotent(index: &Path, record: &Value) -> Result<()> {
    let id = record["id"].as_str().context("record id required")?;
    if index.exists() {
        for (line_no, line) in fs::read_to_string(index)?.lines().enumerate() {
            let existing: Value = serde_json::from_str(line)
                .with_context(|| format!("invalid JSON at line {}", line_no + 1))?;
            if existing["id"] == id {
                let mut existing_content = existing.clone();
                existing_content
                    .as_object_mut()
                    .unwrap()
                    .remove("created_at_unix_ms");
                let mut new_content = record.clone();
                new_content
                    .as_object_mut()
                    .unwrap()
                    .remove("created_at_unix_ms");
                if existing_content == new_content {
                    return Ok(());
                }
                bail!("record ID conflict: {id}");
            }
        }
    }
    append_jsonl(index, record)
}

fn now_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn add_artifact(
    index: &Path,
    path: &Path,
    kind: &str,
    parent_asset: Option<&str>,
    time_ms: Option<u64>,
    mime_type: Option<&str>,
) -> Result<()> {
    if parent_asset.is_some() && time_ms.is_none() && kind == "frame" {
        bail!("frame artifacts require --time-ms to preserve their source position");
    }
    if let Some(parent) = parent_asset {
        validate_observation_refs(index, parent, &[])?;
    }
    let mut file =
        fs::File::open(path).with_context(|| format!("open media {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let n = file
            .read(&mut buffer)
            .with_context(|| format!("hash media {}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let digest = format!("{:x}", hasher.finalize());
    let id = format!("sha256:{digest}");
    let metadata = fs::metadata(path)?;
    if index.exists() {
        let content = fs::read_to_string(index)
            .with_context(|| format!("read existing index {}", index.display()))?;
        let mut already_referenced = false;
        for (line_no, line) in content.lines().enumerate() {
            let record: Value = serde_json::from_str(line)
                .with_context(|| format!("invalid JSON at index line {}", line_no + 1))?;
            if record["artifact_id"] == id
                && record["path"] == json!(path)
                && record["parent_asset_id"] == parent_asset.map(Value::from).unwrap_or(Value::Null)
                && record["time"] == time_ms.map(|ms| json!({"at_ms":ms})).unwrap_or(Value::Null)
            {
                already_referenced = true;
                break;
            }
        }
        if !already_referenced {
            if content.lines().any(|line| {
                serde_json::from_str::<Value>(line).is_ok_and(|record| record["artifact_id"] == id)
            }) {
                append_jsonl(
                    index,
                    &json!({"record_type":"artifact_path","artifact_id":id,"path":path,"parent_asset_id":parent_asset,"kind":kind,"time":time_ms.map(|ms| json!({"at_ms":ms})),"mime_type":mime_type.or_else(||mime_for_path(path)),"evidence":{"filesystem_path":path.display().to_string()}}),
                )?;
                println!("{id}");
                return Ok(());
            }
        } else {
            println!("{id}");
            return Ok(());
        }
    }
    let mut record = json!({
        "record_type":"artifact", "protocol":"filmmap.artifact", "version":"0.1.0",
        "artifact_id":id, "asset_id": if parent_asset.is_none() {Some(id.as_str())} else {None},
        "parent_asset_id":parent_asset, "path":path, "kind":kind,
        "time":{"at_ms":time_ms}, "mime_type":mime_type.or_else(||mime_for_path(path)),
        "size_bytes":metadata.len(), "sha256":digest,
        "evidence":{"filesystem_path":path.display().to_string()}
    });
    if time_ms.is_none() {
        record.as_object_mut().unwrap().remove("time");
    }
    if parent_asset.is_none() {
        record.as_object_mut().unwrap().remove("parent_asset_id");
    }
    if mime_type.is_none() && mime_for_path(path).is_none() {
        record.as_object_mut().unwrap().remove("mime_type");
    }
    append_jsonl(index, &record)?;
    println!("{id}");
    Ok(())
}
fn mime_for_path(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "mp4" | "m4v" => Some("video/mp4"),
        "mov" => Some("video/quicktime"),
        "mkv" => Some("video/x-matroska"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        "wav" => Some("audio/wav"),
        "mp3" => Some("audio/mpeg"),
        "aac" => Some("audio/aac"),
        _ => None,
    }
}
fn append_jsonl(index: &Path, value: &Value) -> Result<()> {
    if let Some(p) = index.parent() {
        fs::create_dir_all(p)?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(index)
        .with_context(|| format!("open index {}", index.display()))?;
    writeln!(f, "{}", value).with_context(|| format!("append index {}", index.display()))?;
    Ok(())
}
fn validate_index(index: &Path) -> Result<()> {
    let data = fs::read_to_string(index)?;
    if let Ok(value) = serde_json::from_str::<Value>(&data)
        && value["protocol"] == "filmmap.index"
    {
        return validate_canonical_index(value);
    }
    let mut seen = std::collections::HashSet::new();
    let mut observation_ids = std::collections::HashSet::new();
    let mut references = Vec::new();
    let mut count = 0;
    for (i, line) in data.lines().enumerate() {
        let v: Value = serde_json::from_str(line)
            .with_context(|| format!("invalid JSON at line {}", i + 1))?;
        let t = v["record_type"].as_str().context("record_type required")?;
        if !["artifact", "artifact_path", "observation", "resolution"].contains(&t) {
            bail!("unknown record_type at line {}: {t}", i + 1);
        }
        if t == "artifact" {
            let id = v["artifact_id"].as_str().context("artifact_id required")?;
            if !seen.insert(id.to_string()) {
                bail!("duplicate artifact_id at line {}: {id}", i + 1);
            }
        } else if t == "observation" && v["protocol"] == "filmmap.observation" {
            validate_observation(&v, i + 1)?;
            if let Some(id) = v["id"].as_str()
                && !observation_ids.insert(id.to_string())
            {
                bail!("duplicate observation id at line {}: {id}", i + 1);
            }
            let asset_id = v["asset_id"].as_str().context("asset_id required")?;
            references.push((i + 1, asset_id.to_string()));
            for reference in v["evidence_refs"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                references.push((i + 1, reference.to_string()));
            }
        } else if v["artifact_id"].as_str().is_none() {
            bail!("artifact_id required at line {}", i + 1);
        } else if let Some(id) = v["artifact_id"].as_str() {
            references.push((i + 1, id.to_string()));
        }
        count += 1;
    }
    for (line, id) in references {
        if !seen.contains(&id) {
            bail!("artifact reference at line {line} has no artifact record: {id}");
        }
    }
    println!("valid index: {count} records");
    Ok(())
}
fn validate_observation(value: &Value, line: usize) -> Result<()> {
    for key in [
        "id", "asset_id", "kind", "class", "status", "producer", "value",
    ] {
        if value[key].is_null() {
            bail!("OBS_REQUIRED at /records/{line}/{key}");
        }
    }
    if !["available", "none"].contains(&value["evidence_state"].as_str().unwrap_or("none")) {
        bail!("OBS_EVIDENCE_STATE at /records/{line}/evidence_state");
    }
    if !["fact", "measurement", "interpretation"]
        .contains(&value["class"].as_str().unwrap_or_default())
    {
        bail!("OBS_CLASS at /records/{line}/class");
    }
    if !["accepted", "candidate", "rejected", "conflicted"]
        .contains(&value["status"].as_str().unwrap_or_default())
    {
        bail!("OBS_STATUS at /records/{line}/status");
    }
    if let Some(c) = value["confidence"].as_f64()
        && !(0.0..=1.0).contains(&c)
    {
        bail!("OBS_CONFIDENCE at /records/{line}/confidence");
    }
    validate_time(&value["time"], &format!("/records/{line}/time"))
}
fn validate_time(time: &Value, path: &str) -> Result<()> {
    if time.is_null() {
        return Ok(());
    }
    let at = time["at_ms"].as_u64();
    let start = time["start_ms"].as_u64();
    let end = time["end_ms"].as_u64();
    match (at, start, end) {
        (Some(_), None, None) => Ok(()),
        (None, Some(s), Some(e)) if e > s => Ok(()),
        _ => bail!(
            "TIME_RANGE at {path}: expected at_ms or half-open start_ms/end_ms with end > start"
        ),
    }
}
fn validate_canonical_index(mut index: Value) -> Result<()> {
    if index["version"] != "0.1.0" {
        bail!("INDEX_VERSION at /version");
    }
    let assets = index["assets"]
        .as_array()
        .context("INDEX_ASSETS at /assets")?;
    let artifacts = index["artifacts"]
        .as_array()
        .context("INDEX_ARTIFACTS at /artifacts")?;
    let observations = index["observations"]
        .as_array()
        .context("INDEX_OBSERVATIONS at /observations")?;
    let segments = index["segments"]
        .as_array()
        .context("INDEX_SEGMENTS at /segments")?;
    let (asset_count, artifact_count, observation_count) =
        (assets.len(), artifacts.len(), observations.len());
    let mut asset_ids = std::collections::HashSet::new();
    for asset in assets {
        let id = asset["id"].as_str().context("ASSET_ID at /assets/*/id")?;
        if !asset_ids.insert(id.to_string()) {
            bail!("DUPLICATE_ASSET_ID at /assets/*/id: {id}");
        }
    }
    let mut artifact_ids = std::collections::HashSet::new();
    for artifact in artifacts {
        let id = artifact["id"]
            .as_str()
            .context("ARTIFACT_ID at /artifacts/*/id")?;
        if !artifact_ids.insert(id.to_string()) {
            bail!("DUPLICATE_ARTIFACT_ID at /artifacts/*/id: {id}");
        }
        if let Some(parent) = artifact["parent_asset_id"].as_str()
            && !asset_ids.contains(parent)
        {
            bail!("BROKEN_ASSET_REF at /artifacts/*/parent_asset_id: {parent}");
        }
        validate_time(&artifact["time"], "/artifacts/*/time")?;
    }
    let mut obs_ids = std::collections::HashSet::new();
    for observation in observations {
        validate_observation(observation, 0)?;
        let id = observation["id"].as_str().unwrap();
        if !obs_ids.insert(id.to_string()) {
            bail!("DUPLICATE_OBSERVATION_ID at /observations/*/id: {id}");
        }
        let asset = observation["asset_id"].as_str().unwrap();
        if !asset_ids.contains(asset) {
            bail!("BROKEN_ASSET_REF at /observations/*/asset_id: {asset}");
        }
        for reference in observation["evidence_refs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !artifact_ids.contains(reference) {
                bail!("BROKEN_EVIDENCE_REF at /observations/*/evidence_refs: {reference}");
            }
        }
    }
    for segment in segments {
        let asset = segment["asset_id"]
            .as_str()
            .context("SEGMENT_ASSET_ID at /segments/*/asset_id")?;
        if !asset_ids.contains(asset) {
            bail!("BROKEN_ASSET_REF at /segments/*/asset_id: {asset}");
        }
        validate_time(&segment["time"], "/segments/*/time")?;
        for observation_id in segment["observation_ids"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !obs_ids.contains(observation_id) {
                bail!("BROKEN_OBSERVATION_REF at /segments/*/observation_ids: {observation_id}");
            }
        }
        for reference in segment["evidence_refs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !artifact_ids.contains(reference) {
                bail!("BROKEN_EVIDENCE_REF at /segments/*/evidence_refs: {reference}");
            }
        }
    }
    let expected = index["revision"]
        .as_str()
        .context("INDEX_REVISION at /revision")?
        .to_string();
    index.as_object_mut().unwrap().remove("revision");
    let actual = format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(&index)?));
    if expected != actual {
        bail!("INDEX_REVISION_MISMATCH at /revision: expected {expected}, computed {actual}");
    }
    println!(
        "valid canonical index: {} assets, {} artifacts, {} observations",
        asset_count, artifact_count, observation_count
    );
    Ok(())
}
fn synthesize(source: &Path, out: &Path) -> Result<()> {
    let data = fs::read_to_string(source)
        .with_context(|| format!("read observations {}", source.display()))?;
    let mut artifacts = Vec::new();
    let mut aliases = Vec::new();
    let mut observations = Vec::new();
    for (line_no, line) in data.lines().enumerate() {
        let value: Value = serde_json::from_str(line)
            .with_context(|| format!("invalid JSON at line {}", line_no + 1))?;
        match value["record_type"]
            .as_str()
            .context("record_type required")?
        {
            "artifact" => artifacts.push(value),
            "artifact_path" => aliases.push(value),
            "observation" => observations.push(normalize_observation(value)?),
            other => bail!("unsupported record_type at line {}: {other}", line_no + 1),
        }
    }
    for observation in &observations {
        validate_observation_refs_from_records(&artifacts, observation)?;
    }
    artifacts.sort_by_key(|x| x["artifact_id"].as_str().unwrap_or_default().to_string());
    observations.sort_by_key(|x| x["id"].as_str().unwrap_or_default().to_string());
    aliases.sort_by_key(|x| {
        (
            x["artifact_id"].as_str().unwrap_or_default().to_string(),
            x["path"].to_string(),
        )
    });
    let mut assets = Vec::new();
    let mut normalized_artifacts = Vec::new();
    for artifact in &artifacts {
        let id = artifact["artifact_id"].as_str().unwrap_or_default();
        let path = artifact["path"].clone();
        let kind = artifact["kind"].clone();
        let mut uris = vec![path.clone()];
        uris.extend(
            aliases
                .iter()
                .filter(|a| a["artifact_id"] == id)
                .map(|a| a["path"].clone()),
        );
        uris.sort_by_key(Value::to_string);
        uris.dedup();
        let is_asset = artifact["parent_asset_id"].is_null();
        if is_asset {
            assets.push(json!({"id":id,"uri":path,"alternate_uris":uris,"kind":kind,"size_bytes":artifact["size_bytes"],"sha256":artifact["sha256"]}));
        }
        let mut occurrences = vec![
            json!({"uri":path,"parent_asset_id":artifact["parent_asset_id"],"time":artifact["time"]}),
        ];
        occurrences.extend(aliases.iter().filter(|a| a["artifact_id"] == id).map(
            |a| json!({"uri":a["path"],"parent_asset_id":a["parent_asset_id"],"time":a["time"]}),
        ));
        normalized_artifacts.push(json!({"id":id,"uri":path,"kind":kind,"parent_asset_id":artifact["parent_asset_id"],"time":artifact["time"],"occurrences":occurrences,"mime_type":artifact["mime_type"],"size_bytes":artifact["size_bytes"],"sha256":artifact["sha256"]}));
    }
    assets.sort_by_key(|x| x["id"].as_str().unwrap_or_default().to_string());
    normalized_artifacts.sort_by_key(|x| x["id"].as_str().unwrap_or_default().to_string());
    let mut issues = find_conflicts(&observations);
    issues.sort_by_key(|x| x.to_string());
    let segments = synthesize_segments(&observations)?;
    let mut result = json!({"protocol":"filmmap.index","version":"0.1.0","status":"partial","assets":assets,"artifacts":normalized_artifacts,"observations":observations,"segments":segments,"coverage":[],"issues":issues,"policy_version":"filmmap-synthesis/v1"});
    let revision = format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(&result)?));
    result["revision"] = json!(revision);
    validate_canonical_index(result.clone())?;
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out, serde_json::to_string_pretty(&result)?)
        .with_context(|| format!("write index {}", out.display()))?;
    println!("{revision}");
    Ok(())
}
fn synthesize_segments(observations: &[Value]) -> Result<Vec<Value>> {
    let mut segments = Vec::new();
    for observation in observations {
        if observation["kind"] != "scene_description" {
            continue;
        }
        let Some(start_ms) = observation["time"]["start_ms"].as_u64() else {
            continue;
        };
        let Some(end_ms) = observation["time"]["end_ms"].as_u64() else {
            continue;
        };
        let body = json!({"asset_id":observation["asset_id"],"time":{"start_ms":start_ms,"end_ms":end_ms},"observation_ids":[observation["id"]]});
        let id = format!(
            "seg_sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&body)?)
        );
        let summary = observation["value"]["text"]
            .as_str()
            .map(str::to_string)
            .or_else(|| {
                let setting = observation["value"]["setting"].as_str().unwrap_or_default();
                let visible = observation["value"]["visible_elements"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                let combined = [setting, visible.as_str()]
                    .into_iter()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("; ");
                (!combined.is_empty()).then_some(combined)
            })
            .unwrap_or_default();
        segments.push(json!({"id":id,"asset_id":observation["asset_id"],"kind":"scene_description","time":{"start_ms":start_ms,"end_ms":end_ms},"summary":summary,"observation_ids":[observation["id"]],"evidence_refs":observation["evidence_refs"],"status":observation["status"],"confidence":observation["confidence"]}));
    }
    segments.sort_by_key(|s| {
        (
            s["asset_id"].as_str().unwrap_or_default().to_string(),
            s["time"]["start_ms"].as_u64().unwrap_or_default(),
            s["id"].to_string(),
        )
    });
    Ok(segments)
}
fn normalize_observation(mut value: Value) -> Result<Value> {
    if value["protocol"] == "filmmap.observation" {
        validate_observation(&value, 0)?;
        return Ok(value);
    }
    let asset_id = value["artifact_id"]
        .as_str()
        .context("legacy observation missing artifact_id")?
        .to_string();
    let kind = value["kind"]
        .as_str()
        .or(value["capability"].as_str())
        .context("observation kind required")?
        .to_string();
    let time = if let Some(ms) = value["time_seconds"].as_f64() {
        json!({"at_ms":(ms*1000.0).round() as u64})
    } else {
        Value::Null
    };
    let text = value["value"].as_str().unwrap_or_default().to_string();
    let source = value["source"]["kind"]
        .as_str()
        .or(value["source"].as_str())
        .unwrap_or("legacy-import")
        .to_string();
    let class = "interpretation";
    let mut normalized = json!({"record_type":"observation","protocol":"filmmap.observation","version":"0.1.0","asset_id":asset_id,"class":class,"kind":kind,"time":time,"value":{"text":text},"producer":{"type":"agent","id":source},"evidence_refs":[],"evidence_state":"none","status":"candidate","confidence":value["confidence"],"created_at_unix_ms":0});
    let digest = Sha256::digest(serde_json::to_vec(&normalized)?);
    normalized["id"] = json!(format!("obs_sha256:{digest:x}"));
    value = normalized;
    Ok(value)
}
fn validate_observation_refs_from_records(artifacts: &[Value], observation: &Value) -> Result<()> {
    validate_observation(observation, 0)?;
    let asset_id = observation["asset_id"].as_str().unwrap_or_default();
    let ids: std::collections::HashSet<String> = artifacts
        .iter()
        .filter_map(|a| a["artifact_id"].as_str().map(str::to_string))
        .collect();
    if !ids.contains(asset_id) {
        bail!("BROKEN_ASSET_REF: {}", asset_id);
    }
    for reference in observation["evidence_refs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !ids.contains(reference) {
            bail!("BROKEN_EVIDENCE_REF: {reference}");
        }
    }
    Ok(())
}
fn find_conflicts(observations: &[Value]) -> Vec<Value> {
    let mut issues = Vec::new();
    for (i, left) in observations.iter().enumerate() {
        for right in observations.iter().skip(i + 1) {
            if left["asset_id"] != right["asset_id"]
                || left["kind"] != right["kind"]
                || left["value"] == right["value"]
            {
                continue;
            }
            let overlaps = match (
                left["time"]["at_ms"].as_u64(),
                right["time"]["at_ms"].as_u64(),
            ) {
                (Some(a), Some(b)) => a == b,
                _ => left["time"]["start_ms"]
                    .as_u64()
                    .zip(left["time"]["end_ms"].as_u64())
                    .zip(
                        right["time"]["start_ms"]
                            .as_u64()
                            .zip(right["time"]["end_ms"].as_u64()),
                    )
                    .is_some_and(|((a, b), (c, d))| a < d && c < b),
            };
            if overlaps {
                issues.push(json!({"code":"OBSERVATION_CONFLICT","severity":"warning","asset_id":left["asset_id"],"kind":left["kind"],"observation_ids":[left["id"],right["id"]]}));
            }
        }
    }
    issues
}
struct QueryOptions {
    from: Option<f64>,
    to: Option<f64>,
    from_ms: Option<u64>,
    to_ms: Option<u64>,
    asset_id: Option<String>,
    kind: Option<String>,
}
fn query_index(index: &Path, q: &str, options: QueryOptions) -> Result<()> {
    let mut found = Vec::new();
    let data = fs::read_to_string(index)?;
    let canonical = serde_json::from_str::<Value>(&data)
        .ok()
        .filter(|v| v["protocol"] == "filmmap.index");
    let records: Vec<Value> = if let Some(index) = canonical {
        let mut records = index["observations"]
            .as_array()
            .cloned()
            .context("canonical index observations must be an array")?;
        records.extend(
            index["segments"]
                .as_array()
                .cloned()
                .context("canonical index segments must be an array")?,
        );
        records
    } else {
        data.lines()
            .map(serde_json::from_str)
            .collect::<std::result::Result<Vec<Value>, _>>()?
    };
    let lower = q.to_lowercase();
    let min_ms = options
        .from_ms
        .or_else(|| options.from.map(|s| (s.max(0.0) * 1000.0).round() as u64));
    let max_ms = options
        .to_ms
        .or_else(|| options.to.map(|s| (s.max(0.0) * 1000.0).round() as u64));
    for v in records {
        let text = v.to_string().to_lowercase();
        if !text.contains(&lower) {
            continue;
        }
        let asset = v["asset_id"].as_str().or(v["artifact_id"].as_str());
        if options
            .asset_id
            .as_deref()
            .is_some_and(|wanted| asset != Some(wanted))
        {
            continue;
        }
        let record_kind = v["kind"].as_str().or(v["capability"].as_str());
        if options
            .kind
            .as_deref()
            .is_some_and(|wanted| record_kind != Some(wanted))
        {
            continue;
        }
        let (point, start, end) = if v["time"].is_object() {
            (
                v["time"]["at_ms"].as_u64(),
                v["time"]["start_ms"].as_u64(),
                v["time"]["end_ms"].as_u64(),
            )
        } else {
            (
                v["time_seconds"]
                    .as_f64()
                    .map(|s| (s * 1000.0).round() as u64),
                None,
                None,
            )
        };
        if (min_ms.is_some() || max_ms.is_some()) && point.is_none() && start.is_none() {
            continue;
        }
        if let (Some(s), Some(e)) = (start, end)
            && (min_ms.is_some_and(|min| e <= min) || max_ms.is_some_and(|max| s > max))
        {
            continue;
        }
        if point.is_some_and(|t| min_ms.is_some_and(|m| t < m) || max_ms.is_some_and(|m| t > m)) {
            continue;
        }
        found.push(v);
    }
    println!("{}", serde_json::to_string_pretty(&found)?);
    Ok(())
}
fn install_text(dir: PathBuf, name: &str, text: &str) -> Result<()> {
    fs::create_dir_all(&dir)?;
    let dst = dir.join(name);
    if dst.exists() {
        let old = fs::read_to_string(&dst)?;
        if old == text {
            println!("already installed: {}", dst.display());
            return Ok(());
        }
        bail!("refusing to overwrite different file: {}", dst.display());
    }
    fs::write(&dst, text)?;
    println!("installed {}", dst.display());
    Ok(())
}
fn install_tools(dir: PathBuf) -> Result<()> {
    fs::create_dir_all(&dir)?;
    let entries: Value = serde_json::from_str(TOOL_MANIFEST)?;
    for entry in entries
        .as_array()
        .context("tool manifest must be an array")?
    {
        let name = entry["name"]
            .as_str()
            .context("tool name must be a string")?;
        let source = entry["path"]
            .as_str()
            .context("tool path must be a string")?;
        if Path::new(name).file_name().and_then(|value| value.to_str()) != Some(name) {
            bail!("tool name must be a plain filename: {name}");
        }
        let content = embedded_tool(source)?;
        let dst = dir.join(name);
        if dst.exists() {
            let same = fs::read(&dst)? == content.as_bytes();
            if !same {
                bail!("refusing to overwrite different file: {}", dst.display());
            }
        } else {
            fs::write(&dst, content)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut p = fs::metadata(&dst)?.permissions();
                p.set_mode(0o755);
                fs::set_permissions(&dst, p)?;
            }
        }
    }
    println!("tools installed in {}", dir.display());
    Ok(())
}
fn embedded_tool(path: &str) -> Result<&'static str> {
    match path {
        "tools/filmmap-probe.sh" => Ok(include_str!("../tools/filmmap-probe.sh")),
        "tools/filmmap-frames.sh" => Ok(include_str!("../tools/filmmap-frames.sh")),
        "tools/filmmap-scan.sh" => Ok(include_str!("../tools/filmmap-scan.sh")),
        _ => bail!("tool is not embedded in this CLI build: {path}"),
    }
}
fn default_skill_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".agents/skills/filmmap")
}
fn default_tools_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/filmmap/tools")
}

#[cfg(test)]
mod e2e {
    use super::*;
    #[test]
    fn cli_workspace_artifact_observation_validate_query_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("project");
        init(root.clone()).unwrap();
        let media = d.path().join("clip.bin");
        fs::write(&media, b"test media").unwrap();
        add_artifact(&root.join("index.jsonl"), &media, "video", None, None, None).unwrap();
        let id = format!("sha256:{:x}", Sha256::digest(b"test media"));
        append_jsonl(
            &root.join("index.jsonl"),
            &json!({"record_type":"observation","artifact_id":id,"capability":"image.describe","time_seconds":3.0,"value":"mountain landscape","source":{"kind":"manual"},"confidence":0.91}),
        ).unwrap();
        validate_index(&root.join("index.jsonl")).unwrap();
        query_index(
            &root.join("index.jsonl"),
            "mountain",
            QueryOptions {
                from: Some(2.0),
                to: Some(4.0),
                from_ms: None,
                to_ms: None,
                asset_id: None,
                kind: None,
            },
        )
        .unwrap();
    }
}
