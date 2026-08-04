//! Application-owned accepted-candidate integration.  It has no transport surface.

use super::accepted_candidate_authority::{revalidate_retained_accepted_candidate, sprint_target_binding_fingerprint};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{fs, fs::{File, OpenOptions}, io::Write, path::{Path, PathBuf}, process::{Command, Stdio}};

pub(crate) const ACCEPTED_INTEGRATION_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS accepted_work_unit_integrations (
 integration_id TEXT PRIMARY KEY, work_unit_id TEXT NOT NULL UNIQUE REFERENCES work_units(work_unit_id), candidate_id TEXT NOT NULL UNIQUE REFERENCES accepted_handler_candidates(candidate_id), authority_id TEXT NOT NULL REFERENCES initiated_sprint_git_authorities(authority_id), target_ref_name TEXT NOT NULL, pre_object_id TEXT NOT NULL, pre_version INTEGER NOT NULL, candidate_commit_id TEXT NOT NULL, candidate_tree_id TEXT NOT NULL, baseline_object_id TEXT NOT NULL, intent_fingerprint TEXT NOT NULL UNIQUE, intent_recorded_at TEXT NOT NULL, commit_fingerprint TEXT, stage TEXT NOT NULL DEFAULT 'intent_reserved' CHECK(stage IN ('intent_reserved','object_created','ref_advanced','runtime_advanced','db_advanced','settled','attention')), integration_commit_id TEXT, integration_tree_id TEXT, object_created_at TEXT, ref_advanced_at TEXT, runtime_advanced_at TEXT, db_advanced_at TEXT, settled_at TEXT, notification_intent_recorded_at TEXT, notification_delivered_at TEXT, attention_code TEXT, attention_recorded_at TEXT, CHECK ((attention_code IS NULL) = (attention_recorded_at IS NULL))
);
CREATE TABLE IF NOT EXISTS accepted_work_unit_integration_evidence (evidence_id TEXT PRIMARY KEY, integration_id TEXT NOT NULL UNIQUE REFERENCES accepted_work_unit_integrations(integration_id), evidence_fingerprint TEXT NOT NULL UNIQUE, integration_commit_id TEXT NOT NULL, integration_tree_id TEXT NOT NULL, parent_object_id TEXT NOT NULL, candidate_id TEXT NOT NULL, target_ref_name TEXT NOT NULL, intent_fingerprint TEXT NOT NULL, recorded_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS work_unit_settlements (settlement_id TEXT PRIMARY KEY, work_unit_id TEXT NOT NULL UNIQUE REFERENCES work_units(work_unit_id), integration_id TEXT NOT NULL UNIQUE REFERENCES accepted_work_unit_integrations(integration_id), settled_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS work_unit_prerequisite_contributions (contribution_id TEXT PRIMARY KEY, prerequisite_work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id), dependent_work_unit_id TEXT NOT NULL REFERENCES work_units(work_unit_id), integration_id TEXT NOT NULL REFERENCES accepted_work_unit_integrations(integration_id), relationship_id TEXT NOT NULL UNIQUE REFERENCES work_unit_relationships(relationship_id), recorded_at TEXT NOT NULL, UNIQUE(prerequisite_work_unit_id,dependent_work_unit_id,integration_id));
CREATE TRIGGER IF NOT EXISTS accepted_work_unit_integrations_stage_insert BEFORE INSERT ON accepted_work_unit_integrations WHEN NEW.stage NOT IN ('intent_reserved','object_created','ref_advanced','runtime_advanced','db_advanced','settled','attention') BEGIN SELECT RAISE(ABORT,'invalid accepted integration stage'); END;
CREATE TRIGGER IF NOT EXISTS accepted_work_unit_integrations_stage_update BEFORE UPDATE OF stage ON accepted_work_unit_integrations WHEN NEW.stage NOT IN ('intent_reserved','object_created','ref_advanced','runtime_advanced','db_advanced','settled','attention') BEGIN SELECT RAISE(ABORT,'invalid accepted integration stage'); END;
"#;

const POLICY_VERSION: &str = "accepted-integration-policy/a1";
const COMMITTER_NAME: &str = "Codex Orchestrator";
const COMMITTER_EMAIL: &str = "orchestrator@local.invalid";

#[derive(Clone)] struct Row { candidate:String, unit:String, authority:String, repo:PathBuf, common:PathBuf, worktree:PathBuf, baseline:String, commit:String, tree:String, private_ref:String, evidence:String }
#[derive(Clone)] struct Target { reference:String, current:String, version:i64, binding:String }
#[derive(Clone)] struct Integration { id:String, reference:String, pre:String, version:i64, intent:String, recorded:String, stage:String, commit:Option<String>, tree:Option<String>, fingerprint:Option<String>, settled:Option<String> }

pub(crate) fn reconcile_accepted_integrations(c: &mut Connection) -> Result<(), String> {
    initialize_accepted_integration_schema(c)?;
    let candidates = c.prepare("SELECT candidate_id FROM accepted_handler_candidates WHERE pinned_at IS NOT NULL AND attention_reason IS NULL ORDER BY work_unit_id")
        .map_err(|e| e.to_string())?.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    for candidate in candidates {
        let Some(row) = load_row(c, &candidate)? else { continue };
        if let Err(code) = reconcile_one(c, &row) {
            if !retryable(&code) { attention(c, &row.candidate, &code)?; }
        }
    }
    Ok(())
}

pub(crate) fn initialize_accepted_integration_schema(c: &Connection) -> Result<(), String> {
    c.execute_batch(ACCEPTED_INTEGRATION_SCHEMA).map_err(|e| e.to_string())?;
    ensure_columns(c)
}

fn ensure_columns(c: &Connection) -> Result<(), String> {
    let columns = c.prepare("PRAGMA table_info(accepted_work_unit_integrations)").and_then(|mut s| s.query_map([], |r| r.get::<_, String>(1))?.collect::<Result<Vec<_>, _>>()).map_err(|e| e.to_string())?;
    for (name, definition) in [("stage", "TEXT NOT NULL DEFAULT 'intent_reserved'"), ("commit_fingerprint", "TEXT")] {
        if !columns.iter().any(|column| column == name) { c.execute_batch(&format!("ALTER TABLE accepted_work_unit_integrations ADD COLUMN {name} {definition}")).map_err(|e| e.to_string())?; }
    }
    c.execute_batch("CREATE TRIGGER IF NOT EXISTS accepted_work_unit_integrations_stage_insert BEFORE INSERT ON accepted_work_unit_integrations WHEN NEW.stage NOT IN ('intent_reserved','object_created','ref_advanced','runtime_advanced','db_advanced','settled','attention') BEGIN SELECT RAISE(ABORT,'invalid accepted integration stage'); END; CREATE TRIGGER IF NOT EXISTS accepted_work_unit_integrations_stage_update BEFORE UPDATE OF stage ON accepted_work_unit_integrations WHEN NEW.stage NOT IN ('intent_reserved','object_created','ref_advanced','runtime_advanced','db_advanced','settled','attention') BEGIN SELECT RAISE(ABORT,'invalid accepted integration stage'); END;").map_err(|e| e.to_string())?;
    Ok(())
}

fn load_row(c: &Connection, candidate: &str) -> Result<Option<Row>, String> {
    c.query_row("SELECT c.candidate_id,c.work_unit_id,c.authority_id,a.repository_root,a.repository_common_dir,a.worktree_root,c.attempt_baseline_object_id,c.candidate_commit_id,c.candidate_tree_id,c.private_ref_name,c.evidence_fingerprint FROM accepted_handler_candidates c JOIN initiated_sprint_git_authorities a ON a.authority_id=c.authority_id WHERE c.candidate_id=?1 AND c.pinned_at IS NOT NULL AND c.attention_reason IS NULL AND c.attempt_baseline_object_id IS NOT NULL", [candidate], |r| Ok(Row { candidate:r.get(0)?, unit:r.get(1)?, authority:r.get(2)?, repo:PathBuf::from(r.get::<_,String>(3)?), common:PathBuf::from(r.get::<_,String>(4)?), worktree:PathBuf::from(r.get::<_,String>(5)?), baseline:r.get(6)?, commit:r.get(7)?, tree:r.get(8)?, private_ref:r.get(9)?, evidence:r.get(10)? })).optional().map_err(|e| e.to_string())
}

fn target(c: &Connection, authority: &str) -> Result<Target, String> {
    c.query_row("SELECT target_ref_name,current_object_id,version,binding_fingerprint FROM sprint_target_currents WHERE authority_id=?1 AND attention_reason IS NULL", [authority], |r| Ok(Target { reference:r.get(0)?, current:r.get(1)?, version:r.get(2)?, binding:r.get(3)? })).map_err(|_| "target_unavailable".to_owned())
}

fn integration(c: &Connection, candidate: &str) -> Result<Option<Integration>, String> {
    c.query_row("SELECT integration_id,target_ref_name,pre_object_id,pre_version,intent_fingerprint,intent_recorded_at,stage,integration_commit_id,integration_tree_id,commit_fingerprint,settled_at FROM accepted_work_unit_integrations WHERE candidate_id=?1", [candidate], |r| Ok(Integration { id:r.get(0)?, reference:r.get(1)?, pre:r.get(2)?, version:r.get(3)?, intent:r.get(4)?, recorded:r.get(5)?, stage:r.get(6)?, commit:r.get(7)?, tree:r.get(8)?, fingerprint:r.get(9)?, settled:r.get(10)? })).optional().map_err(|e| e.to_string())
}

fn reconcile_one(c: &mut Connection, r: &Row) -> Result<(), String> {
    let Some(mut i) = integration(c, &r.candidate)? else { reserve(c, r)?; return reconcile_one(c, r) };
    if i.stage == "attention" { return Ok(()) }
    revalidate(r, c)?;
    validate_integration_correlations(c, r, &i)?;
    let _lock = Lock::take(&r.common, &target(c, &r.authority)?.reference)?;
    let t = target(c, &r.authority)?; // reload after acquiring the inter-process lock
    if i.settled.is_some() || i.stage == "settled" { verify_settled(c, r, &i, &t)?; return Ok(()) }
    let commit = match i.commit.clone() {
        Some(value) => { verify_commit(r, &i, &value)?; value }
        None => {
            require_pre_state(r, &t, &i)?;
            let tree = merged_tree(r, &i.pre)?;
            let (value, fingerprint) = create_commit(r, &i, &tree)?;
            verify_commit_parts(r, &i, &value, &tree, &fingerprint)?;
            c.execute("UPDATE accepted_work_unit_integrations SET integration_commit_id=?2,integration_tree_id=?3,commit_fingerprint=?4,object_created_at=COALESCE(object_created_at,?5),stage='object_created' WHERE integration_id=?1", params![i.id, value, tree, fingerprint, now()]).map_err(|e| e.to_string())?;
            i = integration(c, &r.candidate)?.ok_or("integration_missing")?;
            value
        }
    };
    recover_and_advance(c, r, &i, &commit)
}

fn reserve(c: &mut Connection, r: &Row) -> Result<(), String> {
    let t = target(c, &r.authority)?;
    validate_identity(r, &t)?;
    let id = stable_id("accepted-work-unit-integration", &r.candidate);
    let intent = fingerprint(&[POLICY_VERSION, &id, &r.unit, &r.candidate, &r.authority, &t.reference, &t.current, &t.version.to_string(), &r.baseline, &r.commit, &r.tree, &r.evidence]);
    let tx = c.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| e.to_string())?;
    match tx.execute("INSERT INTO accepted_work_unit_integrations(integration_id,work_unit_id,candidate_id,authority_id,target_ref_name,pre_object_id,pre_version,candidate_commit_id,candidate_tree_id,baseline_object_id,intent_fingerprint,intent_recorded_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)", params![id,r.unit,r.candidate,r.authority,t.reference,t.current,t.version,r.commit,r.tree,r.baseline,intent,now()]) {
        Ok(1) => {},
        Err(rusqlite::Error::SqliteFailure(error,_)) if error.code==rusqlite::ErrorCode::ConstraintViolation => { let exact:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM accepted_work_unit_integrations WHERE integration_id=?1 AND candidate_id=?2 AND authority_id=?3 AND target_ref_name=?4 AND pre_object_id=?5 AND pre_version=?6 AND candidate_commit_id=?7 AND candidate_tree_id=?8 AND baseline_object_id=?9 AND intent_fingerprint=?10)",params![id,r.candidate,r.authority,t.reference,t.current,t.version,r.commit,r.tree,r.baseline,intent],|x|x.get(0)).map_err(|e|e.to_string())?;if !exact{return Err("integration_reservation_conflict".into())} },
        Ok(_) => return Err("integration_reservation_conflict".into()), Err(e)=>return Err(e.to_string()),
    }
    tx.commit().map_err(|e| e.to_string())
}

