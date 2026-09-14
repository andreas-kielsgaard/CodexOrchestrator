import { readFile } from 'node:fs/promises';

const [connectionFile, commandJson = '{"kind":"inspect"}'] = process.argv.slice(2);
if (!connectionFile) {
  console.error(
    'Usage: node scripts/agent-session-ui.mjs <agent-session-navigation.json> [command JSON]',
  );
  process.exit(1);
}
const connection = JSON.parse(await readFile(connectionFile, 'utf8'));
const response = await fetch(connection.endpoint, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${connection.token}` },
  body: JSON.stringify(JSON.parse(commandJson)),
});
const result = await response.json();
console.log(JSON.stringify(result, null, 2));
if (!response.ok || result.error) process.exitCode = 1;
