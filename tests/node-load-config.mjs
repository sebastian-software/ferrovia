import process from "node:process";

import { loadConfig } from "../packages/node/index.mjs";

const root = process.cwd();
const fixtures = `${root}/tests/upstream/svgo-v4.0.1/config-loader`;

const absolute = await loadConfig(`${fixtures}/one/two/config.js`);
if (JSON.stringify(absolute) !== JSON.stringify({ plugins: [] })) {
  throw new Error(`absolute load mismatch: ${JSON.stringify(absolute)}`);
}

const relative = await loadConfig("one/two/config.js", fixtures);
if (JSON.stringify(relative) !== JSON.stringify({ plugins: [] })) {
  throw new Error(`relative load mismatch: ${JSON.stringify(relative)}`);
}

const searchedJs = await loadConfig(null, `${fixtures}/one/two`);
if (JSON.stringify(searchedJs) !== JSON.stringify({ plugins: [] })) {
  throw new Error(`search js mismatch: ${JSON.stringify(searchedJs)}`);
}

const searchedMjs = await loadConfig(null, `${fixtures}/mjs`);
if (JSON.stringify(searchedMjs) !== JSON.stringify({ plugins: ["mjs"] })) {
  throw new Error(`search mjs mismatch: ${JSON.stringify(searchedMjs)}`);
}

const searchedCjs = await loadConfig(null, `${fixtures}/cjs`);
if (JSON.stringify(searchedCjs) !== JSON.stringify({ plugins: ["cjs"] })) {
  throw new Error(`search cjs mismatch: ${JSON.stringify(searchedCjs)}`);
}

const missing = await loadConfig(null, `${root}/tests/upstream/svgo-v4.0.1/missing-config-dir`);
if (missing !== null) {
  throw new Error(`expected null for missing search path, got ${JSON.stringify(missing)}`);
}

let invalidThrown = false;
try {
  await loadConfig(`${fixtures}/invalid-null.js`);
} catch (error) {
  invalidThrown = /Invalid config file/.test(String(error));
}

if (!invalidThrown) {
  throw new Error("expected invalid config file error");
}