fn validate_integration_correlations(c: &Connection, r: &Row, i: &Integration) -> Result<(), String> {
    let exact: bool = c.query_row("SELECT EXISTS(SELECT 1 FROM accepted_work_unit_integrations WHERE integration_id=?1 AND candidate_id=?2 AND work_unit_id=?3 AND authority_id=?4 AND target_ref_name=?5 AND candidate_commit_id=?6 AND candidate_tree_id=?7 AND baseline_object_id=?8)", params![i.id,r.candidate,r.unit,r.authority,i.reference,r.commit,r.tree,r.baseline], |x| x.get(0)).map_err(|e|e.to_string())?;
    if exact { Ok(()) } else { Err("durable_integration_correlation_mismatch".into()) }
}

fn recover_and_advance(c: &mut Connection, r: &Row, i: &Integration, commit: &str) -> Result<(), String> {
    let t = target(c, &r.authority)?;
    match i.stage.as_str() {
        "intent_reserved" | "object_created" => {
            if git(&r.worktree, &["show-ref", "--verify", "--hash", &t.reference])? == commit {
                return adopt_owned_ref_effect(c, r, i, commit, &t);
            }
            require_pre_state(r, &t, i)?
        }
        "ref_advanced" => return adopt_owned_ref_effect(c, r, i, commit, &t),
        "runtime_advanced" => { require_ref_and_runtime(r, i, commit, &t, false)?; advance_database(c, r, i, commit)?; return persist_evidence_and_settlement(c, r, i, commit); }
        "db_advanced" => { require_ref_and_runtime(r, i, commit, &t, true)?; return persist_evidence_and_settlement(c, r, i, commit); }
        _ => return Err("unknown_integration_stage".into()),
    }
    let ref_value = git(&r.worktree, &["show-ref", "--verify", "--hash", &t.reference])?;
    if ref_value == i.pre { git(&r.worktree, &["update-ref", &t.reference, commit, &i.pre]).map_err(|_| "target_ref_cas_lost".to_owned())?; }
    else if ref_value != commit { return Err("target_ref_advanced_or_foreign".into()) }
    c.execute("UPDATE accepted_work_unit_integrations SET ref_advanced_at=COALESCE(ref_advanced_at,?2),stage='ref_advanced' WHERE integration_id=?1", params![i.id, now()]).map_err(|e|e.to_string())?;
    converge_runtime(c, r, i, commit)
}

