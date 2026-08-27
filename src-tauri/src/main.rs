fn main() {
    if codex_orchestrator_lib::run_harness_engine_sidecar_if_requested() {
        return;
    }
    if let Some(result) = codex_orchestrator_lib::run_workflow_cli_if_requested() {
        if let Err(error) = result {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    codex_orchestrator_lib::run()
}
