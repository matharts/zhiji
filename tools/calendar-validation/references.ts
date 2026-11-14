import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gunzipSync } from 'node:zlib';
import { runCli } from '../cli.ts';

export type DayRow = [year: number, month: number, day: number, lunarYear: number, lunarMonth: number, lunarDay: number];
export type TermRow = [year: number, index: number, month: number, day: number];
export type CwaTerm = [year: number, index: number, month: number, day: number, hour: number, minute: number];

export interface ParsedHko {
  days: DayRow[];
  terms: TermRow[];
}

interface ReferenceEntry {
  file: string;
  bytes: number;
  sha256: string;
}

interface ReferenceManifest {
  sources: ReferenceEntry[];
}

interface SourceRecord {
  year: number;
  kind: 'hko' | 'cwa';
  sha256: string;
  url: string | undefined;
}

export interface ReferenceData {
  days: DayRow[];
  terms: TermRow[];
  cwa: CwaTerm[];
  sources: SourceRecord[];
}

export const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export const output = join(root, 'target/calendar-validation');
export const names = ['小寒', '大寒', '立春', '雨水', '驚蟄', '春分', '清明', '穀雨', '立夏', '小滿', '芒種', '夏至', '小暑', '大暑', '立秋', '處暑', '白露', '秋分', '寒露', '霜降', '立冬', '小雪', '大雪', '冬至'];
const monthNames = ['正', '二', '三', '四', '五', '六', '七', '八', '九', '十', '十一', '十二'];
const dayNames = ['初一', '初二', '初三', '初四', '初五', '初六', '初七', '初八', '初九', '初十', '十一', '十二', '十三', '十四', '十五', '十六', '十七', '十八', '十九', '二十', '廿一', '廿二', '廿三', '廿四', '廿五', '廿六', '廿七', '廿八', '廿九', '三十'];
const monthMap = new Map<string, number>(monthNames.flatMap((x, i): [string, number][] => [[x + '月', i + 1], ['閏' + x + '月', -i - 1]]));
const dayMap = new Map<string, number>(dayNames.map((x, i): [string, number] => [x, i + 1]));
// 对原始摘录字节求摘要，确认参考数据未被改写。
export const hash = (data: string | Buffer): string => createHash('sha256').update(data).digest('hex');
// 用 UTC 避免本地夏令时影响参考表的公历月长。
export const monthLength = (y: number, m: number): number => new Date(Date.UTC(y, m, 0)).getUTCDate();

/** 校验带页码行号的摘录连续性，并只对解析文本做 Unicode 归一化。 */
export function numberedLines(raw: string, complete = true): string[] {
  const numbered = [...raw.matchAll(/^L(\d+)@P\d+(?:-\d+)?:[ \t]*(.*)$/gm)];
  assert(numbered.length > 0, 'no numbered source lines');
  if (complete) {
    const total = Number(raw.match(/Total lines: (\d+)/)?.[1]);
    assert.equal(numbered.length, total, 'incomplete extraction');
    numbered.forEach((x, i) => assert.equal(Number(x[1]), i, 'missing or duplicate source line'));
  }
  return numbered.map(x => x[2].normalize('NFKC').trim());
}

