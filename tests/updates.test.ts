import assert from 'node:assert/strict';
import test from 'node:test';
import { manifest, targetInfo, verifySignature } from '../tools/release/updates.ts';

test('三端及两种桌面架构使用同一目标协议', () => {
  for (const [platform, system, format] of [['darwin', 'macos', 'app'], ['win32', 'windows', 'nsis'], ['linux', 'linux', 'appimage']]) {
    for (const [arch, targetArch] of [['arm64', 'aarch64'], ['x64', 'x86_64']]) {
      assert.deepEqual(targetInfo(platform, arch), { target: `${system}-${targetArch}`, format });
    }
  }
});

test('更新元数据将版本、目标、格式和资产限制到本项目', () => {
  const release = manifest({ version: '1.2.3', target: 'linux-x86_64', format: 'appimage', filename: 'zhiji.AppImage', signature: 'test', size: 10 });
  assert.equal(release.url, 'https://github.com/matharts/zhiji/releases/download/v1.2.3/zhiji.AppImage');
  assert.equal(release.size, 10);
  for (const patch of [{ version: '1.2.3-beta.1' }, { filename: '../app' }, { format: 'nsis' }, { size: 0 }, { size: 512 * 1024 * 1024 }, { target: 'other' }, { signature: '' }]) {
    assert.throws(() => manifest({ version: '1.2.3', target: 'linux-x86_64', format: 'appimage', filename: 'zhiji.AppImage', signature: 'test', size: 10, ...patch }));
  }
});

test('签名校验拒绝缺失、损坏及非 Minisign 输入', () => {
  assert.equal(verifySignature(Buffer.from('update'), '', ''), false);
  assert.equal(verifySignature(Buffer.from('update'), 'not-a-signature', 'not-a-key'), false);
});
