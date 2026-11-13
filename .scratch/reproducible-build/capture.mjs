import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { copyFileSync, chmodSync, existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const stamp = new Date().toISOString().replaceAll(':', '-');
const evidence = join(root, 'output/reproducible-build', stamp);
const snapshot = mkdtempSync(join(tmpdir(), 'zhiji-source-'));
mkdirSync(evidence, { recursive: true });

/** 只读取版本控制信息，不修改原仓库索引或提交。 */
function git(args) {
  return execFileSync('git', args, { cwd: root, maxBuffer: 16 * 1024 * 1024 });
}

/** 使用完整内容摘要识别快照，不以时间戳代替源码身份。 */
function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

const names = [...new Set(git(['ls-files', '--cached', '--others', '--exclude-standard', '-z']).toString().split('\0').filter(Boolean))].sort();
const entries = [];
const missing = [];
const suspicious = [];
const credentials = [
  ['private-key', /-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----/],
  ['github-token', /\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,})\b/],
  ['openai-project-key', /\bsk-proj-[A-Za-z0-9_-]{40,}\b/],
  ['aws-access-key', /\bAKIA[A-Z0-9]{16}\b/],
];
for (const name of names) {
  const path = join(root, name);
  if (!existsSync(path)) { missing.push(name); continue; }
  const stat = lstatSync(path);
  if (!stat.isFile()) throw Error(`Snapshot requires an ordinary file: ${name}`);
  const bytes = readFileSync(path);
  if (/\.(?:pem|p12|pfx|key)$|(?:^|\/)\.env(?:\.|$)/i.test(name)) suspicious.push({ path: name, reason: 'sensitive-filename' });
  if (!bytes.includes(0)) {
    const text = bytes.toString('utf8');
    for (const [reason, pattern] of credentials) {
      if (pattern.test(text)) suspicious.push({ path: name, reason });
    }
  }
  const destination = join(snapshot, name);
  mkdirSync(dirname(destination), { recursive: true });
  copyFileSync(path, destination);
  chmodSync(destination, stat.mode & 0o777);
  entries.push({ path: name, bytes: bytes.length, mode: stat.mode & 0o777, sha256: sha256(bytes) });
}

/** 复制后再次核对原文件，拒绝把复制期间发生的改动混入同一候选版本。 */
for (const entry of entries) {
  if (sha256(readFileSync(join(root, entry.path))) !== entry.sha256) throw Error(`Source changed while capturing: ${entry.path}`);
}
const manifest = {
  createdAt: new Date().toISOString(),
  root, snapshot, evidence,
  head: git(['rev-parse', 'HEAD']).toString().trim(),
  indexSha256: sha256(git(['ls-files', '--stage', '-z'])),
  sourceSha256: sha256(JSON.stringify(entries)),
  files: entries,
  missing,
  credentialCandidates: suspicious,
  limitations: ['Uncommitted working-tree snapshot', 'Credential patterns are a limited heuristic', 'Toolchain and downloaded dependency caches are shared'],
};
writeFileSync(join(evidence, 'manifest.json'), JSON.stringify(manifest, null, 2) + '\n');
writeFileSync(join(evidence, 'git-status.txt'), git(['status', '--short']));
writeFileSync(join(evidence, 'source.sha256'), entries.map(entry => `${entry.sha256}  ${entry.path}`).join('\n') + '\n');
execFileSync('tar', ['-czf', join(evidence, 'source.tar.gz'), '-C', snapshot, '.']);
writeFileSync(join(evidence, 'archive.sha256'), `${sha256(readFileSync(join(evidence, 'source.tar.gz')))}  source.tar.gz\n`);
console.log(JSON.stringify({ snapshot, evidence, files: entries.length, bytes: entries.reduce((sum, entry) => sum + entry.bytes, 0), sourceSha256: manifest.sourceSha256, missing, credentialCandidates: suspicious }, null, 2));
