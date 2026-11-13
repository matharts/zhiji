// 从官方快照重建国内地点选项，包含可选的区县层级。
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
// 按 UTF-8 读取本地快照，解析失败即中止生成。
const read = p => JSON.parse(readFileSync(p, 'utf8'));
const base = 'tools/data-sources/';
const tree = read(base + 'mca-division-tree-2026-11-10.json').data;
const points = read(base + 'mca-birthplace-points-2026-11-10.json').points;
const countyPoints = read(base + 'mca-county-points-2026-11-11.json').points;
const zones = read('assets/data/birthplace-timezones.json');
for (const key of Object.keys(zones)) if (key.startsWith('cn:')) delete zones[key];
// 将省市区编码补齐为统一的六位国内地点 ID。
const id = code => `cn:${code.padEnd(6, '0')}`;
const provinces = tree.children.map(p => {
  const code = p.name === '台湾省' ? '71' : p.code;
  const children = ['81', '82'].includes(code) ? [p] : p.children ?? [];
  return { id: id(code), name: p.name, cities: children.map(c => {
    const matches = points.filter(x => x.code === c.code && x.name === c.name && x.province === p.name);
    if (matches.length !== 1 || !Number.isFinite(matches[0].longitude)) throw Error(`Missing or ambiguous coordinate: ${p.name}/${c.name}`);
    const point = matches[0];
    zones[id(c.code)] = code === '65' ? ['Asia/Shanghai', 'Asia/Urumqi'] : [code === '81' ? 'Asia/Hong_Kong' : code === '82' ? 'Asia/Macau' : 'Asia/Shanghai'];
    const counties = (c.children ?? []).map(d => {
      const matches = countyPoints.filter(x => x.code === d.code && x.name === d.name && x.province === p.name);
      if (matches.length !== 1 || !Number.isFinite(matches[0].longitude)) throw Error(`Missing or ambiguous county coordinate: ${p.name}/${c.name}/${d.name}`);
      zones[id(d.code)] = zones[id(c.code)];
      return { id: id(d.code), name: d.name, longitude: matches[0].longitude };
    });
    return { id: id(c.code), name: c.name, longitude: point.longitude, counties };
  }) };
});
const data = { source: '民政部国家地名信息库现行行政区划树', retrieved: '2026-11-10', source_url: 'https://dmfw.mca.gov.cn/xzqh/getList?code=0&trimCode=true&maxLevel=3', provinces };
writeFileSync('assets/data/birthplace-china.json', JSON.stringify(data) + '\n');
writeFileSync('assets/data/birthplace-timezones.json', JSON.stringify(zones) + '\n');
const manifest = read('assets/manifest.json');
for (const p of ['assets/data/birthplace-china.json', 'assets/data/birthplace-timezones.json']) manifest.sha256[p] = createHash('sha256').update(readFileSync(p)).digest('hex');
writeFileSync('assets/manifest.json', JSON.stringify(manifest,null,2)+'\n');
console.log(`Generated ${provinces.length} provinces and ${provinces.reduce((n,p)=>n+p.cities.length,0)} selectable regions`);
