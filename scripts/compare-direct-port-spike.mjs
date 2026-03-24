import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { basename, join, relative, resolve } from "node:path";
import { spawn } from "node:child_process";
import { tmpdir } from "node:os";
import process from "node:process";

const [, , corpusDirArg, outputDirArg] = process.argv;

if (!corpusDirArg) {
  console.error(
    "usage: node scripts/compare-direct-port-spike.mjs <svgo-test-suite-dir> [output-dir]",
  );
  process.exit(1);
}

const root = process.cwd();
const corpusDir = resolve(corpusDirArg);
const configPath = resolve(root, "tests/fixtures/preset-default/default.config.json");
const outputDir =
  outputDirArg == null
    ? await mkdtemp(join(tmpdir(), "ferrovia-direct-port-spike-"))
    : resolve(outputDirArg);

const hotspotFiles = [
  "W3C_SVG_11_TestSuite/svg/animate-elem-60-t.svg",
  "W3C_SVG_11_TestSuite/svg/animate-elem-62-t.svg",
  "W3C_SVG_11_TestSuite/svg/animate-elem-78-t.svg",
  "W3C_SVG_11_TestSuite/svg/animate-elem-83-t.svg",
  "W3C_SVG_11_TestSuite/svg/animate-elem-91-t.svg",
  "W3C_SVG_11_TestSuite/svg/color-prop-03-t.svg",
  "W3C_SVG_11_TestSuite/svg/conform-viewers-01-t.svg",
  "W3C_SVG_11_TestSuite/svg/coords-trans-01-b.svg",
  "W3C_SVG_11_TestSuite/svg/coords-trans-07-t.svg",
];

await mkdir(outputDir, { recursive: true });

const summary = {
  reference: "svgo@4.0.1",
  configPath,
  outputDir,
  totals: {
    files: hotspotFiles.length,
    baselineMismatch: 0,
    spikeMismatch: 0,
    baselineOnlyWins: 0,
    spikeOnlyWins: 0,
  },
  files: [],
};

for (const [index, relativeFile] of hotspotFiles.entries()) {
  const sourcePath = resolve(corpusDir, relativeFile);
  const input = await readFile(sourcePath, "utf8");
  const tempDir = await mkdtemp(join(tmpdir(), "ferrovia-direct-port-input-"));
  const tempInput = join(tempDir, basename(sourcePath));
  await writeFile(tempInput, input);

  const [baseline, spike, oracle] = await Promise.all([
    runCommand("cargo", [
      "run",
      "-p",
      "ferrovia-cli",
      "--",
      tempInput,
      "--config",
      configPath,
    ]),
    runCommand("cargo", [
      "run",
      "-p",
      "ferrovia-core",
      "--example",
      "direct_port_spike",
      "--",
      tempInput,
      configPath,
    ]),
    runCommand("node", [resolve(root, "scripts/run-svgo-oracle.mjs"), tempInput, configPath]),
  ]);

  const artifactDir = join(outputDir, `${String(index + 1).padStart(2, "0")}-${relativeFile.replaceAll("/", "__")}`);
  await mkdir(artifactDir, { recursive: true });

  const baselineMismatch = normalize(baseline) !== normalize(oracle);
  const spikeMismatch = normalize(spike) !== normalize(oracle);

  if (baselineMismatch) {
    summary.totals.baselineMismatch += 1;
  }
  if (spikeMismatch) {
    summary.totals.spikeMismatch += 1;
  }
  if (baselineMismatch && !spikeMismatch) {
    summary.totals.spikeOnlyWins += 1;
  }
  if (!baselineMismatch && spikeMismatch) {
    summary.totals.baselineOnlyWins += 1;
  }

  await writeFile(join(artifactDir, "input.svg"), input);
  await writeFile(join(artifactDir, "baseline.svg"), baseline);
  await writeFile(join(artifactDir, "spike.svg"), spike);
  await writeFile(join(artifactDir, "svgo.svg"), oracle);
  await writeFile(join(artifactDir, "baseline.diff.txt"), await buildDiff(oracle, baseline));
  await writeFile(join(artifactDir, "spike.diff.txt"), await buildDiff(oracle, spike));

  const fileSummary = {
    file: relativeFile,
    artifactDir,
    baselineMismatch,
    spikeMismatch,
    spikeOutcome: classifyOutcome({ baselineMismatch, spikeMismatch }),
  };
  summary.files.push(fileSummary);
  await writeFile(join(artifactDir, "meta.json"), `${JSON.stringify(fileSummary, null, 2)}\n`);

  await rm(tempDir, { recursive: true, force: true });
}

await writeFile(join(outputDir, "summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
await writeFile(join(outputDir, "summary.md"), renderSummary(summary));

console.log(`OUTPUT ${outputDir}`);
console.log(
  `BASELINE ${summary.totals.baselineMismatch}/${summary.totals.files} SPIKE ${summary.totals.spikeMismatch}/${summary.totals.files}`,
);
console.log(`SPIKE_WINS ${summary.totals.spikeOnlyWins}`);

function normalize(value) {
  return value.trim().replaceAll("\r\n", "\n");
}

function classifyOutcome({ baselineMismatch, spikeMismatch }) {
  if (baselineMismatch && !spikeMismatch) {
    return "spike-win";
  }
  if (!baselineMismatch && spikeMismatch) {
    return "baseline-win";
  }
  if (!baselineMismatch && !spikeMismatch) {
    return "both-match";
  }
  return "both-mismatch";
}

function renderSummary(summary) {
  const lines = [
    "# Direct-Port Spike Comparison",
    "",
    `- Files: ${summary.totals.files}`,
    `- Baseline mismatches: ${summary.totals.baselineMismatch}`,
    `- Spike mismatches: ${summary.totals.spikeMismatch}`,
    `- Spike-only wins: ${summary.totals.spikeOnlyWins}`,
    `- Baseline-only wins: ${summary.totals.baselineOnlyWins}`,
    "",
    "| File | Outcome |",
    "| --- | --- |",
  ];
  for (const file of summary.files) {
    lines.push(`| ${file.file} | ${file.spikeOutcome} |`);
  }
  return `${lines.join("\n")}\n`;
}

function runCommand(cmd, args) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(cmd, args, {
      cwd: root,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("close", (code) => {
      if (code === 0) {
        resolvePromise(normalize(stdout));
      } else {
        reject(new Error(`${cmd} ${args.join(" ")} failed (${code}): ${stderr}`));
      }
    });
  });
}

async function buildDiff(expected, actual) {
  const tempDir = await mkdtemp(join(tmpdir(), "ferrovia-direct-port-diff-"));
  const expectedPath = join(tempDir, "expected.svg");
  const actualPath = join(tempDir, "actual.svg");
  await writeFile(expectedPath, `${normalize(expected)}\n`);
  await writeFile(actualPath, `${normalize(actual)}\n`);
  const result = await runCommandAllowFailure("diff", ["-u", expectedPath, actualPath]);
  await rm(tempDir, { recursive: true, force: true });
  return result.stdout || result.stderr;
}

function runCommandAllowFailure(cmd, args) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(cmd, args, {
      cwd: root,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
    });
    child.on("close", (code) => {
      if (code === 0 || code === 1) {
        resolvePromise({ code, stdout: normalize(stdout), stderr: normalize(stderr) });
      } else {
        reject(new Error(`${cmd} ${args.join(" ")} failed (${code}): ${stderr}`));
      }
    });
  });
}
