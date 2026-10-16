// Read-only by default. An explicit target file and prompt opt into one real provider turn.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { randomUUID } from 'node:crypto';
const url=process.env.ORCHID_SERVER_URL?.replace(/\/$/,'');
const token=process.env.ORCHID_SERVICE_TOKEN;
const device=process.env.ORCHID_DEVICE_ID;
assert(url && token && device,'Set ORCHID_SERVER_URL, ORCHID_SERVICE_TOKEN and ORCHID_DEVICE_ID');
async function request(path,body){
  const response=await fetch(`${url}/v1/${path}`,{method:body?'POST':'GET',headers:{Authorization:`Bearer ${token}`,'Content-Type':'application/json'},body:body&&JSON.stringify(body),signal:AbortSignal.timeout(70000)});
  assert(response.ok,`${path}: ${response.status} ${response.ok?'':await response.text()}`);
  return response.json();
}
const command=(method,input)=>request(`devices/${encodeURIComponent(device)}/commands`,{method,input});
const devices=await request('devices');assert(devices.includes(device),'Selected device is offline');
const description=await command('device.description',{});assert.equal(description.id,device);
console.log(JSON.stringify({device:description,profiles:await command('device.profiles',{}),repositories:await command('device.repositories',{})},null,2));
if(process.env.ORCHID_SMOKE_TARGET_FILE){
  const target=JSON.parse(readFileSync(process.env.ORCHID_SMOKE_TARGET_FILE,'utf8'));
  assert.equal(target.execution.deviceId,device,'Target must address the selected host');
  const prompt=process.env.ORCHID_SMOKE_PROMPT;assert(prompt,'Set ORCHID_SMOKE_PROMPT to authorize a real turn');
  const input={submissionId:randomUUID(),sessionId:null,submittedText:prompt,title:'Independent host smoke',attachments:[],context:[],workingDirectory:null,model:null,reasoningMode:null,sandboxMode:null,folderTarget:null,executionSelection:{capabilityProfileId:target.capabilityProfileId,capabilityProfileRevision:target.capabilityProfileRevision,execution:target.execution,workspace:{kind:'existing',target}}};
  const accepted=await command('send',input);
  assert.deepEqual(await command('send',input),accepted,'Retry must retain the accepted invocation');
  console.log(JSON.stringify({accepted},null,2));
  const deadline=Date.now()+240000;
  let complete=false;
  while(Date.now()<deadline){
    const snapshot=await command('snapshot',{sessionId:accepted.sessionId});
    const turn=snapshot.session.history.invocations.find(t=>t.invocation.id===accepted.invocationId);
    if(turn && !['pending','running'].includes(turn.invocation.status)){
      assert.equal(turn.invocation.status,'completed');
      complete=true;break;
    }
    await new Promise(resolve=>setTimeout(resolve,1000));
  }
  assert(complete,'Invocation did not complete');
  for(let attempt=0;attempt<20;attempt++){
    const sessions=await request('sessions');
    if(sessions.some(s=>s.id===accepted.sessionId)){
      const retained=await request(`sessions/${accepted.sessionId}`);
      if(retained.invocations.some(t=>t.invocation.id===accepted.invocationId && t.invocation.status==='completed')){
        console.log('Completed turn retained on the server.');process.exit(0);
      }
    }
    await new Promise(resolve=>setTimeout(resolve,1000));
  }
  throw new Error('Completed turn did not synchronize');
}
