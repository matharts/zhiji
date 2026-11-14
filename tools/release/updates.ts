// 三端共用的 cargo-packager 签名、更新资产和发布元数据工具；不自动发布。
import { createHash, createPublicKey, verify } from 'node:crypto';
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { runCli } from '../cli.ts';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const localKey = join(homedir(), '.config/zhiji-updater/updater.key');
const publicKeyFile = join(root, 'packaging/updates/public-key.txt');

interface ReleaseInput {
  version: string;
  target: string;
  format: string;
  filename: string;
  signature: string;
  size: number;
}

interface ReleaseManifest {
  version: string;
  target: string;
  format: string;
  size: number;
  signature: string;
  url: string;
}

interface RunOptions {
  env?: NodeJS.ProcessEnv;
}

/** 参数数组避免路径或版本被解释为 shell，密钥只通过环境或文件传入。 */
function run(command: string, args: string[], options: RunOptions = {}): string {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) throw Error(args.includes('signer')
    ? '签名工具执行失败，请检查密钥配置'
    : `${command} 执行失败：${result.stderr || '请检查工具输出'}`);
  return result.stdout.trim();
}

/** 校验 Minisign 的内容签名及可信注释，确保 CI 私钥与应用公钥配套。 */
export function verifySignature(bytes: Buffer, encodedSignature: string, encodedKey: string): boolean {
  try {
    const key = Buffer.from(Buffer.from(encodedKey.trim(), 'base64').toString().trim().split('\n')[1], 'base64');
    const lines = Buffer.from(encodedSignature.trim(), 'base64').toString().trim().split('\n');
    const signature = Buffer.from(lines[1], 'base64');
    if (key.length !== 42 || signature.length !== 74 || !signature.subarray(2, 10).equals(key.subarray(2, 10))) return false;
    if (!lines[2].startsWith('trusted comment: ')) return false;
    const algorithm = signature.subarray(0, 2).toString();
    if (!['ED', 'Ed'].includes(algorithm)) return false;
    const publicKey = createPublicKey({
      key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), key.subarray(10)]),
      type: 'spki', format: 'der',
    });
    const message = algorithm === 'ED' ? createHash('blake2b512').update(bytes).digest() : bytes;
    return verify(null, message, publicKey, signature.subarray(10))
      && verify(null, Buffer.concat([signature.subarray(10), Buffer.from(lines[2].slice(17))]), publicKey, Buffer.from(lines[3], 'base64'));
  } catch { return false; }
}

/** 目标名称和格式与 cargo-packager-updater 的协议保持一致。 */
export function targetInfo(
  platform: string = process.platform,
  architecture: string = process.arch,
): { target: string; format: string } {
  const systems: Partial<Record<string, string>> = { darwin: 'macos', win32: 'windows', linux: 'linux' };
  const architectures: Partial<Record<string, string>> = { arm64: 'aarch64', x64: 'x86_64', ia32: 'i686' };
  const system = systems[platform];
  const arch = architectures[architecture];
  if (!system || !arch) throw Error('不支持的更新目标');
  const formats: Partial<Record<string, string>> = { macos: 'app', windows: 'nsis', linux: 'appimage' };
  const format = formats[system];
  if (!format) throw Error('不支持的更新格式');
  return { target: `${system}-${arch}`, format };
}

/** 发布版本、文件名及来源严格对应，不能生成跨项目或预发布更新。 */
export function manifest(input: ReleaseInput): ReleaseManifest {
  const { version, target, format, filename, signature, size } = input;
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw Error('仅允许三段正式版本');
  if (!/^(macos|windows|linux)-(aarch64|x86_64|i686)$/.test(target)) throw Error('无效更新目标');
  const expected = target.startsWith('macos-') ? 'app' : target.startsWith('windows-') ? 'nsis' : 'appimage';
  if (format !== expected || !/^[A-Za-z0-9._-]+$/.test(filename)) throw Error('格式或文件名不匹配');
  if (!Number.isSafeInteger(size) || size <= 0 || size > 256 * 1024 * 1024) throw Error('更新包大小超限');
  if (!signature) throw Error('缺少更新包签名');
  return { version, target, format, size, signature, url: `https://github.com/matharts/zhiji/releases/download/v${version}/${filename}` };
}

