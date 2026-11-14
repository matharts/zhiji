import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { root, runIn } from './context.ts';

const directory = join(root, 'website');
const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm';

/** 用网站自己的锁文件安装依赖，再检查示例数据、类型和静态构建。 */
export function verifyWebsite(offline: boolean): void {
  if (!existsSync(join(directory, 'pnpm-lock.yaml'))) throw Error('Missing website lockfile');
  const install = ['install', '--frozen-lockfile'];
  if (offline) install.push('--offline');
  for (const args of [install, ['test'], ['check'], ['build']]) {
    runIn(directory, [pnpm, ...args], process.env);
  }
  console.log('Website checks passed.');
}