/// Adopt an exact owned ref effect when the effect completed before its durable stage write.
/// The only recoverable states are the old index/worktree or the clean integration runtime.
fn adopt_owned_ref_effect(c: &mut Connection, r: &Row, i: &Integration, commit: &str, t: &Target) -> Result<(), String> {
    if require_ref_advanced(r, i, commit, t).is_ok() {
        c.execute("UPDATE accepted_work_unit_integrations SET ref_advanced_at=COALESCE(ref_advanced_at,?2),stage='ref_advanced' WHERE integration_id=?1", params![i.id,now()]).map_err(|e|e.to_string())?;
        return converge_runtime(c,r,i,commit);
    }
    if require_ref_and_runtime(r,i,commit,t,false).is_ok() {
        c.execute("UPDATE accepted_work_unit_integrations SET ref_advanced_at=COALESCE(ref_advanced_at,?2),runtime_advanced_at=COALESCE(runtime_advanced_at,?2),stage='runtime_advanced' WHERE integration_id=?1",params![i.id,now()]).map_err(|e|e.to_string())?;
        advance_database(c,r,i,commit)?;
        return persist_evidence_and_settlement(c,r,i,commit);
    }
    Err("owned_ref_effect_state_ambiguous".into())
}

fn converge_runtime(c: &mut Connection, r: &Row, i: &Integration, commit: &str) -> Result<(), String> {
    git(&r.worktree, &["read-tree", "--reset", "-u", commit])?;
    let t = target(c, &r.authority)?;
    require_ref_and_runtime(r, i, commit, &t, false)?;
    c.execute("UPDATE accepted_work_unit_integrations SET runtime_advanced_at=COALESCE(runtime_advanced_at,?2),stage='runtime_advanced' WHERE integration_id=?1", params![i.id, now()]).map_err(|e|e.to_string())?;
    advance_database(c, r, i, commit)?;
    persist_evidence_and_settlement(c, r, i, commit)
}

fn require_pre_state(r: &Row, t: &Target, i: &Integration) -> Result<(), String> {
    validate_identity(r, t)?;
    if t.current != i.pre || t.version != i.version || t.binding != sprint_target_binding_fingerprint(&r.authority, &t.reference, &i.pre) { return Err("target_db_pre_state_mismatch".into()) }
    if git(&r.worktree, &["symbolic-ref", "--quiet", "HEAD"])? != t.reference || git(&r.worktree, &["rev-parse", "HEAD^{commit}"])? != i.pre || git(&r.worktree, &["show-ref", "--verify", "--hash", &t.reference])? != i.pre || git(&r.worktree, &["status", "--porcelain"])? != "" { return Err("target_ref_or_runtime_drift".into()) }
    Ok(())
}

fn require_ref_and_runtime(r: &Row, i: &Integration, commit: &str, t: &Target, db_advanced: bool) -> Result<(), String> {
    validate_identity(r, t)?;
    let expected_binding = sprint_target_binding_fingerprint(&r.authority, &t.reference, commit);
    if git(&r.worktree, &["symbolic-ref", "--quiet", "HEAD"])? != t.reference || git(&r.worktree, &["show-ref", "--verify", "--hash", &t.reference])? != commit || git(&r.worktree, &["rev-parse", "HEAD^{commit}"])? != commit || git(&r.worktree, &["status", "--porcelain"])? != "" { return Err("advanced_target_runtime_mismatch".into()) }
    if db_advanced { if t.current != commit || t.version != i.version + 1 || t.binding != expected_binding { return Err("advanced_target_db_mismatch".into()) } }
    else if t.current != i.pre || t.version != i.version || t.binding != sprint_target_binding_fingerprint(&r.authority, &t.reference, &i.pre) { return Err("advanced_target_db_pre_mismatch".into()) }
    Ok(())
}

fn require_ref_advanced(r: &Row, i: &Integration, commit: &str, t: &Target) -> Result<(), String> {
    validate_identity(r, t)?;
    if t.current != i.pre || t.version != i.version || t.binding != sprint_target_binding_fingerprint(&r.authority, &t.reference, &i.pre) { return Err("advanced_target_db_pre_mismatch".into()) }
    if git(&r.worktree, &["symbolic-ref", "--quiet", "HEAD"])? != t.reference || git(&r.worktree, &["show-ref", "--verify", "--hash", &t.reference])? != commit || git(&r.worktree, &["rev-parse", "HEAD^{commit}"])? != commit { return Err("ref_advanced_target_mismatch".into()) }
    let pre_tree = git(&r.repo, &["rev-parse", &format!("{}^{{tree}}", i.pre)])?;
    if git(&r.worktree, &["write-tree"])? != pre_tree || git(&r.worktree, &["diff-files", "--quiet"]).is_err() { return Err("ref_advanced_runtime_foreign".into()) }
    Ok(())
}

fn validate_identity(r: &Row, t: &Target) -> Result<(), String> {
    if !safe_ref(&t.reference) || !oid(&r.baseline) || !oid(&r.commit) || !oid(&t.current) { return Err("unsafe_durable_identity".into()) }
    if canon(&r.repo)? != git_path(&r.repo, "--show-toplevel")? || canon(&r.worktree)? != git_path(&r.worktree, "--show-toplevel")? || canon(&r.common)? != git_path(&r.worktree, "--git-common-dir")? { return Err("target_common_dir_drift".into()) }
    if git(&r.repo, &["show-ref", "--verify", "--hash", &r.private_ref])? != r.commit || git(&r.repo, &["rev-parse", &format!("{}^{{tree}}", r.commit)])? != r.tree || git(&r.repo, &["merge-base", "--is-ancestor", &r.baseline, &r.commit]).is_err() { return Err("candidate_private_ref_drift".into()) }
    Ok(())
}

