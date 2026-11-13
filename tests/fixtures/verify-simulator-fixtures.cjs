const fs = require('node:fs');
const input = JSON.parse(fs.readFileSync(process.argv[2] || 'tests/fixtures/simulator-browser-observations.json', 'utf8'));
if (input.isError) throw new Error(input.error);
const ui = typeof input.result === 'string' ? JSON.parse(input.result) : input.observations;
const cases = JSON.parse(fs.readFileSync('tests/fixtures/simulator-oracle.json', 'utf8')).cases;
const mapping = [
  ['male', 'jiazi-male-month-hour'],
  ['female', 'yichou-female-manual-yearly'],
  ['reverse', 'yichou-male-reverse'],
];
let assertions = 0;
for (const [state, id] of mapping) {
  const actual = ui[state];
  const expected = cases.find(c => c.input.id === id).expected;
  if (actual.fiveElement !== expected.fiveElement.fiveElementName) throw new Error(`${state}: five element mismatch`);
  assertions++;
  for (let i = 0; i < 12; i++) {
    const shown = actual.palaces[i].visibleText;
    const p = expected.palaces[i];
    const checks = [p.stem + p.branch, p.name, p.decadeName, p.decadeRange.join('~'), ...p.stars.map(s => s.name)];
    const year = p.yearlyName || p.yearlyAge;
    if (year) checks.push(year);
    for (const item of checks) {
      if (!shown.includes(item)) throw new Error(`${state} ${p.branch}: missing ${item}, saw ${JSON.stringify(shown)}`);
      assertions++;
    }
  }
}
// 按固定样例的序列化结构比较，失败时保留断言说明。
const eq = (actual, expected, msg) => {if (JSON.stringify(actual)!==JSON.stringify(expected)) throw Error(msg); assertions++};
eq(ui.allowed, [true,false,true,false,true,false,true,false,true,false,true,false], '甲 branch restriction');
eq(ui.allowedFemale, [false,true,false,true,false,true,false,true,false,true,false,true], '乙 branch restriction');
eq(ui.male.selected, ['男','甲','子','辰','正','子','寅','寅',null], 'male selected state');
eq(ui.female.selected, ['女','乙','丑','酉',null,null,'午','午','辰'], 'female selected state');
eq(ui.reverse.selected, ['男','乙','丑','酉',null,null,'午','午',null], 'reverse selected state');
eq(ui.flying.palaces[0].fill, '#F0F0F4', 'flying highlight');
eq(ui.flyingClear.palaces[0].fill, ui.male.palaces[0].fill, 'flying toggle');
eq(ui.manual.selected.slice(4), [null,null,'午','午','辰'], 'manual main and yearly retention');
eq(ui.femaleAfterStem.selected[2], '丑', 'stem change branch reset');
eq(ui.yearlyReset.selected[8], null, 'yearly reset');
eq(ui.exportResult, {filename:'甲子年紫微在辰-男盤.png', failure:null}, 'export');
eq(ui.color, {defaultColor:'rgb(234, 205, 118)', changedColor:'rgb(18, 52, 86)', resetColor:'rgb(234, 205, 118)'}, 'custom color and reset');
// 汇总指定观察状态的可见宫位文字，用于显示开关断言。
const texts = state => ui[state].palaces.flatMap(p => p.visibleText);
eq(texts('female').filter(x => ['天府','天相','七殺'].includes(x)).length, 3, 'location stars initially shown');
eq(texts('toggles').filter(x => ['天府','天相','七殺'].includes(x)).length, 0, 'location stars hidden');
eq(texts('female').filter(x => x === 'D').length, 2, 'D markers initially visible');
eq(texts('toggles').filter(x => x === 'D').length, 1, 'birth D marker hidden');
console.log(`${assertions} assertions passed across three complete UI cases and interaction states.`);
