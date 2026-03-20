import { readFile } from "node:fs/promises";
import process from "node:process";
import { optimize } from "svgo";

const [, , svgPath, configPath] = process.argv;

if (!svgPath) {
  console.error("usage: node scripts/run-svgo-oracle.mjs <svg-path> [config-json-path]");
  process.exit(1);
}

const svg = await readFile(svgPath, "utf8");
const config = configPath
  ? JSON.parse(await readFile(configPath, "utf8"))
  : {};

const result = optimize(svg, {
  path: svgPath,
  ...config,
});

process.stdout.write(result.data);