fn revalidate(r: &Row, c: &mut Connection) -> Result<(), String> { revalidate_retained_accepted_candidate(c, &r.candidate)?; Ok(()) }

fn advance_database(c: &mut Connection, r: &Row, i: &Integration, commit: &str) -> Result<(), String> {
    let t = target(c, &r.authority)?;
    let binding = sprint_target_binding_fingerprint(&r.authority, &t.reference, commit);
    let tx = c.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    let changed = tx.execute("UPDATE sprint_target_currents SET current_object_id=?2,binding_fingerprint=?3,version=version+1,updated_at=?4 WHERE authority_id=?1 AND current_object_id=?5 AND binding_fingerprint=?6 AND version=?7", params![r.authority,commit,binding,now(),i.pre,sprint_target_binding_fingerprint(&r.authority,&t.reference,&i.pre),i.version]).map_err(|e|e.to_string())?;
    if changed == 0 { let exact: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM sprint_target_currents WHERE authority_id=?1 AND current_object_id=?2 AND binding_fingerprint=?3 AND version=?4)",params![r.authority,commit,binding,i.version+1],|x|x.get(0)).map_err(|e|e.to_string())?; if !exact { return Err("target_current_cas_lost".into()) } }
    tx.execute("UPDATE accepted_work_unit_integrations SET db_advanced_at=COALESCE(db_advanced_at,?2),stage='db_advanced' WHERE integration_id=?1", params![i.id,now()]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e|e.to_string())
}

fn persist_evidence_and_settlement(c: &mut Connection, r: &Row, i: &Integration, commit: &str) -> Result<(), String> {
    let t = target(c, &r.authority)?;
    require_ref_and_runtime(r, i, commit, &t, true)?;
    verify_commit(r, i, commit)?;
    let tree = git(&r.repo, &["rev-parse", &format!("{commit}^{{tree}}")])?;
    let evidence = fingerprint(&[POLICY_VERSION, &i.id, &i.intent, &r.candidate, &r.evidence, &i.pre, commit, &tree]);
    let timestamp = now();
    let tx = c.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e|e.to_string())?;
    let evidence_id=stable_id("accepted-integration-evidence",&i.id); exact_insert(&tx, "INSERT INTO accepted_work_unit_integration_evidence(evidence_id,integration_id,evidence_fingerprint,integration_commit_id,integration_tree_id,parent_object_id,candidate_id,target_ref_name,intent_fingerprint,recorded_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![evidence_id,i.id,evidence,commit,tree,i.pre,r.candidate,t.reference,i.intent,timestamp], "SELECT EXISTS(SELECT 1 FROM accepted_work_unit_integration_evidence WHERE evidence_id=?1 AND integration_id=?2 AND evidence_fingerprint=?3 AND integration_commit_id=?4 AND integration_tree_id=?5 AND parent_object_id=?6 AND candidate_id=?7 AND target_ref_name=?8 AND intent_fingerprint=?9)", params![evidence_id,i.id,evidence,commit,tree,i.pre,r.candidate,t.reference,i.intent])?;
    let settlement_id=stable_id("work-unit-settlement",&i.id); exact_insert(&tx, "INSERT INTO work_unit_settlements(settlement_id,work_unit_id,integration_id,settled_at) VALUES(?1,?2,?3,?4)", params![settlement_id,r.unit,i.id,timestamp], "SELECT EXISTS(SELECT 1 FROM work_unit_settlements WHERE settlement_id=?1 AND work_unit_id=?2 AND integration_id=?3)", params![settlement_id,r.unit,i.id])?;
    let edges = tx.prepare("SELECT relationship_id,from_id FROM work_unit_relationships WHERE relationship_kind='depends_on' AND to_id=?1 ORDER BY relationship_id").map_err(|e|e.to_string())?.query_map([&r.unit], |x| Ok((x.get::<_,String>(0)?,x.get::<_,String>(1)?))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    for (edge, dependent) in edges { let contribution_id=stable_id("work-unit-prerequisite-contribution",&format!("{}:{edge}",i.id)); exact_insert(&tx, "INSERT INTO work_unit_prerequisite_contributions(contribution_id,prerequisite_work_unit_id,dependent_work_unit_id,integration_id,relationship_id,recorded_at) VALUES(?1,?2,?3,?4,?5,?6)", params![contribution_id,r.unit,dependent,i.id,edge,timestamp], "SELECT EXISTS(SELECT 1 FROM work_unit_prerequisite_contributions WHERE contribution_id=?1 AND relationship_id=?2 AND prerequisite_work_unit_id=?3 AND dependent_work_unit_id=?4 AND integration_id=?5)", params![contribution_id,edge,r.unit,dependent,i.id])?; }
    tx.execute("UPDATE accepted_work_unit_integrations SET settled_at=COALESCE(settled_at,?2),notification_intent_recorded_at=COALESCE(notification_intent_recorded_at,?2),stage='settled' WHERE integration_id=?1",params![i.id,timestamp]).map_err(|e|e.to_string())?;
    tx.commit().map_err(|e|e.to_string())
}

fn exact_insert(tx: &rusqlite::Transaction<'_>, insert: &str, values: impl rusqlite::Params, exact: &str, exact_values: impl rusqlite::Params) -> Result<(), String> { match tx.execute(insert, values) { Ok(_) => Ok(()), Err(rusqlite::Error::SqliteFailure(error, _)) if error.code == rusqlite::ErrorCode::ConstraintViolation => { let is_exact: bool = tx.query_row(exact, exact_values, |r|r.get(0)).map_err(|e|e.to_string())?; if is_exact { Ok(()) } else { Err("durable_replay_conflict".into()) } }, Err(e) => Err(e.to_string()) } }

fn verify_settled(c: &mut Connection, r: &Row, i: &Integration, t: &Target) -> Result<(), String> {
    let commit=i.commit.as_deref().ok_or("settled_commit_missing")?;
    validate_integration_correlations(c,r,i)?; validate_identity(r,t)?; verify_commit(r,i,commit)?;
    if t.reference != i.reference || t.binding != sprint_target_binding_fingerprint(&r.authority,&t.reference,&t.current) || git(&r.worktree,&["symbolic-ref","--quiet","HEAD"])? != t.reference || git(&r.worktree,&["show-ref","--verify","--hash",&t.reference])? != t.current || git(&r.worktree,&["rev-parse","HEAD^{commit}"])? != t.current || git(&r.worktree,&["status","--porcelain"])? != "" || git(&r.repo,&["merge-base","--is-ancestor",commit,&t.current]).is_err() { return Err("settled_target_line_mismatch".into()) }
    let tree=git(&r.repo,&["rev-parse",&format!("{commit}^{{tree}}")])?; let evidence=fingerprint(&[POLICY_VERSION,&i.id,&i.intent,&r.candidate,&r.evidence,&i.pre,commit,&tree]);
    let exact_evidence: bool=c.query_row("SELECT EXISTS(SELECT 1 FROM accepted_work_unit_integration_evidence WHERE evidence_id=?1 AND integration_id=?2 AND evidence_fingerprint=?3 AND integration_commit_id=?4 AND integration_tree_id=?5 AND parent_object_id=?6 AND candidate_id=?7 AND target_ref_name=?8 AND intent_fingerprint=?9)",params![stable_id("accepted-integration-evidence",&i.id),i.id,evidence,commit,tree,i.pre,r.candidate,i.reference,i.intent],|x|x.get(0)).map_err(|e|e.to_string())?;
    let settlement:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_settlements WHERE settlement_id=?1 AND work_unit_id=?2 AND integration_id=?3)",params![stable_id("work-unit-settlement",&i.id),r.unit,i.id],|x|x.get(0)).map_err(|e|e.to_string())?;
    let missing:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_relationships e WHERE e.relationship_kind='depends_on' AND e.to_id=?1 AND NOT EXISTS(SELECT 1 FROM work_unit_prerequisite_contributions p WHERE p.relationship_id=e.relationship_id AND p.prerequisite_work_unit_id=?1 AND p.dependent_work_unit_id=e.from_id AND p.integration_id=?2))",params![r.unit,i.id],|x|x.get(0)).map_err(|e|e.to_string())?;
    let extra:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM work_unit_prerequisite_contributions p WHERE p.integration_id=?1 AND NOT EXISTS(SELECT 1 FROM work_unit_relationships e WHERE e.relationship_kind='depends_on' AND e.relationship_id=p.relationship_id AND e.to_id=?2 AND e.from_id=p.dependent_work_unit_id))",params![i.id,r.unit],|x|x.get(0)).map_err(|e|e.to_string())?;
    if !exact_evidence||!settlement||missing||extra { return Err("settled_evidence_or_settlement_missing".into()) } Ok(())
}

