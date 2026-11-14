import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { root, run } from './context.ts';

/** 编译并运行 macOS 原生辅助功能检查，应用包由调用方指定或使用默认 release。 */
export function verifyNative(env: NodeJS.ProcessEnv, temporary: string, appBundle?: string): void {
  if (process.platform !== 'darwin') throw Error('Native macOS checks require macOS');
  const bundle = resolve(appBundle ?? join(root, 'target/release/知几.app'));
  const name = run(['plutil', '-extract', 'CFBundleExecutable', 'raw', '-o', '-', join(bundle, 'Contents/Info.plist')], env).trim();
  const executable = join(bundle, 'Contents/MacOS', name);
  if (!existsSync(executable)) throw Error(`Missing bundle executable: ${executable}`);
  const main = join(temporary, 'main.swift');
  writeFileSync(main, readFileSync(join(root, 'tests/macos_accessibility.swift')));
  const binary = join(temporary, 'macos_accessibility');
  run(['swiftc', '-swift-version', '6', '-default-isolation', 'MainActor', '-D', 'ZHIJI_NATIVE_BUNDLE', join(root, 'tests/macos_test_application.swift'), main, '-o', binary], env);
  run([binary, executable], env);
  console.log('Desktop native macOS checks passed: 1 suite.');
}
