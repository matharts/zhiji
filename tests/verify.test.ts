import assert from 'node:assert/strict';
import test from 'node:test';
import { parseArguments } from '../tools/checks/arguments.ts';
import { selected } from '../tools/checks/context.ts';

test('检查参数只接受一个模式及完整的可选值', () => {
  assert.deepEqual(parseArguments(['rust']), { mode: 'rust', offline: false });
  assert.deepEqual(parseArguments(['website']), { mode: 'website', offline: false });
  assert.deepEqual(parseArguments(['--offline', 'native', '--app-bundle', 'target/app']), {
    mode: 'native',
    offline: true,
    appBundle: 'target/app',
  });
  assert.deepEqual(parseArguments(['native', '--app-bundle', 'rust']), { mode: 'native', offline: false, appBundle: 'rust' });
  for (const args of [[], ['rust', 'gpu'], ['rust', '--unknown'], ['native', '--app-bundle']]) {
    assert.throws(() => parseArguments(args), /Usage:/);
  }
});

test('测试路径失效时在执行前失败', () => {
  let calls = 0;
  assert.throws(() => selected(['cargo', 'test'], {}, true, () => {
    calls += 1;
    return '0 tests, 0 benchmarks\n';
  }), /Unexpected test selection: 0/);
  assert.equal(calls, 1);
});

test('精确检查拒绝多个候选', () => {
  assert.throws(() => selected([], {}, true, () => 'first: test\nsecond: test\n'), /Unexpected test selection: 2/);
});

test('执行结果必须与列举数量一致且全部通过', () => {
  for (const result of [
    'test result: ok. 0 passed; 0 failed; 0 ignored;',
    'test result: ok. 0 passed; 0 failed; 1 ignored;',
    'test result: FAILED. 0 passed; 1 failed; 0 ignored;',
    'test result: ok. 2 passed; 0 failed; 0 ignored;',
    '',
  ]) {
    assert.throws(() => selected([], {}, true, args => args.includes('--list') ? 'signature: test\n' : result), /Executed test count/);
  }
});

test('验签执行保留精确匹配与 ignored 参数并接受 Windows 换行', () => {
  const args = ['cargo', 'test', 'signature', '--', '--ignored', '--exact'];
  const environment = { ZHIJI_CONFIG_DIR: '/isolated' };
  const calls: string[][] = [];
  const count = selected(args, environment, true, (command, env) => {
    assert.equal(env, environment);
    calls.push(command);
    return command.includes('--list')
      ? 'signature: test\r\n\r\n1 test, 0 benchmarks\r\n'
      : 'test result: ok. 1 passed; 0 failed; 0 ignored;\r\n';
  });
  assert.equal(count, 1);
  assert.deepEqual(calls, [[...args, '--list'], args]);
});
