import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const missing = [];
const totals = {};
const sources = new Map();

// 空匹配的退出码为一；其他语法或工具错误必须使审计失败。
function matches(language, kind, roots, input) {
  const args = ["run", "--lang", language, "--kind", kind, "--json=compact"];
  if (input === undefined) args.push(...roots);
  else args.push("--stdin");
  try {
    return JSON.parse(execFileSync("ast-grep", args, {
      input, encoding: "utf8", maxBuffer: 32 * 1024 * 1024,
    }));
  } catch (error) {
    if (error.status === 1 && !String(error.stderr).trim()) return [];
    throw error;
  }
}

// 仅接受紧邻声明的中文注释，属性行可位于注释与声明之间。
function check(file, line, language, signature) {
  totals[language] = (totals[language] ?? 0) + 1;
  const lines = sources.get(file);
  let comments = "";
  for (let position = line - 1; position >= 0; position -= 1) {
    const text = lines[position].trim();
    if (/^(?:\/\/|\/\*|\*|@)/u.test(text)) comments += text;
    else break;
  }
  if (!/[\u3400-\u9fff]/u.test(comments)) {
    missing.push({ file, line: line + 1, signature });
  }
}

// 具名声明、类方法和赋给变量的箭头函数需要说明；内联匿名回调不逐个计数。
function audit(language, roots, file, input) {
  const kinds = language === "c"
    ? ["function_definition"]
    : ["function_declaration", "generator_function_declaration", "method_definition", "function_expression", "arrow_function"];
  for (const kind of kinds) {
    for (const match of matches(language, kind, roots, input)) {
      const path = file ?? match.file;
      if (!sources.has(path)) sources.set(path, readFileSync(path, "utf8").split("\n"));
      const line = match.range.start.line;
      if (kind === "arrow_function") {
        const prefix = sources.get(path)[line].slice(0, match.range.start.column);
        if (!/(?:const|let|var)\s+\w+\s*(?::[^=]+)?=\s*$/u.test(prefix)) continue;
      }
      check(path, line, language, match.text.split("\n")[0]);
    }
  }
}

audit("typescript", ["tools", "tests", "website/src", "website/tests"]);
audit("c", ["tests/linux_accessibility.c"]);

const files = execFileSync("rtk", [
  "proxy", "rg", "--files", "tests", "website/src", "-g", "*.swift", "-g", "*.astro",
], { encoding: "utf8" }).trim().split("\n");
for (const file of files) {
  const text = readFileSync(file, "utf8");
  sources.set(file, text.split("\n"));
  if (file.endsWith(".swift")) {
    sources.get(file).forEach((line, index) => {
      if (/^\s*(?:@\w+\s+)*(?:(?:private|static|mutating|override)\s+)*(?:func\s+\w+|init)\s*\(/u.test(line)) {
        check(file, index, "swift", line.trim());
      }
    });
  } else {
    for (const match of text.matchAll(/<script(?:\s[^>]*)?>([\s\S]*?)<\/script>/gu)) {
      const offset = text.slice(0, match.index + match[0].indexOf(">") + 1).split("\n").length - 1;
      const script = "\n".repeat(offset) + match[1];
      audit("typescript", [], file, script);
    }
  }
}

console.log(JSON.stringify({ totals, missing: missing.length, functions: missing }, null, 2));
if (missing.length) process.exitCode = 1;
