import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { names, output, prepare, root } from './reference.mjs';

const resultDir = join(output, 'results');
const manifest = join(root, 'tools/calendar-validation/Cargo.toml');
const binary = join(root, 'tools/calendar-validation/target/release', process.platform === 'win32' ? 'zhiji-calendar-validation.exe' : 'zhiji-calendar-validation');
// 对验证报告中的有限 JSON 结构做严格字段顺序一致比较。
const equal = (a, b) => JSON.stringify(a) === JSON.stringify(b);
// 参考表均使用同一 UTC 表示法，统一换算后再计算时间差。
const milliseconds = a => Date.UTC(a[0], a[1] - 1, ...a.slice(2));
// 返回实际时刻减参考时刻的秒数，保留差异方向。
export const termDifference = (actual, reference) => (milliseconds(actual) - milliseconds(reference)) / 1000;

/** 逐行核对输入身份、农历差异和往返异常，未知农历月不强行比较。 */
export function compareCalendar(rows, actual) {
  assert.equal(actual.length, rows.length, 'missing engine outputs');
  const differences = [], roundtripIssues = [];
  rows.forEach((row, i) => {
    const item = actual[i], date = row.slice(0, 3), reference = row.slice(3);
    assert(equal(item.date, date), 'engine output order or input lost');
    assert(item.lunar.length === 3 && item.lunar.every(Number.isInteger));
    if (!equal(item.back, date)) roundtripIssues.push({ date, back: item.back });
    if (item.lunar[0] !== reference[0] || (reference[1] !== 0 && item.lunar[1] !== reference[1]) || item.lunar[2] !== reference[2]) differences.push({ date, reference, actual: item.lunar });
  });
  return { differences, roundtripIssues };
}

/** 用独立输入输出文件调用验证二进制，设置超时并读取结构化结果。 */
function invoke(mode, input) {
  const inPath = join(resultDir, `${mode}-input.json`), outPath = join(resultDir, `${mode}-actual.json`);
  if (input) writeFileSync(inPath, JSON.stringify(input));
  execFileSync(binary, [mode, input ? inPath : '-', outPath], { cwd: root, stdio: 'inherit', timeout: 600000 });
  return JSON.parse(readFileSync(outPath));
}

