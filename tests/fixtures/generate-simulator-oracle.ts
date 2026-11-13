// 使用 Bun 和固定版本的干净参考仓库生成样例：
// 执行 bun tests/fixtures/generate-simulator-oracle.ts /path/to/ziwei-simulator > /tmp/simulator-oracle.json。
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const expectedRevision = "5a5ace5898f43911ce91de031d79f9c4ccffaaef";
const checkout = process.argv[2];
if (!checkout) throw new Error("Pass the path to the reference simulator checkout.");

// 仅查询参考仓库的版本与修改状态，不改变其工作树。
const git = (...args: string[]) =>
  execFileSync("git", ["-C", checkout, ...args], { encoding: "utf8" }).trim();
if (git("rev-parse", "HEAD") !== expectedRevision) {
  throw new Error(`Reference checkout must be at ${expectedRevision}.`);
}
if (git("status", "--porcelain", "--untracked-files=no")) {
  throw new Error("Reference checkout has modified tracked files.");
}

// 从已核验的参考仓库按文件 URL 加载原实现，兼容路径中的特殊字符。
const load = (file: string) => import(pathToFileURL(resolve(checkout, file)).href);
const rules = await load("src/rules/index.ts");
const constants = await load("src/constants/index.ts");
const { wrapIndex, oppositeIndex } = await load("src/utils/array.ts");
const { BRANCH, PLACE_BRANCH, STEM, STEM_TRANSFORMATIONS } = constants;

type Scenario = {
  id: string;
  gender: "男" | "女";
  stem: string;
  branch: string;
  ziwei: string;
  month: number | null; // 月份取 1 至 12；null 表示未提供。
  hour: string | null;
  manualMain: string | null;
  decade: string;
  yearly: string | null;
  showLocationStar: boolean;
};

const examples: Scenario[] = [
  { id: "jiazi-male-month-hour", gender: "男", stem: "甲", branch: "子", ziwei: "辰", month: 1, hour: "子", manualMain: null, decade: "寅", yearly: null, showLocationStar: true },
  { id: "yichou-female-manual-yearly", gender: "女", stem: "乙", branch: "丑", ziwei: "酉", month: null, hour: null, manualMain: "午", decade: "午", yearly: "辰", showLocationStar: true },
  { id: "yichou-male-reverse", gender: "男", stem: "乙", branch: "丑", ziwei: "酉", month: null, hour: null, manualMain: "午", decade: "午", yearly: null, showLocationStar: false },
  { id: "guihai-female-twelfth-month", gender: "女", stem: "癸", branch: "亥", ziwei: "子", month: 12, hour: "亥", manualMain: null, decade: "未", yearly: "亥", showLocationStar: false },
  { id: "renxu-male-wraparound", gender: "男", stem: "壬", branch: "戌", ziwei: "亥", month: 2, hour: "戌", manualMain: null, decade: "午", yearly: "子", showLocationStar: true },
];

// 仅显式请求时生成更大的确定性交叉样例。
// 常规 Rust 测试不依赖 Bun，也不要求检入大规模样例。
const matrix = process.argv.includes("--matrix");
const scenarios: Scenario[] = matrix
  ? STEM.flatMap((stem: string, stemIndex: number) =>
      (["男", "女"] as const).flatMap((gender) =>
        BRANCH.flatMap((ziwei: string, ziweiIndex: number) =>
          Array.from({ length: 12 }, (_, monthIndex) =>
            ([false, true] as const).map((manual) => {
              const hourIndex = (ziweiIndex * 5 + monthIndex + stemIndex) % 12;
              return {
                id: `${stem}-${gender}-${ziwei}-${monthIndex + 1}-${manual ? "manual" : "auto"}`,
                gender,
                stem,
                branch: BRANCH[(2 * monthIndex + stemIndex % 2) % 12],
                ziwei,
                month: manual ? null : monthIndex + 1,
                hour: manual ? null : BRANCH[hourIndex],
                manualMain: manual ? BRANCH[(monthIndex + stemIndex * 3 + ziweiIndex * 5) % 12] : null,
                decade: BRANCH[(monthIndex + ziweiIndex * 3 + stemIndex) % 12],
                yearly: (monthIndex + stemIndex) % 3 === 0 ? null : BRANCH[(hourIndex + ziweiIndex) % 12],
                showLocationStar: (monthIndex + ziweiIndex + Number(manual)) % 2 === 0,
              } satisfies Scenario;
            }),
          ).flat(),
        ),
      ),
    )
  : examples;

