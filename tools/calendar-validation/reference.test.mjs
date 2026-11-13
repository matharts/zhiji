import assert from 'node:assert/strict';
import test from 'node:test';
import { compareCalendar, termDifference } from './run.mjs';
import { assemble, numberedLines, prepare, selfTest } from './reference.mjs';

// 用参考摘录及负向对照核验缺行、非法表格和篡改均会被识别。
test('official source parsing rejects missing lines, invalid tables and tampering controls', () => {
  const { packed } = prepare();
  selfTest(packed);
});
// 保留 PDF 摘录的空行位置，避免行号与相邻内容错位。
test('blank numbered PDF lines are not swallowed into adjacent content', () => {
  assert.deepEqual(numberedLines('Total lines: 3\nL0@P0: first\nL1@P0: \nL2@P0: third'), ['first', '', 'third']);
});
// 缺少年份必须失败，不能将空数据报告视为验证成功。
test('missing calendar years fail instead of producing an empty successful report', () => {
  assert.throws(() => assemble(new Map()), /missing year/);
});
// 月份未知时仅跳过该字段，年份和日期差异仍须参与比较。
test('unknown month is preserved while year and day are still compared', () => {
  const rows = [[1901, 1, 1, 1900, 0, 11]];
  const actual = [{ date: [1901, 1, 1], lunar: [1900, 11, 11], back: [1901, 1, 1] }];
  assert.equal(compareCalendar(rows, actual).differences.length, 0);
  actual[0].lunar[2]++;
  assert.equal(compareCalendar(rows, actual).differences.length, 1);
});
// 闰月符号错误属于结果差异，缺少引擎输出则属于验证流程失败。
test('signed leap month and missing outputs are distinguishable', () => {
  const rows = [[2023, 3, 22, 2023, -2, 1]];
  const actual = [{ date: [2023, 3, 22], lunar: [2023, 2, 1], back: [2023, 3, 22] }];
  assert.equal(compareCalendar(rows, actual).differences.length, 1);
  assert.throws(() => compareCalendar(rows, []), /missing engine outputs/);
});
// 节气跨日时仍按真实时间差比较，不能只比较钟面分钟数。
test('a five minute reference shift is detected across a day boundary', () => {
  assert.equal(termDifference([2024, 2, 5, 0, 1, 0], [2024, 2, 4, 23, 56, 0]), 300);
});