fn merged_tree(r: &Row, pre: &str) -> Result<String, String> { let dir=std::env::temp_dir().join(format!("codex-integration-{}",uuid::Uuid::new_v4())); fs::create_dir_all(&dir).map_err(|_|"temporary_index_unavailable".to_string())?; let index=dir.join("index"); let result=git_index(&r.repo,&["read-tree","-m",&r.baseline,pre,&r.commit],&index).and_then(|_|git_index(&r.repo,&["write-tree"],&index)); let _=fs::remove_dir_all(dir); result.map_err(|_|"integration_conflict".into()) }

fn create_commit(r:&Row, i:&Integration, tree:&str) -> Result<(String,String),String> { let (name,email,date)=author(r)?; let fingerprint=commit_fingerprint(r,i,tree,&name,&email,&date); let message=commit_message(r,i,tree,&fingerprint); let mut process=Command::new("git").args(["--no-replace-objects","commit-tree",tree,"-p",&i.pre]).current_dir(&r.repo).env("GIT_TERMINAL_PROMPT","0").env("GIT_EDITOR","true").env("GIT_CONFIG_NOSYSTEM","1").env("GIT_CONFIG_GLOBAL","NUL").env("GIT_AUTHOR_NAME",name).env("GIT_AUTHOR_EMAIL",email).env("GIT_AUTHOR_DATE",date).env("GIT_COMMITTER_NAME",COMMITTER_NAME).env("GIT_COMMITTER_EMAIL",COMMITTER_EMAIL).env("GIT_COMMITTER_DATE",&i.recorded).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().map_err(|_|"git_unavailable".to_string())?; process.stdin.as_mut().ok_or("git_unavailable")?.write_all(message.as_bytes()).map_err(|_|"git_command_failed".to_string())?; let out=process.wait_with_output().map_err(|_|"git_command_failed".to_string())?; if out.status.success(){Ok((decode_git(out.stdout)?.trim().to_owned(),fingerprint))}else{Err("git_command_failed".into())} }

fn author(r:&Row)->Result<(String,String,String),String>{let value=git(&r.repo,&["show","-s","--format=%an%x00%ae%x00%aI",&r.commit])?;let mut parts=value.split('\0');Ok((parts.next().ok_or("candidate_author_invalid")?.to_owned(),parts.next().ok_or("candidate_author_invalid")?.to_owned(),parts.next().ok_or("candidate_author_invalid")?.to_owned()))}
fn commit_fingerprint(r:&Row,i:&Integration,tree:&str,author:&str,email:&str,date:&str)->String{fingerprint(&[POLICY_VERSION,&i.id,&i.intent,&r.evidence,&r.candidate,&r.commit,&r.tree,&r.baseline,&r.unit,&r.authority,&i.reference,&i.pre,tree,author,email,date,COMMITTER_NAME,COMMITTER_EMAIL,&i.recorded])}
fn commit_message(r:&Row,i:&Integration,tree:&str,fingerprint:&str)->String{format!("Codex Orchestrator accepted Work Unit integration v1\n\nPolicy-Version: {POLICY_VERSION}\nIntegration: {}\nIntent: {}\nEvidence: {}\nCandidate: {}\nCandidate-Commit: {}\nCandidate-Tree: {}\nBaseline: {}\nWork-Unit: {}\nAuthority: {}\nTarget-Ref: {}\nPre-Object: {}\nResult-Tree: {tree}\nFingerprint: {fingerprint}\n",i.id,i.intent,r.evidence,r.candidate,r.commit,r.tree,r.baseline,r.unit,r.authority,i.reference,i.pre)}
fn verify_commit(r:&Row,i:&Integration,commit:&str)->Result<(),String>{let tree=i.tree.as_deref().ok_or("integration_tree_missing")?;let fingerprint=i.fingerprint.as_deref().ok_or("integration_fingerprint_missing")?;verify_commit_parts(r,i,commit,tree,fingerprint)}
fn verify_commit_parts(r:&Row,i:&Integration,commit:&str,tree:&str,fingerprint_value:&str)->Result<(),String>{if git(&r.repo,&["rev-list","--parents","-n","1",commit])?!=format!("{commit} {}",i.pre)||git(&r.repo,&["rev-parse",&format!("{commit}^{{tree}}")])?!=tree||tree!=merged_tree(r,&i.pre)?{return Err("integration_object_mismatch".into())}let(name,email,date)=author(r)?;if fingerprint_value!=commit_fingerprint(r,i,tree,&name,&email,&date){return Err("integration_fingerprint_mismatch".into())}let actual=git(&r.repo,&["show","-s","--format=%an%x00%ae%x00%aI%x00%cn%x00%ce%x00%cI%x00%B",commit])?;let mut p=actual.split('\0');let fields=[p.next(),p.next(),p.next(),p.next(),p.next(),p.next()];if fields!=[Some(name.as_str()),Some(email.as_str()),Some(date.as_str()),Some(COMMITTER_NAME),Some(COMMITTER_EMAIL),Some(i.recorded.as_str())]{return Err("integration_identity_metadata_mismatch".into())}let expected=commit_message(r,i,tree,fingerprint_value);if p.next().unwrap_or("").trim_end()!=expected.trim_end(){return Err("integration_message_mismatch".into())}Ok(())}