// 将参考名称转换为索引，未知名称立即失败，避免负索引污染结果。
function indexOf(values: readonly string[], value: string): number {
  const result = values.indexOf(value);
  if (result < 0) throw new Error(`Unknown value: ${value}`);
  return result;
}

// 调用固定参考实现计算期望值，统一手动命宫与月份时辰两种输入。
function calculate(input: Scenario) {
  const stem = indexOf(STEM, input.stem);
  const branch = indexOf(BRANCH, input.branch);
  const ziwei = indexOf(BRANCH, input.ziwei);
  const month = input.month == null ? undefined : input.month - 1;
  const hour = input.hour == null ? undefined : indexOf(BRANCH, input.hour);
  const main = input.manualMain == null
    ? rules.calculateMainPalaceIndex(month, hour)
    : wrapIndex(indexOf(BRANCH, input.manualMain) - 2);
  const decade = wrapIndex(indexOf(BRANCH, input.decade) - 2);
  const yearly = input.yearly == null ? null : wrapIndex(indexOf(BRANCH, input.yearly) - 2);
  const stems = rules.calculateStems(stem);
  const stars = rules.calculateStars(ziwei, month, hour, input.showLocationStar);
  const palaces = rules.calculatePalaces(main);
  const decadePalaces = rules.calculateDecadePalaces(decade);
  const yearlyPalaces = yearly == null ? [] : rules.calculateYearlyPalaces(yearly);
  const fiveElement = rules.calculateFiveElementNum(stem, main);
  const direction = rules.getDecadeDirection(stem, input.gender === "男" ? 1 : 0);
  const ranges = rules.calculateDecadeRanges(main, direction, fiveElement.fiveElementNum);
  const yearlyAges = rules.calculateYearlys(ranges[decade][0], wrapIndex(branch - 2));

  return {
    input,
    expected: {
      mainBranch: BRANCH[wrapIndex(main + 2)],
      fiveElement,
      decadeDirection: direction === 1 ? "顺" : "逆",
      flyingStarsByPalace: Object.fromEntries(
        PLACE_BRANCH.map((name: string, i: number) => [name, STEM_TRANSFORMATIONS[stems[i]]]),
      ),
      palaces: PLACE_BRANCH.map((name: string, i: number) => ({
        branch: name,
        stem: stems[i],
        name: palaces[i],
        decadeName: decadePalaces[i],
        decadeRange: ranges[i],
        yearlyAge: yearlyAges[i] || null,
        yearlyName: yearlyPalaces[i] ?? null,
        isLaiYin: rules.isLaiYin(name, input.stem),
        stars: stars[i].map((star: string) => ({
          name: star,
          birthTransformation: rules.calculateStarTransformation(input.stem, star) ?? null,
          outwardTransformation: rules.calculateStarTransformation(stems[i], star) ?? null,
          inwardTransformation: rules.calculateStarTransformation(stems[oppositeIndex(i)], star) ?? null,
        })),
      })),
    },
  };
}

console.log(JSON.stringify({
  reference: {
    repository: "https://github.com/lzm0x219/ziwei-simulator",
    revision: expectedRevision,
    basis: "Original implementation output; behavioral reference, not independent validation of astrological correctness.",
    palaceOrder: "寅 through 丑",
    transformationCodes: { A: "禄", B: "权", C: "科", D: "忌" },
  },
  cases: scenarios.map(calculate),
}, null, matrix ? undefined : 2));
