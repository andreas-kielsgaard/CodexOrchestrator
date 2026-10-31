use super::*;
use crate::otp_api::OtpPackage;

struct FacadeOwner(Option<Box<dyn ManagedMcpUpstreamOwner>>);
impl Drop for FacadeOwner {
    fn drop(&mut self) {
        if let Some(owner) = self.0.take() {
            owner.stop();
        }
    }
}

#[tokio::test]
#[ignore = "Requires disposable Job Agent data and its Python environment"]
async fn real_job_agent_curation_facade() {
    let root = std::path::PathBuf::from(std::env::var("JOB_AGENT_PROOF_ROOT").unwrap());
    let allowed = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../.dev/curation-proof")
        .canonicalize()
        .unwrap();
    assert!(root.canonicalize().unwrap().starts_with(allowed));
    let installations = OtpInstallationService::in_memory();
    installations
        .save_configuration(JobAgentInstallation {
            root: root.to_string_lossy().into(),
            python: std::env::var("JOB_AGENT_PROOF_PYTHON").unwrap(),
        })
        .unwrap();
    let tools = crate::otp_packages::job_agent::JobAgentPackage
        .descriptor()
        .agent_mcp_servers
        .remove(0)
        .tools
        .into_iter()
        .filter(|tool| tool.capability == "curation")
        .collect();
    let facade = JobAgentMcpFacade::start(
        installations,
        tools,
        ["curation".into()].into_iter().collect(),
        vec![],
    )
    .unwrap();
    let owner = FacadeOwner(Some(facade.owner));
    let client = reqwest::Client::new();
    let call = |name: &'static str, arguments: Value| {
        let client = client.clone();
        let descriptor = facade.descriptor.clone();
        async move {
            client.post(&descriptor.url).bearer_auth(&descriptor.bearer_token)
                .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":arguments}}))
                .send().await.unwrap().json::<Value>().await.unwrap()
        }
    };
    let exported = call("export_curation_dataset", json!({})).await;
    let payload = &exported["result"]["structuredContent"];
    assert_eq!(payload["ok"], true, "{exported}");
    assert_eq!(payload["candidate_count"], 1);
    assert!(payload.get("candidates").is_none());
    assert_eq!(
        exported["result"]["_meta"]["com.orchid/resultHandling"]["disposition"],
        "artifact-required"
    );
    let uri = |name: &str| {
        exported["result"]["content"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["name"] == name)
            .unwrap()["uri"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let jobs = reqwest::Url::parse(&uri("jobs.jsonl"))
        .unwrap()
        .to_file_path()
        .unwrap();
    let directory = jobs.parent().unwrap();
    let candidate: Value = serde_json::from_str(
        std::fs::read_to_string(&jobs)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    let invalid = call("create_curation", json!({"export_id":payload["export_id"]})).await;
    assert_eq!(invalid["result"]["structuredContent"]["ok"], false);
    assert_eq!(
        invalid["result"]["structuredContent"]["error"]["path"],
        "/items"
    );
    std::fs::write(directory.join("curation-result.json"), json!({
        "schema_version":1,"export_id":payload["export_id"],"title":"OTP synthetic proof", "criteria":"Synthetic Python role",
        "summary":"One synthetic role", "items":[{"job_ref":candidate["job_ref"],"rank":1,"verdict":"shortlist",
            "confidence":"high","reasons":["Synthetic Python evidence"],"research":[]}]
    }).to_string()).unwrap();
    let saved = call("create_curation", json!({"export_id":payload["export_id"]})).await;
    assert_eq!(saved["result"]["structuredContent"]["ok"], true, "{saved}");
    let id = &saved["result"]["structuredContent"]["curation_id"];
    let loaded = call("get_curation", json!({"curation_id":id})).await;
    assert_eq!(
        loaded["result"]["structuredContent"]["curation"]["items"][0]["verdict"],
        "shortlist"
    );
    let saved_path = root
        .join("jobs/curations")
        .join(format!("{}.json", id.as_str().unwrap()));
    assert!(saved_path.is_file());
    drop(owner);
    assert!(!directory.exists(), "MCP host scratch survived shutdown");
    assert!(saved_path.is_file());
    println!(
        "Real Job Agent facade: exported 1, validated retry, saved 1, loaded 1, cleaned scratch."
    );
}
