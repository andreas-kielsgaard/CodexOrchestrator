//! Private, retained authority for a completed Handler acceptance.  It deliberately does not
//! integrate the candidate or advance the Sprint target.

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(crate) const ACCEPTED_CANDIDATE_AUTHORITY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS accepted_handler_candidates (
  candidate_id TEXT PRIMARY KEY,
  work_unit_id TEXT NOT NULL UNIQUE REFERENCES work_units(work_unit_id) ON DELETE RESTRICT,
  sprint_id TEXT NOT NULL REFERENCES initiated_sprints(id) ON DELETE RESTRICT,
  authority_id TEXT NOT NULL REFERENCES initiated_sprint_git_authorities(authority_id) ON DELETE RESTRICT,
  attempt_id TEXT NOT NULL,
  reporting_invocation_id TEXT NOT NULL UNIQUE,
  review_invocation_id TEXT NOT NULL UNIQUE REFERENCES work_unit_handler_reviews(review_invocation_id) ON DELETE RESTRICT,
  decision_fingerprint TEXT NOT NULL UNIQUE REFERENCES work_unit_handler_decisions(decision_fingerprint) ON DELETE RESTRICT,
  capture_authorization_id TEXT NOT NULL UNIQUE REFERENCES file_review_git_capture_authorizations(capture_authorization_id) ON DELETE RESTRICT,
  attempt_baseline_object_id TEXT,
  candidate_commit_id TEXT NOT NULL,
  candidate_tree_id TEXT NOT NULL,
  private_ref_name TEXT NOT NULL UNIQUE,
  evidence_fingerprint TEXT NOT NULL,
  intent_recorded_at TEXT NOT NULL,
  pinned_at TEXT,
  attention_reason TEXT,
  attention_recorded_at TEXT,
  CHECK ((pinned_at IS NULL) OR attention_reason IS NULL)
);
CREATE TABLE IF NOT EXISTS sprint_target_currents (
  authority_id TEXT PRIMARY KEY REFERENCES initiated_sprint_git_authorities(authority_id) ON DELETE RESTRICT,
  sprint_id TEXT NOT NULL UNIQUE REFERENCES initiated_sprints(id) ON DELETE RESTRICT,
  target_ref_name TEXT NOT NULL,
  current_object_id TEXT NOT NULL,
  binding_fingerprint TEXT NOT NULL UNIQUE,
  version INTEGER NOT NULL DEFAULT 1,
  initialized_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  attention_reason TEXT,
  attention_recorded_at TEXT,
  CHECK ((attention_reason IS NULL) = (attention_recorded_at IS NULL))
);
CREATE TABLE IF NOT EXISTS accepted_candidate_authority_attentions (
  candidate_id TEXT PRIMARY KEY,
  attention_reason TEXT NOT NULL,
  recorded_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS sprint_target_current_attentions (
  authority_id TEXT PRIMARY KEY REFERENCES initiated_sprint_git_authorities(authority_id) ON DELETE RESTRICT,
  attention_reason TEXT NOT NULL,
  recorded_at TEXT NOT NULL
);
"#;

#[derive(Clone)]
struct CandidateRow {
    work_unit_id: String,
    sprint_id: String,
    authority_id: String,
    attempt_id: String,
    reporting: String,
    review: String,
    decision: String,
    delivered_evidence: String,
    manifest: String,
    comparison: String,
    contents: String,
    repo: String,
    common: String,
    authority_root: String,
    baseline: String,
    authority_current: String,
    capture_id: String,
    document_id: String,
    artifact_id: String,
    capture_root: String,
    capture_commit: String,
}

/// Reconciles only accepted Handler decisions.  Every input is loaded from durable application
/// records; Git routes, object ids, and ref names are never caller inputs.
pub(crate) fn reconcile_accepted_candidate_authorities(
    connection: &mut Connection,
) -> Result<(), String> {
    initialize_accepted_candidate_authority_schema(connection)?;
    let rows = connection.prepare(
        "SELECT d.work_unit_id,a.sprint_id,a.authority_id,o.attempt_id,o.reporting_invocation_id,
                d.review_invocation_id,d.decision_fingerprint,r.delivered_payload_fingerprint,o.evidence_manifest_json,
                o.comparison_fingerprint,o.evidence_content_fingerprints_json,
                a.repository_root,a.repository_common_dir,a.worktree_root,x.baseline_object_id,a.current_object_id,
                c.capture_authorization_id,l.document_ref_id,l.artifact_id,c.worktree_root,c.current_object_id
         FROM work_unit_handler_decisions d
         JOIN work_unit_handler_reviews r ON r.review_invocation_id=d.review_invocation_id
         JOIN work_unit_implementer_outcomes o ON o.work_unit_id=d.work_unit_id
           AND o.reporting_invocation_id=r.reporting_invocation_id
         JOIN work_unit_handler_activations h ON h.work_unit_id=d.work_unit_id AND h.attempt_id=o.attempt_id
         JOIN work_unit_materializations m ON m.materialization_id=h.materialization_id
         JOIN initiated_sprint_git_authorities a ON a.sprint_id=m.sprint_id
         JOIN execution_support_attempt_authorizations x ON x.attempt_id=o.attempt_id AND x.work_unit_id=d.work_unit_id AND x.role_kind='work_unit_implementer' AND x.sprint_git_authority_id=a.authority_id
         JOIN execution_support_grants g ON g.attempt_id=o.attempt_id AND g.role_id='work_unit_implementer'
         JOIN file_review_git_capture_authorizations c ON c.capture_authorization_id=o.file_review_capture_authorization_id AND c.worktree_id=g.workspace_id AND c.repository_id=a.repository_id AND c.baseline_object_id=x.baseline_object_id
         JOIN file_review_git_capture_documents l ON l.capture_authorization_id=c.capture_authorization_id
         WHERE d.decision_variant='accepted' AND d.implementation_accepted_at IS NOT NULL
           AND r.lifecycle_status='completed' AND r.semantic_judgment_variant='accept'
           AND o.evidence_ready_at IS NOT NULL AND o.application_accepted_at IS NOT NULL
         ORDER BY d.work_unit_id"
    ).map_err(|e| e.to_string())?
    .query_map([], |r| Ok(CandidateRow { work_unit_id:r.get(0)?, sprint_id:r.get(1)?, authority_id:r.get(2)?, attempt_id:r.get(3)?, reporting:r.get(4)?, review:r.get(5)?, decision:r.get(6)?,delivered_evidence:r.get(7)?, manifest:r.get(8)?, comparison:r.get(9)?, contents:r.get(10)?, repo:r.get(11)?, common:r.get(12)?, authority_root:r.get(13)?, baseline:r.get(14)?, authority_current:r.get(15)?, capture_id:r.get(16)?, document_id:r.get(17)?,artifact_id:r.get(18)?,capture_root:r.get(19)?, capture_commit:r.get(20)? }))
    .map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    for row in rows {
        reconcile_candidate(connection, row)?;
    }
    Ok(())
}

pub(crate) fn initialize_accepted_candidate_authority_schema(
    connection: &Connection,
) -> Result<(), String> {
    connection.execute_batch(ACCEPTED_CANDIDATE_AUTHORITY_SCHEMA).map_err(|e| e.to_string())?;
    let has_attempt_baseline = connection.prepare("PRAGMA table_info(accepted_handler_candidates)").and_then(|mut statement| statement.query_map([], |row| row.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>()).map_err(|error| error.to_string())?.iter().any(|column| column == "attempt_baseline_object_id");
    if !has_attempt_baseline { connection.execute("ALTER TABLE accepted_handler_candidates ADD COLUMN attempt_baseline_object_id TEXT", []).map_err(|error| format!("Unable to migrate accepted candidate baseline: {error}"))?; }
    Ok(())
}

/// The only binding contract for a mutable Sprint target current.
pub(crate) fn sprint_target_binding_fingerprint(
    authority_id: &str,
    target_ref_name: &str,
    current_object_id: &str,
) -> String {
    fingerprint(&[authority_id, target_ref_name, current_object_id])
}

/// Application-only retained-candidate gate used by the integration drain.  It deliberately
/// reruns the same durable review/evidence/capture checks as retention; no attempt worktree is
/// consulted after the candidate was pinned.
pub(crate) fn revalidate_retained_accepted_candidate(
    connection: &mut Connection,
    candidate_id: &str,
) -> Result<(), String> {
    let has_decisions: bool = connection
        .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='work_unit_handler_decisions')", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if has_decisions {
        reconcile_accepted_candidate_authorities(connection)?;
    }
    let row: Option<(Option<String>, Option<String>)> = connection
        .query_row(
            "SELECT pinned_at,attention_reason FROM accepted_handler_candidates WHERE candidate_id=?1",
            [candidate_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())?;
    match row {
        Some((Some(_), None)) => Ok(()),
        Some((_, Some(reason))) => Err(format!("candidate_revalidation:{reason}")),
        _ => Err("candidate_revalidation:not_retained".into()),
    }
}

fn reconcile_candidate(connection: &mut Connection, row: CandidateRow) -> Result<(), String> {
    let candidate_id = stable_id("accepted-handler-candidate", &row.decision);
    connection.execute("UPDATE accepted_handler_candidates SET attempt_baseline_object_id=COALESCE(attempt_baseline_object_id,?2) WHERE candidate_id=?1",params![candidate_id,row.baseline]).map_err(|e|e.to_string())?;
    let private_ref = format!("refs/codex/orchestrator/accepted/{candidate_id}");
    let evidence = fingerprint(&[
        &row.work_unit_id,
        &row.attempt_id,
        &row.authority_id,
        &row.baseline,
        &row.capture_commit,
        &row.reporting,
        &row.review,
        &row.decision,
        &row.delivered_evidence,
        &row.document_id,
        &row.artifact_id,
        &row.manifest,
        &row.comparison,
        &row.contents,
        &row.capture_id,
    ]);
    let retained:Option<(String,String,String,Option<String>)>=connection.query_row("SELECT candidate_commit_id,candidate_tree_id,evidence_fingerprint,pinned_at FROM accepted_handler_candidates WHERE candidate_id=?1",[&candidate_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|e|e.to_string())?;
    if let Some((commit, tree, stored, Some(_))) = retained {
        if commit != row.capture_commit || stored != evidence {
            return record_attention(
                connection,
                &candidate_id,
                "retained_durable_lineage_mismatch",
            );
        }
        if let Err(reason) = validate_durable_lineage(connection, &row) {
            return record_attention(connection, &candidate_id, &reason);
        }
        let repository = PathBuf::from(&row.repo);
        let actual = git(
            &repository,
            &["show-ref", "--verify", "--hash", &private_ref],
        );
        if actual.as_deref() != Ok(commit.as_str()) {
            return record_attention(connection, &candidate_id, "private_ref_divergent");
        }
        if git(
            &repository,
            &["rev-parse", "--verify", &format!("{commit}^{{tree}}")],
        )
        .as_deref()
            != Ok(tree.as_str())
        {
            return record_attention(
                connection,
                &candidate_id,
                "retained_candidate_tree_mismatch",
            );
        }
        return initialize_target(connection, &row);
    }
    let valid = validate_candidate(connection, &row).and_then(|tree| {
        let ref_value = git(
            &PathBuf::from(&row.capture_root),
            &["show-ref", "--verify", "--hash", &private_ref],
        );
        match ref_value {
            Ok(value) if value == row.capture_commit => Ok(tree),
            Ok(_) => Err("private_ref_divergent".into()),
            Err(_) => Ok(tree),
        }
    });
    let tree = match valid {
        Ok(tree) => tree,
        Err(reason) => return record_attention(connection, &candidate_id, &reason),
    };
    let existing: Option<(String,String,String,String,String,String,String)> = connection.query_row(
        "SELECT work_unit_id,authority_id,reporting_invocation_id,review_invocation_id,decision_fingerprint,candidate_commit_id,evidence_fingerprint FROM accepted_handler_candidates WHERE candidate_id=?1 OR work_unit_id=?2",
        params![candidate_id,row.work_unit_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional().map_err(|e|e.to_string())?;
    if let Some(existing) = existing {
        if existing
            != (
                row.work_unit_id.clone(),
                row.authority_id.clone(),
                row.reporting.clone(),
                row.review.clone(),
                row.decision.clone(),
                row.capture_commit.clone(),
                evidence.clone(),
            )
        {
            return record_attention(connection, &candidate_id, "durable_candidate_conflict");
        }
    } else {
        let now = chrono::Utc::now().to_rfc3339();
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO accepted_handler_candidates (candidate_id,work_unit_id,sprint_id,authority_id,attempt_id,reporting_invocation_id,review_invocation_id,decision_fingerprint,capture_authorization_id,attempt_baseline_object_id,candidate_commit_id,candidate_tree_id,private_ref_name,evidence_fingerprint,intent_recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",params![candidate_id,row.work_unit_id,row.sprint_id,row.authority_id,row.attempt_id,row.reporting,row.review,row.decision,row.capture_id,row.baseline,row.capture_commit,tree,private_ref,evidence,now]).map_err(|e|e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    // Database intent precedes the ref effect.  `update-ref <new> <old>` is absent-or-exact.
    let root = PathBuf::from(&row.capture_root);
    let pinned = git(
        &root,
        &["update-ref", &private_ref, &row.capture_commit, ""],
    )
    .or_else(|_| {
        git(&root, &["show-ref", "--verify", "--hash", &private_ref]).and_then(|actual| {
            if actual == row.capture_commit {
                Ok(actual)
            } else {
                Err("private_ref_divergent".into())
            }
        })
    });
    match pinned {
        Ok(_) => {
            connection.execute("UPDATE accepted_handler_candidates SET pinned_at=COALESCE(pinned_at,?2) WHERE candidate_id=?1",params![candidate_id,chrono::Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
            initialize_target(connection, &row)
        }
        Err(reason) => record_attention(connection, &candidate_id, &reason),
    }
}

fn validate_candidate(connection: &Connection, row: &CandidateRow) -> Result<String, String> {
    let root = PathBuf::from(&row.capture_root);
    let repo = PathBuf::from(&row.repo);
    if !root.is_dir() || git(&root, &["status", "--porcelain"])? != "" {
        return Err("candidate_worktree_dirty".into());
    }
    if git(&root, &["rev-parse", "--show-toplevel"])? != canonical(&root)?
        || git(&repo, &["rev-parse", "--show-toplevel"])? != canonical(&repo)?
    {
        return Err("repository_root_drift".into());
    }
    if git(&root, &["rev-parse", "--git-common-dir"])? != canonical(&PathBuf::from(&row.common))?
        || git(&repo, &["rev-parse", "--git-common-dir"])?
            != canonical(&PathBuf::from(&row.common))?
    {
        return Err("repository_common_dir_drift".into());
    }
    if git(&root, &["rev-parse", "--verify", "HEAD^{commit}"])? != row.capture_commit {
        return Err("candidate_head_drift".into());
    }
    if git(
        &root,
        &[
            "merge-base",
            "--is-ancestor",
            &row.baseline,
            &row.capture_commit,
        ],
    )
    .is_err()
    {
        return Err("candidate_not_descended_from_baseline".into());
    }
    let tree = git(
        &root,
        &[
            "rev-parse",
            "--verify",
            &format!("{}^{{tree}}", row.capture_commit),
        ],
    )?;
    validate_durable_lineage(connection, row)?;
    Ok(tree)
}

/// Revalidates every durable File Review/outcome/review/decision linkage without reading an
/// attempt workspace. Retained reopen uses this after the private object check.
fn validate_durable_lineage(connection: &Connection, row: &CandidateRow) -> Result<(), String> {
    let direct:Option<i64>=connection.query_row("SELECT 1 FROM file_review_git_capture_documents l JOIN file_review_git_capture_authorizations c ON c.capture_authorization_id=l.capture_authorization_id WHERE l.capture_authorization_id=?1 AND l.document_ref_id=?2 AND l.artifact_id=?3 AND c.current_object_id=?4 AND c.baseline_object_id=?5",params![row.capture_id,row.document_id,row.artifact_id,row.capture_commit,row.baseline],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    if direct.is_none() {
        return Err("capture_document_artifact_mismatch".into());
    }
    let payload:Vec<u8>=connection.query_row("SELECT payload FROM stored_file_review_artifacts WHERE artifact_id=?1 AND document_ref_id=?2",params![row.artifact_id,row.document_id],|r|r.get(0)).map_err(|_|"capture_document_artifact_mismatch".to_string())?;
    if fingerprint_bytes("implementer-evidence-comparison", &payload) != row.comparison {
        return Err("comparison_fingerprint_mismatch".into());
    }
    let db_manifest=connection.prepare("SELECT changed_file_reference_id,display_name,change_kind FROM file_review_changed_files WHERE document_ref_id=?1 ORDER BY ordinal").map_err(|e|e.to_string())?.query_map([&row.document_id],|r|Ok(serde_json::json!({"evidenceRef":r.get::<_,String>(0)?,"displayName":r.get::<_,String>(1)?,"changeKind":r.get::<_,String>(2)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    if serde_json::to_string(&db_manifest).map_err(|e| e.to_string())? != row.manifest {
        return Err("manifest_metadata_mismatch".into());
    }
    let artifact: serde_json::Value =
        serde_json::from_slice(&payload).map_err(|_| "artifact_invalid".to_string())?;
    let mut actual_contents = Vec::new();
    for entry in db_manifest {
        let reference = entry
            .get("evidenceRef")
            .and_then(|v| v.as_str())
            .ok_or("manifest_metadata_mismatch")?;
        let file = artifact
            .get("files")
            .and_then(|v| v.as_array())
            .and_then(|files| {
                files.iter().find(|file| {
                    file.get("changedFileReferenceId").and_then(|v| v.as_str()) == Some(reference)
                })
            })
            .ok_or("artifact_membership_mismatch")?;
        actual_contents.push(serde_json::json!({"evidenceRef":reference,"contentFingerprint":fingerprint_bytes("implementer-evidence-content",&serde_json::to_vec(file).map_err(|e|e.to_string())?)}));
    }
    actual_contents.sort_by(|a, b| a["evidenceRef"].as_str().cmp(&b["evidenceRef"].as_str()));
    if serde_json::to_string(&actual_contents).map_err(|e| e.to_string())? != row.contents {
        return Err("evidence_content_fingerprint_mismatch".into());
    }
    Ok(())
}

fn initialize_target(connection: &mut Connection, row: &CandidateRow) -> Result<(), String> {
    let existing:Option<(String,String,String,String,i64,String,String)>=connection.query_row("SELECT sprint_id,target_ref_name,current_object_id,binding_fingerprint,version,initialized_at,updated_at FROM sprint_target_currents WHERE authority_id=?1",[&row.authority_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional().map_err(|e|e.to_string())?;
    if let Some((sprint, reference, current, binding, version, initialized, updated)) = existing {
        let exact = sprint == row.sprint_id
            && safe_ref(&reference)
            && git_object(&current)
            && version >= 1
            && initialized <= updated
            && binding == sprint_target_binding_fingerprint(&row.authority_id, &reference, &current);
        return if exact {
            Ok(())
        } else {
            record_target_attention(
                connection,
                &row.authority_id,
                "target_current_durable_mismatch",
            )
        };
    }
    let root = PathBuf::from(&row.repo);
    let worktree = PathBuf::from(&row.authority_root);
    let ref_name = match git(&worktree, &["symbolic-ref", "--quiet", "HEAD"]) {
        Ok(v) if safe_ref(&v) => v,
        _ => {
            return record_target_attention(
                connection,
                &row.authority_id,
                "target_ref_detached_or_unsafe",
            )
        }
    };
    if git(&worktree, &["status", "--porcelain"])? != "" {
        return record_target_attention(connection, &row.authority_id, "target_worktree_dirty");
    };
    let current = git(&worktree, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    if git(&worktree, &["rev-parse", "--show-toplevel"])? != canonical(&worktree)?
        || git(
            &worktree,
            &["rev-parse", "--verify", &format!("{ref_name}^{{commit}}")],
        )? != current
        || current != row.authority_current
        || git(&worktree, &["rev-parse", "--git-common-dir"])?
            != canonical(&PathBuf::from(&row.common))?
    {
        return record_target_attention(connection, &row.authority_id, "target_worktree_drift");
    }
    let fingerprint = sprint_target_binding_fingerprint(&row.authority_id, &ref_name, &current);
    let now = chrono::Utc::now().to_rfc3339();
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO sprint_target_currents(authority_id,sprint_id,target_ref_name,current_object_id,binding_fingerprint,version,initialized_at,updated_at) VALUES(?1,?2,?3,?4,?5,1,?6,?6)",params![row.authority_id,row.sprint_id,ref_name,current,fingerprint,now]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    connection
        .execute(
            "DELETE FROM sprint_target_current_attentions WHERE authority_id=?1",
            [&row.authority_id],
        )
        .map_err(|e| e.to_string())?;
    let _ = root;
    Ok(())
}

fn record_attention(
    connection: &mut Connection,
    candidate_id: &str,
    reason: &str,
) -> Result<(), String> {
    connection.execute("INSERT INTO accepted_candidate_authority_attentions(candidate_id,attention_reason,recorded_at) VALUES(?1,?2,?3) ON CONFLICT(candidate_id) DO UPDATE SET attention_reason=excluded.attention_reason",params![candidate_id,reason,chrono::Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
    connection.execute("UPDATE accepted_handler_candidates SET attention_reason=COALESCE(attention_reason,?2),attention_recorded_at=COALESCE(attention_recorded_at,?3) WHERE candidate_id=?1",params![candidate_id,reason,chrono::Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
    Ok(())
}
fn record_target_attention(
    connection: &mut Connection,
    authority_id: &str,
    reason: &str,
) -> Result<(), String> {
    connection.execute("INSERT INTO sprint_target_current_attentions(authority_id,attention_reason,recorded_at) VALUES(?1,?2,?3) ON CONFLICT(authority_id) DO UPDATE SET attention_reason=excluded.attention_reason",params![authority_id,reason,chrono::Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
    Ok(())
}
fn canonical(path: &Path) -> Result<String, String> {
    path.canonicalize()
        .map_err(|_| "path_unavailable".to_string())
        .map(|p| {
            p.to_string_lossy()
                .trim_end_matches(['\\', '/'])
                .to_string()
        })
}
fn safe_ref(value: &str) -> bool {
    value.starts_with("refs/")
        && value.len() <= 512
        && !value.contains("..")
        && !value.contains(' ')
        && !value.ends_with('/')
}
fn git_object(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .args(["--no-pager", "--no-replace-objects"])
        .args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .output()
        .map_err(|_| "git_unavailable".to_string())?;
    if !output.status.success() {
        return Err("git_validation_failed".into());
    }
    String::from_utf8(output.stdout)
        .map_err(|_| "git_invalid_output".into())
        .map(|v| v.trim().to_string())
}
fn stable_id(prefix: &str, value: &str) -> String {
    format!("{prefix}-{}", fingerprint(&[prefix, value]))
}
fn fingerprint(parts: &[&str]) -> String {
    let mut h = Sha256::new();
    for part in parts {
        h.update((part.len() as u64).to_be_bytes());
        h.update(part.as_bytes())
    }
    format!("{:x}", h.finalize())
}
fn fingerprint_bytes(prefix: &str, value: &[u8]) -> String {
    stable_id(
        prefix,
        &value
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(payload: &[u8]) -> CandidateRow {
        let artifact: serde_json::Value = serde_json::from_slice(payload).unwrap();
        let file = &artifact["files"][0];
        let manifest=serde_json::json!([{"evidenceRef":"e1","displayName":"one.rs","changeKind":"modified"}]).to_string();
        let contents=serde_json::json!([{"evidenceRef":"e1","contentFingerprint":fingerprint_bytes("implementer-evidence-content",&serde_json::to_vec(file).unwrap())}]).to_string();
        CandidateRow {
            work_unit_id: "unit".into(),
            sprint_id: "sprint".into(),
            authority_id: "authority".into(),
            attempt_id: "attempt".into(),
            reporting: "report".into(),
            review: "review".into(),
            decision: "decision".into(),
            delivered_evidence: "delivery".into(),
            manifest,
            comparison: fingerprint_bytes("implementer-evidence-comparison", payload),
            contents,
            repo: "x".into(),
            common: "x".into(),
            authority_root: "x".into(),
            baseline: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            authority_current: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            capture_id: "capture".into(),
            document_id: "document".into(),
            artifact_id: "artifact".into(),
            capture_root: "x".into(),
            capture_commit: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        }
    }

    fn durable_connection(payload: &[u8]) -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch("CREATE TABLE initiated_sprints(id TEXT PRIMARY KEY);CREATE TABLE initiated_sprint_git_authorities(authority_id TEXT PRIMARY KEY);CREATE TABLE work_units(work_unit_id TEXT PRIMARY KEY);CREATE TABLE work_unit_handler_reviews(review_invocation_id TEXT PRIMARY KEY);CREATE TABLE work_unit_handler_decisions(decision_fingerprint TEXT PRIMARY KEY);CREATE TABLE file_review_git_capture_authorizations(capture_authorization_id TEXT PRIMARY KEY,current_object_id TEXT,baseline_object_id TEXT);CREATE TABLE file_review_git_capture_documents(capture_authorization_id TEXT,document_ref_id TEXT,artifact_id TEXT);CREATE TABLE stored_file_review_artifacts(artifact_id TEXT,document_ref_id TEXT,payload BLOB);CREATE TABLE file_review_changed_files(document_ref_id TEXT,changed_file_reference_id TEXT,display_name TEXT,change_kind TEXT,ordinal INTEGER);").unwrap();
        connection
            .execute("INSERT INTO initiated_sprints VALUES('sprint')", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO initiated_sprint_git_authorities VALUES('authority')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO file_review_git_capture_authorizations VALUES('capture',?1,?2)",
                params![
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                ],
            )
            .unwrap();
        connection.execute("INSERT INTO file_review_git_capture_documents VALUES('capture','document','artifact')",[]).unwrap();
        connection
            .execute(
                "INSERT INTO stored_file_review_artifacts VALUES('artifact','document',?1)",
                [payload],
            )
            .unwrap();
        connection.execute("INSERT INTO file_review_changed_files VALUES('document','e1','one.rs','modified',0)",[]).unwrap();
        connection
    }

    #[test]
    fn durable_lineage_requires_exact_capture_link_and_content_fingerprints() {
        let payload=br#"{"files":[{"changedFileReferenceId":"e1","content":{"encoding":"base64","bytesBase64":"eA=="}}]}"#;
        let connection = durable_connection(payload);
        let row = row(payload);
        assert!(validate_durable_lineage(&connection, &row).is_ok());
        let mut tampered = row.clone();
        tampered.contents = "[]".into();
        assert_eq!(
            validate_durable_lineage(&connection, &tampered),
            Err("evidence_content_fingerprint_mismatch".into())
        );
        connection
            .execute("DELETE FROM file_review_git_capture_documents", [])
            .unwrap();
        assert_eq!(
            validate_durable_lineage(&connection, &row),
            Err("capture_document_artifact_mismatch".into())
        );
    }

    #[test]
    fn existing_mutable_target_requires_consistent_fingerprint() {
        let payload=br#"{"files":[{"changedFileReferenceId":"e1","content":{"encoding":"base64","bytesBase64":"eA=="}}]}"#;
        let mut connection = durable_connection(payload);
        connection
            .execute_batch(ACCEPTED_CANDIDATE_AUTHORITY_SCHEMA)
            .unwrap();
        let mut row = row(payload);
        row.sprint_id = "sprint".into();
        let binding = fingerprint(&[
            "authority",
            "refs/heads/main",
            "cccccccccccccccccccccccccccccccccccccccc",
        ]);
        connection.execute("INSERT INTO sprint_target_currents(authority_id,sprint_id,target_ref_name,current_object_id,binding_fingerprint,version,initialized_at,updated_at) VALUES('authority','sprint','refs/heads/main','cccccccccccccccccccccccccccccccccccccccc',?1,2,'2026-01-01T00:00:00Z','2026-01-02T00:00:00Z')",[binding]).unwrap();
        assert!(initialize_target(&mut connection, &row).is_ok());
        connection
            .execute(
                "UPDATE sprint_target_currents SET version=0 WHERE authority_id='authority'",
                [],
            )
            .unwrap();
        assert!(initialize_target(&mut connection, &row).is_ok());
        let attention:i64=connection.query_row("SELECT COUNT(*) FROM sprint_target_current_attentions WHERE authority_id='authority'",[],|r|r.get(0)).unwrap();
        assert_eq!(attention, 1);
    }

    #[test]
    fn safe_target_ref_rejects_detached_and_unsafe_names() {
        assert!(safe_ref("refs/heads/main"));
        assert!(!safe_ref("HEAD"));
        assert!(!safe_ref("refs/heads/../other"));
    }
}