fn attention(c:&mut Connection,candidate:&str,code:&str)->Result<(),String>{c.execute("UPDATE accepted_work_unit_integrations SET attention_code=COALESCE(attention_code,?2),attention_recorded_at=COALESCE(attention_recorded_at,?3),stage='attention' WHERE candidate_id=?1",params![candidate,code,now()]).map_err(|e|e.to_string())?;Ok(())}
fn retryable(code:&str)->bool{matches!(code,"target_lock_unavailable"|"database is locked"|"database is busy")||code.contains("database is locked")||code.contains("database is busy")}
struct Lock{_file:File} impl Lock{fn take(common:&Path,reference:&str)->Result<Self,String>{let file=OpenOptions::new().read(true).write(true).create(true).open(common.join(format!("codex-orchestrator-{}.lock",stable_id("target-ref-lock",reference)))).map_err(|_|"target_lock_unavailable".to_string())?;file.try_lock().map_err(|_|"target_lock_unavailable".to_string())?;Ok(Self{_file:file})}}
fn decode_git(bytes:Vec<u8>)->Result<String,String>{if bytes.len()>1024*1024{return Err("git_output_too_large".into())}String::from_utf8(bytes).map_err(|_|"git_output_invalid_utf8".into())}
fn git(root:&Path,args:&[&str])->Result<String,String>{let out=Command::new("git").arg("--no-replace-objects").args(args).current_dir(root).env("GIT_TERMINAL_PROMPT","0").env("GIT_EDITOR","true").env("GIT_CONFIG_NOSYSTEM","1").env("GIT_CONFIG_GLOBAL","NUL").stdin(Stdio::null()).output().map_err(|_|"git_unavailable".to_string())?;if out.status.success(){Ok(decode_git(out.stdout)?.trim().to_owned())}else{Err("git_command_failed".into())}}
fn git_path(root:&Path,arg:&str)->Result<String,String>{let path=PathBuf::from(git(root,&["rev-parse",arg])?);let resolved=if path.is_absolute(){path}else{root.join(path)};canon(&resolved)}
fn git_index(root:&Path,args:&[&str],index:&Path)->Result<String,String>{let out=Command::new("git").arg("--no-replace-objects").args(args).env("GIT_INDEX_FILE",index).env("GIT_TERMINAL_PROMPT","0").env("GIT_EDITOR","true").env("GIT_CONFIG_NOSYSTEM","1").env("GIT_CONFIG_GLOBAL","NUL").current_dir(root).stdin(Stdio::null()).output().map_err(|_|"git_unavailable".to_string())?;if out.status.success(){Ok(decode_git(out.stdout)?.trim().to_owned())}else{Err("git_command_failed".into())}}
fn canon(path:&Path)->Result<String,String>{path.canonicalize().map_err(|_|"path_unavailable".to_string()).map(|p|p.to_string_lossy().replace('\\',"/"))} fn safe_ref(value:&str)->bool{value.starts_with("refs/heads/")&&value.len()<256&&!value.contains("..")&&!value.ends_with('/')}fn oid(value:&str)->bool{(value.len()==40||value.len()==64)&&value.bytes().all(|byte|byte.is_ascii_hexdigit())}fn fingerprint(values:&[&str])->String{let mut hasher=Sha256::new();for value in values{hasher.update(value.as_bytes());hasher.update([0]);}format!("{:x}",hasher.finalize())}fn stable_id(domain:&str,value:&str)->String{fingerprint(&[domain,value])[..32].to_owned()}fn now()->String{chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()}

