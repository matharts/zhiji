// 统计海外显示名称是否含汉字，不据此判定译名质量；只输出报告，不修改目录。
import { readFileSync } from 'node:fs';
const world = JSON.parse(readFileSync('assets/data/birthplace-regions.json', 'utf8'));
const places = world.places.filter(place => !['CN', 'HK', 'MO', 'TW'].includes(place[4]));
const missing = places.filter(place => !/\p{Script=Han}/u.test(place[1]));
const countries = {};
for (const place of missing) countries[place[4]] = (countries[place[4]] ?? 0) + 1;
const summary = { total: places.length, containsHan: places.length - missing.length, withoutHan: missing.length, missingByCountry: Object.fromEntries(Object.entries(countries).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))) };
if (process.argv.includes('--places')) summary.places = missing.map(place => ({ id: place[0], name: place[1], english: place[2], region: place[3], country: place[4], longitude: place[5] }));
console.log(JSON.stringify(summary, null, 2));
