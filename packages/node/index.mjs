import { readdirSync } from "node:fs";
import { readFile, stat } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, isAbsolute, join } from "node:path";
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

async function importConfig(configPath) {
  const module = await import(pathToFileURL(configPath).href);
  const config = Object.prototype.hasOwnProperty.call(module, "default")
    ? module.default
    : module;
  if (config == null || typeof config !== "object" || Array.isArray(config)) {
    throw new Error(`Invalid config file "${configPath}"`);
  }
  return config;
}

async function isFile(path) {
  try {
    return (await stat(path)).isFile();
  } catch {
    return false;
  }
}

export async function loadConfig(configPath = null, cwd = process.cwd()) {
  if (configPath != null) {
    const resolved = isAbsolute(configPath) ? configPath : join(cwd, configPath);
    if (resolved.endsWith(".json")) {
      return JSON.parse(await readFile(resolved, "utf8"));
    }
    return importConfig(resolved);
  }

  let dir = cwd;
  while (true) {
    for (const name of ["svgo.config.js", "svgo.config.mjs", "svgo.config.cjs"]) {
      const candidate = join(dir, name);
      if (await isFile(candidate)) {
        return importConfig(candidate);
      }
    }
    const parent = dirname(dir);
    if (parent === dir) {
      return null;
    }
    dir = parent;
  }
}

export async function optimize(svg, config = {}) {
  const { optimize: optimizeNative } = loadNativeBinding();
  return optimizeNative(svg, {
    configJson: JSON.stringify(config),
  });
}
