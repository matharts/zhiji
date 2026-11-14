import { spawnSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/** 递归枚举实际文件，不把目录作为资源条目。 */
export function* files(dir: string): Generator<string> {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) yield* files(path);
    else if (entry.isFile()) yield path;
  }
}

/** 在仓库根目录执行命令，保留输出并将启动或退出失败向上报告。 */
export function run(args: string[], env: NodeJS.ProcessEnv): string {
  return runIn(root, args, env);
}

/** 在指定目录执行命令，供独立构建的网站复用同一输出与错误约定。 */
export function runIn(cwd: string, args: string[], env: NodeJS.ProcessEnv): string {
  console.log('+ ' + args.map(s => JSON.stringify(s)).join(' '));
  const result = spawnSync(args[0], args.slice(1), { cwd, env, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.error) throw result.error;
  if (result.status !== 0) throw Error(`Command exited ${result.status}: ${args[0]}`);
  return (result.stdout ?? '') + (result.stderr ?? '');
}

/** 先列出待选测试再执行，拒绝零项、数量不符或被忽略的检查。 */
export function selected(
  args: string[],
  env: NodeJS.ProcessEnv,
  exact = false,
  execute: (args: string[], env: NodeJS.ProcessEnv) => string = run,
): number {
  const listing = execute([...args, '--list'], env);
  const chosen = [...listing.matchAll(/^(.+): test\r?$/gm)];
  if (!chosen.length || (exact && chosen.length !== 1)) throw Error(`Unexpected test selection: ${chosen.length}`);
  const output = execute(args, env);
  const counts = [...output.matchAll(/test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)];
  if (counts.length !== 1 || Number(counts[0][1]) !== chosen.length || counts[0][2] !== '0' || counts[0][3] !== '0') throw Error('Executed test count does not match the inspected selection');
  return chosen.length;
}
