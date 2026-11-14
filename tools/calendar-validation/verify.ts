import { execFileSync } from 'node:child_process';
import { join } from 'node:path';
import { root } from './references.ts';

const manifest = join(root, 'tools/calendar-validation/Cargo.toml');
for (const args of [
  ['fmt', '--manifest-path', manifest, '--', '--check'],
  ['clippy', '--locked', '--offline', '--manifest-path', manifest, '--all-targets', '--', '-D', 'warnings'],
  ['test', '--locked', '--offline', '--manifest-path', manifest],
]) execFileSync('cargo', args, { cwd: root, stdio: 'inherit' });
execFileSync(process.execPath, ['--test', 'tools/calendar-validation/references.test.ts'], { cwd: root, stdio: 'inherit' });
console.log('HARNESS_CHECK_COMPLETE');