/** 构建固定后端工具，执行独立参考、负向对照和全范围边界扫描并输出报告。 */
export function evaluate() {
  mkdirSync(resultDir, { recursive: true });
  const { data, summary: reference } = prepare();
  execFileSync('cargo', ['build', '--locked', '--offline', '--release', '--manifest-path', manifest], { cwd: root, stdio: 'inherit' });
  const actual = invoke('calendar', data.days);
  const calendar = compareCalendar(data.days, actual);
  const altered = structuredClone(actual);
  altered[0].lunar[2] += 1;
  assert.equal(compareCalendar(data.days, altered).differences.length, calendar.differences.length + 1, 'negative comparison control failed');
  writeFileSync(join(resultDir, 'calendar-differences.json'), JSON.stringify(calendar, null, 2) + '\n');
  console.log(`Calendar comparison: ${data.days.length} dates, ${calendar.differences.length} disagreements, ${calendar.roundtripIssues.length} round-trip issues`);

  const termKeys = data.terms.map(([y, i]) => [y, i]);
  const termActual = invoke('terms', termKeys);
  assert.equal(termActual.length, termKeys.length);
  const termsByKey = new Map(termActual.map(x => [`${x.year}/${x.index}`, x]));
  assert.equal(termsByKey.size, termKeys.length, 'duplicate term output');
  const termDates = data.terms.flatMap(([y, i, m, d]) => {
    const item = termsByKey.get(`${y}/${i}`);
    assert(item && item.actual.length === 6 && item.actual[0] === y, 'wrong term identity');
    return equal(item.actual.slice(0, 3), [y, m, d]) ? [] : [{ year: y, name: names[i], reference: [y, m, d], actual: item.actual }];
  });
  const calendarTermDates = data.terms.flatMap(([y, i, m, d]) => {
    const item = termsByKey.get(`${y}/${i}`);
    return equal(item.calendar_day, [y, m, d]) ? [] : [{ year: y, name: names[i], reference: [y, m, d], actual: item.calendar_day }];
  });
  const minutes = data.cwa.map(([y, i, m, d, h, min]) => {
    const item = termsByKey.get(`${y}/${i}`);
    const reference = [y, m, d, h, min, 0];
    return { year: y, name: names[i], reference, actual: item.actual, difference_seconds: termDifference(item.actual, reference) };
  });
  assert(minutes.every(x => Number.isFinite(x.difference_seconds)));
  const minuteMax = minutes.reduce((a, b) => Math.abs(a.difference_seconds) >= Math.abs(b.difference_seconds) ? a : b);
  writeFileSync(join(resultDir, 'term-instant-date-differences.json'), JSON.stringify(termDates, null, 2) + '\n');
  writeFileSync(join(resultDir, 'term-calendar-date-differences.json'), JSON.stringify(calendarTermDates, null, 2) + '\n');
  writeFileSync(join(resultDir, 'term-minute-comparison.json'), JSON.stringify(minutes, null, 2) + '\n');
  console.log(`Solar terms: ${termKeys.length} date references, ${calendarTermDates.length} calendar-date and ${termDates.length} instant-projected-date disagreements; ${minutes.length} minute references, maximum ${Math.abs(minuteMax.difference_seconds)} seconds from printed minute`);

  console.log('Scanning every representable solar and lunar date in the numerical year range...');
  const bounds = invoke('boundaries');
  assert.equal(bounds.solar_month_boundary_checks, 9999 * 12 * 2, 'incomplete solar boundary scan');
  assert(bounds.lunar_month_boundary_checks > 9999 * 12 * 2, 'incomplete lunar boundary scan');
  const gregorianDays = 9999 * 365 + Math.floor(9999 / 4) - Math.floor(9999 / 100) + Math.floor(9999 / 400);
  const julianExtraBefore1582 = Math.floor(1581 / 100) - Math.floor(1581 / 400);
  assert.equal(bounds.solar_daily_checks, gregorianDays + julianExtraBefore1582 - 10, 'incomplete hybrid-calendar daily scan');
  assert(bounds.lunar_daily_checks > 354 * 9999, 'incomplete lunar daily scan');
  const groups = [];
  for (const difference of calendar.differences) {
    const group = groups.at(-1);
    if (group && milliseconds(difference.date) - milliseconds(group.end) === 86400000) { group.end = difference.date; group.count++; }
    else groups.push({ start: difference.date, end: difference.date, count: 1 });
  }
  const summary = {
    library: 'tyme4rs 1.5.1', reference,
    calendar: { compared: actual.length, differences: calendar.differences.length, groups, roundtripIssues: calendar.roundtripIssues.length },
    terms: { dateReferences: termKeys.length, calendarDifferingDates: calendarTermDates.length, instantProjectionDifferingDates: termDates.length, minuteReferences: minutes.length, over30Seconds: minutes.filter(x => Math.abs(x.difference_seconds) > 30).length, over60Seconds: minutes.filter(x => Math.abs(x.difference_seconds) > 60).length, maximum: minuteMax },
    boundaries: bounds,
  };
  writeFileSync(join(resultDir, 'summary.json'), JSON.stringify(summary, null, 2) + '\n');
  console.log(JSON.stringify({ calendar: summary.calendar, terms: summary.terms, boundaries: { solar: bounds.solar_month_boundary_checks, lunar: bounds.lunar_month_boundary_checks, outsideDomainPanics: bounds.boundary_panics.length, issues: bounds.issues.length, endpoints: bounds.endpoints } }));
  console.log(`Artifacts: ${resultDir}`);
  console.log('CALENDAR_EVALUATION_COMPLETE');
  return summary;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) evaluate();
