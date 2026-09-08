fn main() {
    if codex_orchestrator_lib::run_harness_engine_sidecar_if_requested() {
        return;
    }
    codex_orchestrator_lib::run()
}
