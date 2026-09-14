import fs from 'node:fs';
import { invokeCargo } from './cargo.mjs';

try {
  const { layout, selection } = JSON.parse(
    fs.readFileSync(process.env.ORCHESTRATOR_CARGO_CONTEXT, 'utf8'),
  );
  await invokeCargo(layout, selection, process.argv.slice(2));
} catch (error) {
  console.error(error.message);
  process.exitCode = error.exitCode ?? 1;
}
