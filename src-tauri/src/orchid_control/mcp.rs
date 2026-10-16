use super::OrchidControl;
use crate::{voice_sessions::VoiceSessionApplication,harness_engine::{ManagedMcpUpstreamDescriptor,ManagedMcpUpstreamOwner}};
use std::sync::{Arc,Weak};
use serde_json::{json,Value};
pub(crate) fn start(control:Arc<OrchidControl>,voice:Weak<VoiceSessionApplication>)->Result<(Vec<ManagedMcpUpstreamDescriptor>,Box<dyn ManagedMcpUpstreamOwner>),String>{
 let extension=orchid_core::orchid_control::mcp::ToolExtension{name:"voice_session".into(),definitions:definitions(),invoke:Arc::new(move|caller,invocation,tool,args|{
  let voice=voice.upgrade().ok_or("Voice service unavailable")?;
  match tool {
   "publish"=>voice.publish(caller,args),"delegate_work"=>voice.delegate(caller,invocation,args),
   "report_outcome"=>voice.report_outcome(caller,invocation,args),"observe_sessions"=>voice.observe(caller,args),
   "read_context"=>voice.read_context(caller,args),"report_progress"=>voice.report_progress(caller,args),
   "request_output"=>voice.request_output(caller,args),"route_input"=>voice.route_input(caller,args),
   _=>Err("Unknown voice tool".into())
  }
 })};
 orchid_core::orchid_control::mcp::start(control,Some(Arc::new(extension)))
}
fn definition(name:&str,description:&str,properties:Value,required:&[&str])->Value{json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required}})}
fn definitions()->Vec<Value>{let string=json!({"type":"string"});

        vec![
        definition("delegate_work","Assign work or amend an existing task. Read_context lists taskId and assignmentId. Supply taskId for an amendment and sessionId to reuse its worker. Workers can delegate a child assignment using parentAssignmentId. Returns pending assignment references; active recipients receive steering. Finish your turn while waiting.",json!({"task":string,"title":string,"taskId":string,"sessionId":string,"profileId":string,"workingDirectory":string,"parentAssignmentId":string}),&["task"]),
        definition("report_outcome","Worker: report an assignment outcome and the request revision addressed, with a concise summary, material references, and unresolved issues. This is an agent report, not automatic overall task completion.",json!({"assignmentId":string,"revision":{"type":"integer","minimum":1},"summary":string,"materialRefs":{"type":"array","items":string},"unresolved":{"type":"array","items":string}}),&["assignmentId","revision","summary"]),
        definition("publish","Output publishes deliberate speech; the input summarizer may acknowledge receipt. Retain report material with pendingTopic {id,title,material}; remove a finished topic with completedTopicId. Publication is not proof of playback.",json!({"kind":{"type":"string","enum":["acknowledgement","announcement","explanation"]},"text":string,"topicId":string,"completedTopicId":string,"pendingTopic":{"type":"object","properties":{"id":string,"title":string,"material":string},"required":["id","title","material"]}}),&["kind","text"]),
        definition("observe_sessions","Observe terminal results from selected sessions anywhere in Orchid. Replaces the explicit observation selection; delegated sessions remain observed.",json!({"sessionIds":{"type":"array","items":string}}),&["sessionIds"]),
        definition("read_context","Read retained voice activity, accepted input, task handoffs, publications, and playback. Optionally select activity IDs; use orchid_control.read_session for detailed referenced sessions.",json!({"activityIds":{"type":"array","items":string}}),&[]),
        definition("report_progress","Send process feedback to the ongoing digest.",json!({"text":string}),&["text"]),
        definition("request_output","Digest only: request output with a brief reason why now is a good time and relevant activity or topic references.",json!({"reason":string,"activityIds":{"type":"array","items":string},"topicId":string}),&["reason"]),
        definition("route_input","Output only: forward a misclassified direct reaction to the stronger work router using the original accepted requestId and a brief reason.",json!({"requestId":string,"reason":string}),&["requestId","reason"]),
    ]

}
