import { readdirSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const require = createRequire(import.meta.url);
const packageDir = dirname(fileURLToPath(import.meta.url));

function loadNativeBinding() {
  const bindingName = readdirSync(packageDir).find((entry) => entry.endsWith(".node"));
  if (!bindingName) {
    throw new Error(
      "No compiled ferrovia native binding found in packages/node. Run `pnpm build:napi` first.",
    );
  }
  return require(join(packageDir, bindingName));
}

export async function loadConfig(configPath) {
  if (!configPath) {
    return null;
  }
  if (configPath.endsWith(".mjs")) {
    const module = await import(
      configPath.startsWith(".")
        ? pathToFileURL(join(process.cwd(), configPath)).href
        : configPath,
    );
    return module.default ?? module;
  }
  return JSON.parse(await readFile(configPath, "utf8"));
}

export async function optimize(svg, config = {}) {
  const { optimize: optimizeNative } = loadNativeBinding();
  return optimizeNative(svg, {
    configJson: JSON.stringify(config),
  });
}