#[cfg(test)] mod tests { use super::*; use std::sync::{Arc,Barrier}; use tempfile::TempDir;
 fn run(root:&Path,args:&[&str]){assert!(Command::new("git").args(args).current_dir(root).env("GIT_CONFIG_NOSYSTEM","1").env("GIT_CONFIG_GLOBAL","NUL").status().unwrap().success(),"git {args:?}")}
 fn fixture()->(TempDir,Connection,Row){let dir=TempDir::new().unwrap();let root=dir.path();run(root,&["init","-b","main"]);run(root,&["config","user.name","Test User"]);run(root,&["config","user.email","test@example.invalid"]);fs::write(root.join("base.txt"),"base\n").unwrap();run(root,&["add","."]);run(root,&["commit","-m","base"]);let baseline=git(root,&["rev-parse","HEAD"]).unwrap();run(root,&["checkout","-b","candidate"]);fs::write(root.join("base.txt"),"candidate\n").unwrap();fs::write(root.join("candidate.txt"),"candidate\n").unwrap();run(root,&["add","."]);run(root,&["commit","-m","candidate"]);let candidate=git(root,&["rev-parse","HEAD"]).unwrap();let tree=git(root,&["rev-parse","HEAD^{tree}"]).unwrap();run(root,&["update-ref","refs/codex/orchestrator/accepted/candidate",&candidate]);run(root,&["checkout","main"]);let c=Connection::open_in_memory().unwrap();c.execute_batch("CREATE TABLE work_units(work_unit_id TEXT PRIMARY KEY);CREATE TABLE initiated_sprint_git_authorities(authority_id TEXT PRIMARY KEY,repository_root TEXT,repository_common_dir TEXT,worktree_root TEXT,baseline_object_id TEXT);CREATE TABLE accepted_handler_candidates(candidate_id TEXT PRIMARY KEY,work_unit_id TEXT,authority_id TEXT,pinned_at TEXT,attention_reason TEXT,attempt_baseline_object_id TEXT,candidate_commit_id TEXT,candidate_tree_id TEXT,private_ref_name TEXT,evidence_fingerprint TEXT);CREATE TABLE sprint_target_currents(authority_id TEXT PRIMARY KEY,target_ref_name TEXT,current_object_id TEXT,binding_fingerprint TEXT,version INTEGER,attention_reason TEXT,updated_at TEXT);CREATE TABLE work_unit_relationships(relationship_id TEXT PRIMARY KEY,relationship_kind TEXT,from_id TEXT,to_id TEXT);").unwrap();c.execute("INSERT INTO work_units VALUES('unit')",[]).unwrap();let common=root.join(".git");c.execute("INSERT INTO initiated_sprint_git_authorities VALUES('authority',?1,?2,?1,?3)",params![root.to_string_lossy(),common.to_string_lossy(),baseline]).unwrap();let time=now();c.execute("INSERT INTO accepted_handler_candidates VALUES('candidate','unit','authority',?1,NULL,?2,?3,?4,'refs/codex/orchestrator/accepted/candidate','evidence')",params![time,baseline,candidate,tree]).unwrap();let binding=sprint_target_binding_fingerprint("authority","refs/heads/main",&baseline);c.execute("INSERT INTO sprint_target_currents VALUES('authority','refs/heads/main',?1,?2,1,NULL,?3)",params![baseline,binding,now()]).unwrap();let row=Row{candidate:"candidate".into(),unit:"unit".into(),authority:"authority".into(),repo:root.to_path_buf(),common,worktree:root.to_path_buf(),baseline,commit:candidate,tree,private_ref:"refs/codex/orchestrator/accepted/candidate".into(),evidence:"evidence".into()};(dir,c,row)}
 fn file_fixture()->(TempDir,PathBuf,Row){let dir=TempDir::new().unwrap();let root=dir.path().join("repo");fs::create_dir(&root).unwrap();run(&root,&["init","-b","main"]);run(&root,&["config","user.name","Test User"]);run(&root,&["config","user.email","test@example.invalid"]);fs::write(root.join("base.txt"),"base\n").unwrap();run(&root,&["add","."]);run(&root,&["commit","-m","base"]);let baseline=git(&root,&["rev-parse","HEAD"]).unwrap();run(&root,&["checkout","-b","candidate"]);fs::write(root.join("a.txt"),"a\n").unwrap();run(&root,&["add","."]);run(&root,&["commit","-m","candidate"]);let candidate=git(&root,&["rev-parse","HEAD"]).unwrap();let tree=git(&root,&["rev-parse","HEAD^{tree}"]).unwrap();run(&root,&["update-ref","refs/codex/orchestrator/accepted/candidate",&candidate]);run(&root,&["checkout","main"]);let db=dir.path().join("state.sqlite");let c=Connection::open(&db).unwrap();c.busy_timeout(std::time::Duration::from_millis(50)).unwrap();c.execute_batch("PRAGMA journal_mode=WAL;CREATE TABLE work_units(work_unit_id TEXT PRIMARY KEY);CREATE TABLE initiated_sprint_git_authorities(authority_id TEXT PRIMARY KEY,repository_root TEXT,repository_common_dir TEXT,worktree_root TEXT,baseline_object_id TEXT);CREATE TABLE accepted_handler_candidates(candidate_id TEXT PRIMARY KEY,work_unit_id TEXT,authority_id TEXT,pinned_at TEXT,attention_reason TEXT,attempt_baseline_object_id TEXT,candidate_commit_id TEXT,candidate_tree_id TEXT,private_ref_name TEXT,evidence_fingerprint TEXT);CREATE TABLE sprint_target_currents(authority_id TEXT PRIMARY KEY,target_ref_name TEXT,current_object_id TEXT,binding_fingerprint TEXT,version INTEGER,attention_reason TEXT,updated_at TEXT);CREATE TABLE work_unit_relationships(relationship_id TEXT PRIMARY KEY,relationship_kind TEXT,from_id TEXT,to_id TEXT);INSERT INTO work_units VALUES('unit'),('dependent');").unwrap();let common=root.join(".git");c.execute("INSERT INTO initiated_sprint_git_authorities VALUES('authority',?1,?2,?1,?3)",params![root.to_string_lossy(),common.to_string_lossy(),baseline]).unwrap();c.execute("INSERT INTO accepted_handler_candidates VALUES('candidate','unit','authority',?1,NULL,?2,?3,?4,'refs/codex/orchestrator/accepted/candidate','evidence')",params![now(),baseline,candidate,tree]).unwrap();let binding=sprint_target_binding_fingerprint("authority","refs/heads/main",&baseline);c.execute("INSERT INTO sprint_target_currents VALUES('authority','refs/heads/main',?1,?2,1,NULL,?3)",params![baseline,binding,now()]).unwrap();c.execute("INSERT INTO work_unit_relationships VALUES('edge','depends_on','dependent','unit')",[]).unwrap();drop(c);let row=Row{candidate:"candidate".into(),unit:"unit".into(),authority:"authority".into(),repo:root.clone(),common,worktree:root,baseline,commit:candidate,tree,private_ref:"refs/codex/orchestrator/accepted/candidate".into(),evidence:"evidence".into()};(dir,db,row)}
 #[test] fn candidate_initial_pin_and_exact_commit_adoption(){let(_d,mut c,row)=fixture();reconcile_accepted_integrations(&mut c).unwrap();let attention:Option<String>=c.query_row("SELECT attention_code FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap();assert!(attention.is_none(),"attention={attention:?}");let(commit,tree):(String,String)=c.query_row("SELECT integration_commit_id,integration_tree_id FROM accepted_work_unit_integrations",[],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();assert_eq!(tree,row.tree);assert_eq!(git(&row.repo,&["rev-list","--parents","-n","1",&commit]).unwrap(),format!("{commit} {}",row.baseline));assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_settlements",[],|r|r.get(0)).unwrap(),1);}
 #[test] fn db_advanced_reopen_settles_exactly(){let(_d,mut c,_row)=fixture();reconcile_accepted_integrations(&mut c).unwrap();c.execute_batch("DELETE FROM accepted_work_unit_integration_evidence;DELETE FROM work_unit_settlements;UPDATE accepted_work_unit_integrations SET stage='db_advanced',settled_at=NULL;").unwrap();reconcile_accepted_integrations(&mut c).unwrap();assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM accepted_work_unit_integration_evidence",[],|r|r.get(0)).unwrap(),1);assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_settlements",[],|r|r.get(0)).unwrap(),1);assert_eq!(c.query_row::<String,_,_>("SELECT stage FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),"settled");}
 #[test] fn lock_contention_stays_pending_then_converges(){let(_d,mut c,row)=fixture();let held=Lock::take(&row.common,"refs/heads/main").unwrap();reconcile_accepted_integrations(&mut c).unwrap();assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),1);assert_eq!(c.query_row::<Option<String>,_,_>("SELECT attention_code FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),None);drop(held);reconcile_accepted_integrations(&mut c).unwrap();assert_eq!(c.query_row::<String,_,_>("SELECT stage FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),"settled");}
 #[test] fn divergent_evidence_replay_is_rejected(){let(_d,mut c,_row)=fixture();reconcile_accepted_integrations(&mut c).unwrap();c.execute_batch("UPDATE accepted_work_unit_integration_evidence SET evidence_fingerprint='forged';UPDATE accepted_work_unit_integrations SET stage='db_advanced',settled_at=NULL;").unwrap();reconcile_accepted_integrations(&mut c).unwrap();assert_eq!(c.query_row::<String,_,_>("SELECT attention_code FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),"durable_replay_conflict");}
 #[test] fn ref_before_runtime_reopen_converges_once(){let(_d,mut c,row)=fixture();reconcile_accepted_integrations(&mut c).unwrap();let commit:String=c.query_row("SELECT integration_commit_id FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap();run(&row.repo,&["read-tree","--reset","-u",&row.baseline]);let binding=sprint_target_binding_fingerprint(&row.authority,"refs/heads/main",&row.baseline);c.execute_batch("DELETE FROM accepted_work_unit_integration_evidence;DELETE FROM work_unit_settlements;").unwrap();c.execute("UPDATE sprint_target_currents SET current_object_id=?1,binding_fingerprint=?2,version=1",params![row.baseline,binding]).unwrap();c.execute("UPDATE accepted_work_unit_integrations SET stage='ref_advanced',settled_at=NULL",[]).unwrap();reconcile_accepted_integrations(&mut c).unwrap();assert_eq!(git(&row.repo,&["rev-parse","HEAD"]).unwrap(),commit);assert_eq!(c.query_row::<String,_,_>("SELECT stage FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),"settled");assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),1);}
 #[test] fn ref_cas_before_stage_reopen_adopts_owned_pre_runtime(){let(_d,mut c,row)=fixture();reconcile_accepted_integrations(&mut c).unwrap();run(&row.repo,&["read-tree","--reset","-u",&row.baseline]);let binding=sprint_target_binding_fingerprint(&row.authority,"refs/heads/main",&row.baseline);c.execute_batch("DELETE FROM accepted_work_unit_integration_evidence;DELETE FROM work_unit_settlements;").unwrap();c.execute("UPDATE sprint_target_currents SET current_object_id=?1,binding_fingerprint=?2,version=1",params![row.baseline,binding]).unwrap();c.execute("UPDATE accepted_work_unit_integrations SET stage='object_created',settled_at=NULL",[]).unwrap();reconcile_accepted_integrations(&mut c).unwrap();assert_eq!(c.query_row::<String,_,_>("SELECT stage FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),"settled");assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_settlements",[],|r|r.get(0)).unwrap(),1);}
 #[test] fn runtime_before_stage_reopen_adopts_owned_clean_runtime(){let(_d,mut c,row)=fixture();reconcile_accepted_integrations(&mut c).unwrap();let binding=sprint_target_binding_fingerprint(&row.authority,"refs/heads/main",&row.baseline);c.execute_batch("DELETE FROM accepted_work_unit_integration_evidence;DELETE FROM work_unit_settlements;").unwrap();c.execute("UPDATE sprint_target_currents SET current_object_id=?1,binding_fingerprint=?2,version=1",params![row.baseline,binding]).unwrap();c.execute("UPDATE accepted_work_unit_integrations SET stage='ref_advanced',settled_at=NULL",[]).unwrap();reconcile_accepted_integrations(&mut c).unwrap();assert_eq!(c.query_row::<String,_,_>("SELECT stage FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),"settled");assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_settlements",[],|r|r.get(0)).unwrap(),1);}
 #[test] fn two_file_backed_connections_converge_one_target_effect(){let(_d,path,row)=file_fixture();let barrier=Arc::new(Barrier::new(3));let calls=(0..2).map(|_|{let path=path.clone();let barrier=barrier.clone();std::thread::spawn(move||{let mut c=Connection::open(path).unwrap();c.busy_timeout(std::time::Duration::from_millis(50)).unwrap();barrier.wait();reconcile_accepted_integrations(&mut c)})}).collect::<Vec<_>>();barrier.wait();for call in calls{assert!(call.join().unwrap().is_ok())}let mut reopened=Connection::open(&path).unwrap();reconcile_accepted_integrations(&mut reopened).unwrap();assert_eq!(reopened.query_row::<i64,_,_>("SELECT COUNT(*) FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),1);assert_eq!(reopened.query_row::<i64,_,_>("SELECT COUNT(*) FROM accepted_work_unit_integration_evidence",[],|r|r.get(0)).unwrap(),1);assert_eq!(reopened.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_settlements",[],|r|r.get(0)).unwrap(),1);assert_eq!(reopened.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_prerequisite_contributions",[],|r|r.get(0)).unwrap(),1);assert_eq!(reopened.query_row::<Option<String>,_,_>("SELECT attention_code FROM accepted_work_unit_integrations",[],|r|r.get(0)).unwrap(),None);assert_eq!(git(&row.repo,&["rev-list","--count","HEAD"]).unwrap(),"2");}
 #[test] fn two_candidates_serialize_against_reloaded_target(){let(_d,mut c,row)=fixture();c.execute_batch("INSERT INTO work_units VALUES('unit-2'),('dependent-1'),('dependent-2');INSERT INTO work_unit_relationships VALUES('edge-1','depends_on','dependent-1','unit'),('edge-2','depends_on','dependent-2','unit-2');").unwrap();run(&row.repo,&["checkout","-b","candidate-two",&row.baseline]);fs::write(row.repo.join("b.txt"),"b\n").unwrap();run(&row.repo,&["add","."]);run(&row.repo,&["commit","-m","candidate two"]);let candidate_two=git(&row.repo,&["rev-parse","HEAD"]).unwrap();let tree_two=git(&row.repo,&["rev-parse","HEAD^{tree}"]).unwrap();run(&row.repo,&["update-ref","refs/codex/orchestrator/accepted/candidate-two",&candidate_two]);run(&row.repo,&["checkout","main"]);c.execute("INSERT INTO accepted_handler_candidates VALUES('candidate-two','unit-2','authority',?1,NULL,?2,?3,?4,'refs/codex/orchestrator/accepted/candidate-two','evidence-two')",params![now(),row.baseline,candidate_two,tree_two]).unwrap();reconcile_accepted_integrations(&mut c).unwrap();let commits=c.prepare("SELECT integration_commit_id FROM accepted_work_unit_integrations ORDER BY work_unit_id").unwrap().query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<Result<Vec<_>,_>>().unwrap();assert_eq!(commits.len(),2);assert_eq!(git(&row.repo,&["rev-list","--parents","-n","1",&commits[1]]).unwrap(),format!("{} {}",commits[1],commits[0]));let result_tree=git(&row.repo,&["rev-parse",&format!("{}^{{tree}}",commits[1])]).unwrap();assert_eq!(git(&row.repo,&["show",&format!("{result_tree}:candidate.txt")]).unwrap(),"candidate");assert_eq!(git(&row.repo,&["show",&format!("{result_tree}:b.txt")]).unwrap(),"b");reconcile_accepted_integrations(&mut c).unwrap();assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_settlements",[],|r|r.get(0)).unwrap(),2);assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM accepted_work_unit_integrations WHERE attention_code IS NULL AND stage='settled'",[],|r|r.get(0)).unwrap(),2);assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM work_unit_prerequisite_contributions WHERE (prerequisite_work_unit_id='unit' AND dependent_work_unit_id='dependent-1') OR (prerequisite_work_unit_id='unit-2' AND dependent_work_unit_id='dependent-2')",[],|r|r.get(0)).unwrap(),2);assert_eq!(c.query_row::<i64,_,_>("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('work_unit_handler_activations','work_slice_planning_points','sprint_settlements','epic_settlements')",[],|r|r.get(0)).unwrap(),0);}
}