/** 验证香港天文台来源及完整月序，提取逐日农历和节气日期。 */
export function parseHko(year: number, raw: string): ParsedHko {
  assert(raw.includes(`/${year}.pdf`), `wrong HKO source for ${year}`);
  assert(/https:\/\/www\.(?:hko|weather)\.gov\.hk\//.test(raw), 'not an official HKO host');
  const lines = numberedLines(raw), starts: number[] = [];
  for (let i = 0; i < lines.length; i++) {
    let start;
    if (lines[i] === '農曆日期') start = i + 1;
    else if (lines[i] === '農曆' && lines[i + 1] === '日期') start = i + 2;
    else if (/^\d{1,2}\s+農曆$/.test(lines[i]) && lines[i + 1] === '日期') {
      assert.equal(Number(lines[i].split(/\s+/)[0]), starts.length + 1);
      starts.push(i + 2);
      continue;
    }
    if (start) {
      assert.equal(lines[i - 1], '月', 'wrong Gregorian month header');
      assert.equal(Number(lines[i - 2]), starts.length + 1);
      starts.push(start);
    }
  }
  assert.equal(starts.length, 12, `${year}: Gregorian month rows`);
  let lunarYear = year - 1, lunarMonth = 0, previousDay: number | undefined;
  const days: DayRow[] = [], terms: TermRow[] = [];
  for (let month = 1; month <= 12; month++) {
    let at = starts[month - 1];
    for (let day = 1; day <= monthLength(year, month); day++) {
      let marker: [month: number, width: number] | undefined;
      for (let width = 1; width <= 4; width++) {
        const token = lines.slice(at, at + width).join('');
        const month = monthMap.get(token);
        if (month !== undefined) { marker = [month, width]; break; }
      }
      let lunarDay;
      if (marker) {
        const [next, width] = marker;
        if (previousDay !== undefined) assert([29, 30].includes(previousDay), 'month starts before day 29');
        if (lunarMonth) assert(next < 0 ? lunarMonth > 0 && next === -lunarMonth : next === Math.abs(lunarMonth) % 12 + 1, 'invalid lunar month sequence');
        lunarMonth = next; lunarDay = 1; at += width;
        if (next === 1) lunarYear = year;
      } else {
        let width = 1, token = lines[at];
        if (!dayMap.has(token)) { token = lines.slice(at, at + 2).join(''); width = 2; }
        const parsedDay = dayMap.get(token);
        if (parsedDay === undefined) throw Error(`${year}-${month}-${day}: unknown day token ${token}`);
        lunarDay = parsedDay; at += width;
        if (previousDay !== undefined) assert.equal(lunarDay, previousDay + 1, 'nonconsecutive lunar day');
      }
      previousDay = lunarDay;
      days.push([year, month, day, lunarYear, lunarMonth, lunarDay]);
    }
    const end = month === 12 ? lines.length : starts[month] - 2;
    const tail = lines.slice(at, end).join('').replaceAll(/\s/g, '').split(/附[註注]|備註/)[0];
    const foundNames: string[] = [...tail.matchAll(new RegExp(names.join('|'), 'g'))].map(x => x[0]);
    let foundDays: number[] = [...tail.matchAll(/(\d{1,2})日/g)].map(x => Number(x[1]));
    if (foundDays.length === 0) foundDays = [...tail.matchAll(/:(\d{1,2})(?!\d)/g)].map(x => Number(x[1]));
    assert.deepEqual(foundNames, names.slice((month - 1) * 2, month * 2), `${year}-${month}: term names`);
    assert.equal(foundDays.length, 2, `${year}-${month}: term days`);
    foundDays.forEach((day, k) => {
      assert(day >= 1 && day <= monthLength(year, month));
      terms.push([year, (month - 1) * 2 + k, month, day]);
    });
  }
  return { days, terms };
}

/** 核对中央气象署来源和 UTC+8 时间基准，解析完整二十四节气分钟表。 */
export function parseCwa(year: number, raw: string): CwaTerm[] {
  assert(raw.includes(`https://www.cwa.gov.tw/Data/astronomy/${year}cal.pdf`), 'wrong CWA source');
  // 缓存摘录可能省略无关后页，此处仍强制要求节气表自身完整连续。
  const lines = numberedLines(raw, false);
  const indices = [...raw.matchAll(/^L(\d+)@P\d+(?:-\d+)?:/gm)].map(x => Number(x[1]));
  const heading = lines.findIndex(x => x.includes('一、節氣'));
  assert(heading >= 0 && lines[heading].replaceAll(/\s/g, '').includes('世界時+8小時'), 'missing source time basis');
  for (let i = heading; i < heading + 14; i++) assert.equal(indices[i], indices[heading] + i - heading, 'incomplete term table');
  const pattern = /^(.+?)\s+(?:(\d{1,2})\s*月\s+)?(\d{1,2})\s+(\d{1,2})\s+(\d{1,2})\s+(.+?)\s+(?:(\d{1,2})\s*月\s+)?(\d{1,2})\s+(\d{1,2})\s+(\d{1,2})$/;
  const result: CwaTerm[] = [];
  for (let i = 0; i < 12; i++) {
    const row = lines[heading + 2 + i], m = row?.match(pattern);
    if (!m) throw Error(`${year}: missing term table row ${i}`);
    for (const [group, month, index] of [[1, Math.floor(i / 2) + 1, i], [6, Math.floor(i / 2) + 7, i + 12]] as const) {
      assert.equal(m[group].replaceAll(/\s/g, ''), names[index]);
      if (m[group + 1]) assert.equal(Number(m[group + 1]), month);
      const day: number = Number(m[group + 2]);
      const hour: number = Number(m[group + 3]);
      const minute: number = Number(m[group + 4]);
      assert(day >= 1 && day <= monthLength(year, month) && hour < 24 && minute < 60);
      result.push([year, index, month, day, hour, minute]);
    }
  }
  return result.sort((a, b) => a[1] - b[1]);
}

/** 按 1901 至 2100 年拼接参考数据，检查公历逐日和农历跨年连续性。 */
export function assemble(years: Map<number, ParsedHko>): { days: DayRow[]; terms: TermRow[] } {
  const days: DayRow[] = [], terms: TermRow[] = [];
  let previous: DayRow | undefined;
  for (let year = 1901; year <= 2100; year++) {
    const part = years.get(year);
    assert(part, `missing year ${year}`);
    for (const original of part.days) {
      const row: DayRow = [...original], [y, m, d, ly, , ld] = row;
      if (previous) {
        assert.equal(Date.UTC(y, m - 1, d) - Date.UTC(previous[0], previous[1] - 1, previous[2]), 86400000, 'missing or duplicate civil day');
        if (row[4] === 0 && previous[4]) row[4] = previous[4];
        if (ld !== 1) {
          assert.equal(ld, previous[5] + 1);
          if (row[4] && previous[4]) assert.deepEqual([ly, row[4]], previous.slice(3, 5));
        } else if (previous[4]) {
          assert([29, 30].includes(previous[5]));
          assert.equal(row[4], row[4] < 0 ? -previous[4] : Math.abs(previous[4]) % 12 + 1);
          assert.equal(ly, previous[3] + Number(row[4] === 1));
        }
      }
      days.push(row); previous = row;
    }
    terms.push(...part.terms);
  }
  const expectedDays = (Date.UTC(2101, 0, 1) - Date.UTC(1901, 0, 1)) / 86400000;
  assert.equal(days.length, expectedDays);
  const last = days.at(-1);
  assert(last);
  assert.deepEqual(last.slice(0, 3), [2100, 12, 31]);
  return { days, terms };
}

/** 校验压缩摘录清单及摘要，解析并写出可独立对照的参考数据。 */
export function prepare(): {
  data: ReferenceData;
  packed: Record<string, string>;
  summary: { years: number; days: number; knownMonths: number; hkoTerms: number; cwaTerms: number };
} {
  const base = join(root, 'tools/calendar-validation');
  const manifest = JSON.parse(readFileSync(join(base, 'source-manifest.json'), 'utf8')) as ReferenceManifest;
  const packed = JSON.parse(gunzipSync(readFileSync(join(base, 'fixtures/reference-extracts.json.gz'))).toString('utf8')) as Record<string, string>;
  assert.deepEqual(Object.keys(packed).sort(), manifest.sources.map(x => x.file).sort());
  const years = new Map<number, ParsedHko>(), cwa: CwaTerm[] = [], sources: SourceRecord[] = [];
  for (const entry of manifest.sources) {
    const body = packed[entry.file];
    if (body === undefined) throw Error(`Missing reference extract: ${entry.file}`);
    assert.equal(Buffer.byteLength(body), entry.bytes);
    assert.equal(hash(body), entry.sha256, 'reference content hash mismatch');
    for (const [textYear, raw] of Object.entries(JSON.parse(body) as Record<string, string>)) {
      const year = Number(textYear);
      if (year === 2040 && !entry.file.includes('2040_fix')) continue;
      sources.push({ year, kind: entry.file.startsWith('cwa') ? 'cwa' : 'hko', sha256: hash(raw), url: raw.match(/https:\/\/[^\s)]+\.pdf/)?.[0] });
      if (entry.file.startsWith('cwa')) cwa.push(...parseCwa(year, raw));
      else if (entry.file.includes('2040_fix')) { assert.equal(year, 2040); years.set(year, parseHko(year, raw)); }
      else if (year !== 2040) { assert(!years.has(year), 'duplicate HKO year'); years.set(year, parseHko(year, raw)); }
    }
  }
  assert.equal(years.size, 200);
  const { days, terms } = assemble(years);
  assert.equal(cwa.length, (2027 - 2018 + 1) * names.length);
  assert.equal(terms.length, years.size * names.length);
  mkdirSync(join(output, 'references'), { recursive: true });
  const data: ReferenceData = { days, terms, cwa, sources };
  for (const [key, value] of Object.entries(data)) writeFileSync(join(output, `references/${key}.json`), JSON.stringify(value) + '\n');
  return { data, packed, summary: { years: years.size, days: days.length, knownMonths: days.filter(x => x[4] !== 0).length, hkoTerms: terms.length, cwaTerms: cwa.length } };
}

/** 用删除行、改日期和错误来源作为负向对照，确认解析器会拒绝损坏参考。 */
export function selfTest(packed: Record<string, string>): void {
  const raw = JSON.parse(packed['hko_pdf_extract_1901_1940.json'])['1901'] as string;
  assert.throws(() => numberedLines(raw.replace(/^L10@P0:.*\n/m, '')), /incomplete extraction/);
  assert.throws(() => parseHko(1901, raw.replace('初二', '初三')), /nonconsecutive lunar day/);
  assert.throws(() => parseHko(1902, raw), /wrong HKO source/);
  assert.notEqual(hash(raw), hash(raw + 'altered'));
  const cwa = JSON.parse(packed['cwa_pdf_extract_2018_2027.json'])['2024'] as string;
  assert.throws(() => parseCwa(2024, cwa.replace('16 27', '25 27')));
  assert.throws(() => parseCwa(2024, cwa.replace(/^L50@P1:.*\n/m, '')));
  assert.throws(() => assemble(new Map()), /missing year/);
}

runCli(() => {
  const { packed, summary } = prepare();
  selfTest(packed);
  console.log(JSON.stringify(summary));
  console.log('REFERENCE_CHECK_COMPLETE');
}, import.meta.url);
