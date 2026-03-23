import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join, relative, resolve } from "node:path";
import process from "node:process";
import { spawn } from "node:child_process";

const [, , corpusDirArg, profileArg = "smoke-20", outputDirArg] = process.argv;

if (!corpusDirArg) {
  console.error(
    "usage: node scripts/triage-svgo-corpus.mjs <corpus-dir> [profile-or-limit] [output-dir]",
  );
  process.exit(1);
}

const root = process.cwd();
const corpusDir = resolve(corpusDirArg);
const configPath = resolve(root, "tests/fixtures/preset-default/default.config.json");
const profile = parseProfile(profileArg);
const outputDir =
  outputDirArg === undefined
    ? await mkdtemp(join(tmpdir(), `ferrovia-triage-${profile.label}-`))
    : resolve(outputDirArg);

const files = await collectSvgFiles(corpusDir);
files.sort();
const selected =
  profile.limit === null ? files : files.slice(0, profile.limit);

await mkdir(outputDir, { recursive: true });

const summary = {
  profile: profile.label,
  corpusDir,
  configPath,
  outputDir,
  selectedFiles: selected.length,
  mismatches: 0,
  clusters: {},
  files: [],
};

for (const [index, file] of selected.entries()) {
  const svg = await readFile(file, "utf8");
  const tempDir = await mkdtemp(join(tmpdir(), "ferrovia-triage-"));
  const inputPath = join(tempDir, basename(file));
  await writeFile(inputPath, svg);

  const [ferrovia, oracle] = await Promise.all([
    runCargoOptimize(inputPath, configPath),
    runNodeOracle(inputPath, configPath),
  ]);

  if (ferrovia !== oracle) {
    const relativeFile = relative(corpusDir, file);
    const artifactDir = join(
      outputDir,
      `${String(index + 1).padStart(4, "0")}-${sanitizePath(relativeFile)}`,
    );
    const diff = await buildDiff(oracle, ferrovia, artifactDir);
    const clusters = classifyMismatch({ ferrovia, oracle, diff });

    await mkdir(artifactDir, { recursive: true });
    await writeFile(join(artifactDir, "input.svg"), svg);
    await writeFile(join(artifactDir, "ferrovia.svg"), ferrovia);
    await writeFile(join(artifactDir, "svgo.svg"), oracle);
    await writeFile(join(artifactDir, "diff.txt"), diff);
    await writeFile(
      join(artifactDir, "meta.json"),
      `${JSON.stringify(
        {
          file,
          relativeFile,
          profile: profile.label,
          clusters,
        },
        null,
        2,
      )}\n`,
    );

    summary.mismatches += 1;
    for (const cluster of clusters) {
      summary.clusters[cluster] = (summary.clusters[cluster] ?? 0) + 1;
    }
    summary.files.push({
      file,
      relativeFile,
      artifactDir,
      clusters,
    });

    console.log(`FILE ${file}`);
    console.log(`CLUSTERS ${clusters.join(", ")}`);
    console.log(`ARTEFACT ${artifactDir}`);
    console.log(`FERROVIA ${summarize(ferrovia)}`);
    console.log(`SVGO     ${summarize(oracle)}`);
    console.log("");
  }

  await rm(tempDir, { recursive: true, force: true });
}

const sortedClusters = Object.entries(summary.clusters).sort((left, right) => right[1] - left[1]);
await writeFile(join(outputDir, "summary.json"), `${JSON.stringify(summary, null, 2)}\n`);
await writeFile(
  join(outputDir, "summary.md"),
  renderSummaryMarkdown(summary, sortedClusters),
);

console.log(`SUMMARY ${summary.mismatches}/${selected.length} mismatches`);
console.log(`OUTPUT  ${outputDir}`);
for (const [cluster, count] of sortedClusters) {
  console.log(`CLUSTER ${cluster} ${count}`);
}

async function collectSvgFiles(dir) {
  const { readdir } = await import("node:fs/promises");
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectSvgFiles(path)));
    } else if (entry.isFile() && entry.name.toLowerCase().endsWith(".svg")) {
      files.push(path);
    }
  }
  return files;
}

function parseProfile(value) {
  if (value === "smoke-20") {
    return { label: value, limit: 20 };
  }
  if (value === "sample-100") {
    return { label: value, limit: 100 };
  }
  if (value === "milestone-500") {
    return { label: value, limit: 500 };
  }
  if (value === "full-corpus") {
    return { label: value, limit: null };
  }

  const limit = Number.parseInt(value, 10);
  if (Number.isNaN(limit) || limit <= 0) {
    throw new Error(`unknown profile or invalid limit: ${value}`);
  }
  return { label: `limit-${limit}`, limit };
}

