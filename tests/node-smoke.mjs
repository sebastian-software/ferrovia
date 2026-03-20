import { readFile } from "node:fs/promises";
import process from "node:process";

import { optimize } from "../packages/node/index.mjs";

const svg = await readFile(new URL("./fixtures/oracle/remove-comments.svg", import.meta.url), "utf8");
const config = JSON.parse(
  await readFile(new URL("./fixtures/oracle/remove-comments.config.json", import.meta.url), "utf8"),
);

const result = await optimize(svg, config);
const expected =
  '<svg xmlns="http://www.w3.org/2000/svg"><!--!keep legal--><desc>World</desc><g><text>Hi</text></g></svg>';

if (result.data !== expected) {
  console.error("node smoke mismatch");
  console.error("expected:", expected);
  console.error("actual:", result.data);
  process.exit(1);
}
