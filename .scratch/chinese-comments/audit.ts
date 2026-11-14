import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

// 用语法节点枚举函数，避免把宏调用、注释或字符串中的 fn 当作真实方法。
const roots = process.argv.slice(2);
const matches = ["function_item", "function_signature_item"].flatMap((kind) => JSON.parse(execFileSync("ast-grep", [
  "run", "--lang", "rust", "--kind", kind, "--json=compact",
  ...(roots.length ? roots : ["src", "crates", "tests", "build.rs", "tools/calendar-validation/src"]),
], { maxBuffer: 32 * 1024 * 1024, encoding: "utf8" })));
const cache = new Map();
const missing = [];
for (const match of matches) {
  if (!cache.has(match.file)) cache.set(match.file, readFileSync(match.file, "utf8").split("\n"));
  const lines = cache.get(match.file);
  let position = match.range.start.line - 1;
  let documented = false;
  let attributeDepth = 0;
  while (position >= 0) {
    const line = lines[position].trim();
    if (line.startsWith("//")) {
      documented ||= /[\u3400-\u9fff]/u.test(line);
      position -= 1;
      continue;
    }
    if (line.startsWith("#[") || line.startsWith("#![")) {
      position -= 1;
      continue;
    }
    if (line.endsWith("]") || attributeDepth > 0) {
      attributeDepth += [...line].filter((value) => value === "]").length;
      attributeDepth -= [...line].filter((value) => value === "[").length;
      position -= 1;
      continue;
    }
    break;
  }
  if (!documented) {
    missing.push({
      file: match.file,
      line: match.range.start.line + 1,
      signature: match.text.split("{")[0].trim(),
      body: process.env.COMMENT_BODIES === "1" ? match.text : undefined,
    });
  }
}
console.log(JSON.stringify({ total: matches.length, missing: missing.length, functions: missing }, null, 2));
