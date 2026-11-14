import { selected } from './context.ts';

/** 精确选中一项客户端验签测试，核对签名产物与发布协议一致。 */
export function verifyUpdates(env: NodeJS.ProcessEnv, offline: boolean): void {
  const locked = ['--locked', ...(offline ? ['--offline'] : [])];
  const count = selected(['cargo', 'test', ...locked, '-p', 'zhiji', '--bin', 'zhiji', 'updates::tests::packaged_update_signatures_match_client', '--', '--ignored', '--exact'], env, true);
  console.log(`Desktop update artifact checks passed: ${count} test.`);
}
