import { run } from './context.ts';

/** 运行 Node、格式、严格 Clippy 和全工作区测试，核对实际测试数量。 */
export function verifyRust(env: NodeJS.ProcessEnv, offline: boolean): void {
  const locked = ['--locked', ...(offline ? ['--offline'] : [])];
  run([process.execPath, '--test',
    'tests/font_tools.test.ts',
    'tests/updates.test.ts',
    'tests/verify.test.ts',
    'tests/architecture.test.ts',
    'tests/overseas_candidate_audit.test.ts',
    'tests/overseas_priority_migration.test.ts',
    'tests/china_divisions.test.ts',
    'tests/places_pipeline.test.ts',
    'tests/name_assignments.test.ts',
  ], env);
  run(['cargo', 'fmt', '--all', '--', '--check'], env);
  run(['cargo', 'clippy', ...locked, '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings'], env);
  run(['cargo', 'test', ...locked, '-p', 'zhiji', '--test', 'scene_configuration'], env);
  const output = run(['cargo', 'test', ...locked, '--workspace', '--all-targets', '--all-features', '--', '--test-threads=1'], env);
  const passed = [...output.matchAll(/test result: ok\. (\d+) passed;/g)].reduce((count, match) => count + Number(match[1]), 0);
  if (!passed) throw Error('No workspace tests executed');
  console.log(`Desktop Rust checks passed: ${passed} tests.`);
}
