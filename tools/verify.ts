#!/usr/bin/env node
// 桌面统一检查：node tools/verify.ts [--offline] resources|types|rust|gpu|native|updates
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { parseArguments } from './checks/arguments.ts';
import { verifyGpu } from './checks/gpu.ts';
import { verifyNative } from './checks/native.ts';
import { verifyResources } from './checks/resources.ts';
import { verifyRust } from './checks/rust.ts';
import { verifyTypes } from './checks/types.ts';
import { verifyUpdates } from './checks/updates.ts';
import { runCli } from './cli.ts';

/** 先核验公共资源，再在隔离配置目录中执行对应检查并清理临时环境。 */
function main(): void {
  const { mode, offline, appBundle } = parseArguments(process.argv.slice(2));
  if (mode !== 'types') verifyResources();
  if (mode === 'resources') return;
  if (mode === 'types') {
    verifyTypes();
    return;
  }
  const directory = mkdtempSync(join(tmpdir(), 'zhiji-check-'));
  const env = { ...process.env, ZHIJI_CONFIG_DIR: directory };
  try {
    if (mode === 'rust') verifyRust(env, offline);
    else if (mode === 'gpu') verifyGpu(env, offline);
    else if (mode === 'updates') verifyUpdates(env, offline);
    else verifyNative(env, directory, appBundle);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

runCli(main, import.meta.url);
