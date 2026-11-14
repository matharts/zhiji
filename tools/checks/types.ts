import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { root, run } from './context.ts';

/** 使用根级 TypeScript 对工具和测试做严格检查，不生成文件。 */
export function verifyTypes(): void {
  const tsc = join(root, 'node_modules/typescript/bin/tsc');
  if (!existsSync(tsc)) throw Error('Missing TypeScript tooling; run pnpm install');
  run([process.execPath, tsc, '-p', 'tsconfig.json'], process.env);
  console.log('Desktop TypeScript checks passed.');
}