/** 只创建新密钥，不覆盖现有密钥；私钥文件位于仓库外且仅当前用户可读。 */
function keygen(): void {
  mkdirSync(dirname(localKey), { recursive: true, mode: 0o700 });
  chmodSync(dirname(localKey), 0o700);
  if (!existsSync(localKey)) {
    run('cargo', ['packager', 'signer', 'generate', '--ci', '--path', localKey]);
  }
  chmodSync(localKey, 0o600);
  console.log(`签名私钥：${localKey}\n公钥：${localKey}.pub`);
}

/** 与 cargo-packager 共用密钥格式；签名后立即用编译时公钥独立复核。 */
function sign(path: string, key: string): string {
  const privateKey = process.env.CARGO_PACKAGER_SIGN_PRIVATE_KEY || localKey;
  if (!process.env.CARGO_PACKAGER_SIGN_PRIVATE_KEY && !existsSync(privateKey)) throw Error('缺少签名私钥，请先运行 keygen');
  run('cargo', ['packager', 'signer', 'sign', path], {
    env: { ...process.env, CARGO_PACKAGER_SIGN_PRIVATE_KEY: privateKey },
  });
  const signature = readFileSync(`${path}.sig`, 'utf8').trim();
  if (!verifySignature(readFileSync(path), signature, key)) throw Error('签名密钥与应用公钥不一致');
  return signature;
}

/** 构建产物转换为三端更新包，签名元数据将版本、架构和包签名绑定。 */
function prepare(): void {
  const { target, format } = targetInfo();
  const version = /^version\s*=\s*"([^"]+)"/m.exec(readFileSync(join(root, 'Cargo.toml'), 'utf8'))?.[1];
  if (!version || !/^\d+\.\d+\.\d+$/.test(version)) throw Error('Cargo 版本不是正式三段版本');
  if (process.env.GITHUB_REF_NAME?.startsWith('v') && process.env.GITHUB_REF_NAME !== `v${version}`) throw Error('发布 tag 与应用版本不一致');
  const output = join(root, 'target/updates');
  const build = join(root, 'target/release');
  const key = readFileSync(publicKeyFile, 'utf8').trim();
  mkdirSync(output, { recursive: true });
  let filename: string;
  if (format === 'app') {
    const app = join(build, '知几.app');
    run('/usr/bin/codesign', ['--verify', '--deep', '--strict', app]);
    filename = `zhiji-${version}-${target}.app.tar.gz`;
    run('/usr/bin/tar', ['--no-xattrs', '-czf', join(output, filename), '-C', build, '知几.app'], { env: { ...process.env, COPYFILE_DISABLE: '1' } });
  } else {
    const extension = format === 'nsis' ? '-setup.exe' : '.AppImage';
    const candidates = readdirSync(build).filter(name => name.endsWith(extension));
    if (candidates.length !== 1) throw Error(`预期一个 ${extension} 安装包，实际 ${candidates.length}`);
    filename = `zhiji-${version}-${target}${format === 'nsis' ? '-setup.exe' : '.AppImage'}`;
    copyFileSync(join(build, candidates[0]), join(output, filename));
  }
  const artifact = join(output, filename);
  const signature = sign(artifact, key);
  const bytes = readFileSync(artifact);
  const payload = JSON.stringify(manifest({ version, target, format, filename, signature, size: bytes.length }));
  const temporary = mkdtempSync(join(tmpdir(), 'zhiji-manifest-'));
  try {
    const payloadFile = join(temporary, 'payload.json');
    writeFileSync(payloadFile, payload);
    const envelope = { payload, signature: sign(payloadFile, key) };
    writeFileSync(join(output, `latest-${target}.json`), JSON.stringify(envelope));
    writeFileSync(`${artifact}.sha256`, `${createHash('sha256').update(bytes).digest('hex')}  ${filename}\n`);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
  console.log(`已生成 ${target} 签名更新包及更新源，未公开发布`);
}

runCli(() => {
  const command = process.argv[2];
  if (command === 'keygen') keygen();
  else if (command === 'prepare') prepare();
  else throw Error('用法：node tools/release/updates.ts keygen | prepare');
}, import.meta.url);