function runCargoOptimize(inputPath, configPath) {
  return runCommand("cargo", [
    "run",
    "-p",
    "ferrovia-cli",
    "--",
    inputPath,
    "--config",
    configPath,
  ]);
}

function runNodeOracle(inputPath, configPath) {
  return runCommand("node", [
    resolve(root, "scripts/run-svgo-oracle.mjs"),
    inputPath,
    configPath,
  ]);
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

function runCommandWithStatus(cmd, args) {
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

async function buildDiff(oracle, ferrovia, artifactDir) {
  const diffDir = await mkdtemp(join(tmpdir(), "ferrovia-triage-diff-"));
  const oraclePath = join(diffDir, "svgo.svg");
  const ferroviaPath = join(diffDir, "ferrovia.svg");
  await writeFile(oraclePath, `${oracle}\n`);
  await writeFile(ferroviaPath, `${ferrovia}\n`);
  const result = await runCommandWithStatus("diff", ["-u", oraclePath, ferroviaPath]);
  await rm(diffDir, { recursive: true, force: true });
  if (result.code === 0) {
    return "";
  }
  return result.stdout || result.stderr || `diff reported mismatch for ${artifactDir}`;
}

function normalize(value) {
  return value.trim().replaceAll("\r\n", "\n");
}

function summarize(value) {
  return value.replace(/\s+/g, " ").slice(0, 220);
}

function sanitizePath(value) {
  return value.replaceAll("/", "__");
}

function classifyMismatch({ ferrovia, oracle, diff }) {
  const clusters = [];
  if (containsForeignDescriptionSubtree(ferrovia, oracle)) {
    clusters.push("foreign-descriptive-subtree-retained");
  }
  if (containsNamespaceXlinkDelta(ferrovia, oracle)) {
    clusters.push("namespace-xlink");
  }
  if (containsTransformFoldingDelta(ferrovia, oracle, diff)) {
    clusters.push("transform-folding-and-shape-normalization");
  }
  if (containsSingleQuoteDelta(ferrovia, oracle)) {
    clusters.push("serializer-quote-normalization");
  }
  if (clusters.length === 0) {
    clusters.push("unclassified");
  }
  return clusters;
}

function containsForeignDescriptionSubtree(ferrovia, oracle) {
  return (
    /<d:(testDescription|operatorScript|passCriteria)\b[^>]*>\s*</.test(ferrovia) &&
    /<d:(testDescription|operatorScript|passCriteria)\b[^>]*\/>/.test(oracle)
  );
}

function containsNamespaceXlinkDelta(ferrovia, oracle) {
  return ferrovia.includes("xmlns:xlink=") !== oracle.includes("xmlns:xlink=");
}

function containsTransformFoldingDelta(ferrovia, oracle, diff) {
  return (
    /transform=['"]translate\(/.test(ferrovia) &&
    !/transform=['"]translate\(/.test(oracle) &&
    diff.includes("@@")
  );
}

function containsSingleQuoteDelta(ferrovia, oracle) {
  return /[ =]'/.test(ferrovia) || /\w='[^']*'/.test(ferrovia)
    ? !/\w='[^']*'/.test(oracle)
    : false;
}

function renderSummaryMarkdown(summary, sortedClusters) {
  const lines = [
    "# Corpus Triage Summary",
    "",
    `- Profile: \`${summary.profile}\``,
    `- Corpus dir: \`${summary.corpusDir}\``,
    `- Selected files: \`${summary.selectedFiles}\``,
    `- Mismatches: \`${summary.mismatches}\``,
    `- Output dir: \`${summary.outputDir}\``,
    "",
    "## Top Clusters",
  ];

  if (sortedClusters.length === 0) {
    lines.push("- none");
  } else {
    for (const [cluster, count] of sortedClusters) {
      lines.push(`- \`${cluster}\`: ${count}`);
    }
  }

  lines.push("", "## Mismatch Files");
  if (summary.files.length === 0) {
    lines.push("- none");
  } else {
    for (const entry of summary.files) {
      lines.push(
        `- \`${entry.relativeFile}\` -> ${entry.clusters.join(", ")} (${entry.artifactDir})`,
      );
    }
  }

  lines.push("");
  return `${lines.join("\n")}\n`;
}
