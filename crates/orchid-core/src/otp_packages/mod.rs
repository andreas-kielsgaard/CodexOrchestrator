pub mod job_agent;
pub mod workflow;
use crate::otp_api::OtpPackage;
use std::sync::Arc;

pub fn instantiate(id: &str) -> Result<Arc<dyn OtpPackage>, String> {
    match id {
        "job_agent" => Ok(Arc::new(job_agent::JobAgentPackage)),
        "workflow" => Ok(Arc::new(workflow::WorkflowPackage)),
        _ => Err(format!("Unknown local OTP package: {id}")),
    }
}
