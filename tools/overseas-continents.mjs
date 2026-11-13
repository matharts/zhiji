// 按 CLDR 的 M49 式分组补充洲信息，并同步资源清单摘要。
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
const source = JSON.parse(readFileSync('tools/data-sources/cldr-territory-containment-48.2.0.json', 'utf8')).supplemental.territoryContainment;
const path = 'assets/data/birthplace-countries.json';
const data = JSON.parse(readFileSync(path, 'utf8'));
const mapping = new Map();
/** 递归展开区域包含关系，国家映射出现歧义时拒绝写出。 */
function visit(code, continent) {
  const item = source[code];
  if (item?._contains) {
    for (const child of item._contains) visit(child, continent);
  } else if (/^[A-Z]{2}$/.test(code)) {
    const prior = mapping.get(code);
    if (prior && prior !== continent) throw Error(`Ambiguous continent for ${code}`);
    mapping.set(code, continent);
  }
}
for (const [code, continent] of [['142','AS'], ['150','EU'], ['002','AF'], ['003','NA'], ['005','SA'], ['009','OC']]) visit(code, continent);
// CLDR 将南极与偏远岛屿分组，AQ 按地理含义单独归入南极洲。
mapping.set('AQ', 'AN');
for (const country of data.countries) {
  const continent = mapping.get(country.code);
  if (!continent) throw Error(`No CLDR continent for ${country.code}`);
  country.continent = continent;
}
data.continentSource = 'Unicode CLDR 48.2 territory containment; AQ placed in Antarctica';
writeFileSync(path, JSON.stringify(data) + '\n');
const manifest = JSON.parse(readFileSync('assets/manifest.json', 'utf8'));
manifest.sha256[path] = createHash('sha256').update(readFileSync(path)).digest('hex');
writeFileSync('assets/manifest.json', JSON.stringify(manifest, null, 2) + '\n');
console.log(`Grouped ${data.countries.length} territories by continent.`);
