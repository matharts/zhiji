import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { chartExample } from "../src/data/chart-example.ts";
import type { Branch, ChartCell, TransformationKind } from "../src/data/chart-example.ts";
import { palaceStems, readingDecadeRanges, readingYearlyAges, selfTransformationExample, transformationsFor } from "../src/data/flight-example.ts";

type TransformationCode = "A" | "B" | "C" | "D";
interface ReferenceStar { name: string; inwardTransformation: TransformationCode | null; outwardTransformation: TransformationCode | null; }
interface ReferencePalace { branch: Branch; stem: string; name: string; decadeRange: [number, number]; yearlyAge: string | null; stars: ReferenceStar[]; }
interface ReferenceCase {
    input: { id: string; stem: string; branch: string; gender: string; month: number; hour: Branch; ziwei: Branch };
    expected: { mainBranch: Branch; palaces: ReferencePalace[]; flyingStarsByPalace: Record<Branch, string[]> };
}
interface Fixture { cases: ReferenceCase[]; reference: { transformationCodes: Record<TransformationCode, TransformationKind> }; }

const fixture: Fixture = JSON.parse(
    await readFile(
        new URL("../../tests/fixtures/simulator-oracle.json", import.meta.url),
        "utf8",
    ),
);
const example = fixture.cases.find(
    (item) => item.input.id === "jiazi-male-month-hour",
);
assert.ok(example);
const names: Record<string, string> = {
    命宮: "命宫",
    官祿: "官禄",
    遷移: "迁移",
    財帛: "财帛",
    貪狼: "贪狼",
    天機: "天机",
    巨門: "巨门",
    七殺: "七杀",
    廉貞: "廉贞",
    破軍: "破军",
    太陽: "太阳",
    太陰: "太阴",
};
// 仅转换已知繁体名称，其余名称保持原样以便暴露参考数据差异。
const simplified = (name: string): string => names[name] ?? name;
// 收窄非空宫格类型，后续比较只处理实际占用的格子。
const hasCell = (cell: ChartCell | null): cell is ChartCell => cell !== null;

// 将网站宫位和飞宫展示与独立参考样例逐项比较，防止演示数据漂移。
test("website chart and flying transformations match the project reference case", () => {
    assert.ok(example);
    assert.equal(
        chartExample.year,
        `${example.input.stem}${example.input.branch}年`,
    );
    assert.equal(chartExample.origin, example.expected.mainBranch);
    // 工作流展示的参数也来自独立参考样例，避免为视觉演示编造出生资料。
    for (const key of ['gender', 'month', 'hour', 'ziwei'] as const) {
        assert.equal(chartExample.parameters[key], example.input[key]);
    }

    const palaces = new Map(
        example.expected.palaces.map((palace) => [palace.branch, palace]),
    );
    const order = [
        "巳",
        "午",
        "未",
        "申",
        "辰",
        null,
        null,
        "酉",
        "卯",
        null,
        null,
        "戌",
        "寅",
        "丑",
        "子",
        "亥",
    ];
    assert.deepEqual(
        chartExample.cells.map((cell) => cell?.branch ?? null),
        order,
    );
    for (const cell of chartExample.cells.filter(hasCell)) {
        const reference = palaces.get(cell.branch);
        assert.ok(reference);
        assert.equal(cell.palace, simplified(reference.name));
        assert.deepEqual(
            cell.stars,
            reference.stars.map((star) => simplified(star.name)),
        );
    }

    const origin = palaces.get(chartExample.origin);
    assert.ok(origin);
    assert.equal(chartExample.originStem, origin.stem);
    const flyingStars =
        example.expected.flyingStarsByPalace[chartExample.origin];
    const kinds = (["A", "B", "C", "D"] as const).map(
        (code) => fixture.reference.transformationCodes[code],
    );
    assert.equal(chartExample.transformations.length, kinds.length);
    chartExample.transformations.forEach((item, index) => {
        const target = example.expected.palaces.find((palace) =>
            palace.stars.some((star) => star.name === flyingStars[index]),
        );
        assert.ok(target);
        assert.equal(item.kind, kinds[index]);
        assert.equal(item.star, simplified(flyingStars[index]));
        assert.equal(item.branch, target.branch);
        assert.equal(item.palace, simplified(target.name));
    });
});

// 遍历所有可选宫位，验证四化顺序、星曜及落宫，未知宫位不产生结果。
test("every selectable palace uses the reference chart's four transformations", () => {
    const palaces = new Map(example.expected.palaces.map((palace) => [palace.branch, palace]));
    for (const cell of chartExample.cells.filter(hasCell)) {
        const reference = palaces.get(cell.branch);
        assert.ok(reference);
        assert.equal(palaceStems[cell.branch], reference.stem);
        const transformations = transformationsFor(cell.branch);
        assert.equal(transformations.length, 4);
        transformations.forEach((item, index) => {
            const star = simplified(example.expected.flyingStarsByPalace[cell.branch][index]);
            const destination = chartExample.cells.find((candidate) => candidate?.stars.includes(star));
            assert.ok(destination);
            assert.equal(item.kind, ["禄", "权", "科", "忌"][index]);
            assert.equal(item.star, star);
            assert.equal(item.branch, destination.branch);
            assert.equal(item.palace, destination.palace);
        });
    }
    assert.deepEqual(transformationsFor("不存在"), []);
});

// 自化演示的宫位、星曜及向心离心代码必须对应同一参考星曜。
test("the self-transformation example matches the reference star", () => {
    const sample = selfTransformationExample;
    const palace = example.expected.palaces.find((item) => item.branch === sample.branch);
    assert.ok(palace);
    const star = palace.stars.find((item) => simplified(item.name) === sample.star);
    assert.ok(star);
    assert.ok(star.inwardTransformation);
    assert.ok(star.outwardTransformation);
    assert.equal(sample.palace, simplified(palace.name));
    assert.equal(sample.inward, fixture.reference.transformationCodes[star.inwardTransformation]);
    assert.equal(sample.outward, fixture.reference.transformationCodes[star.outwardTransformation]);
});

// 盘面对照里的大限区间与流年年龄必须和同一参考盘一致。
test("the displayed decade ranges and yearly ages match the reference chart", () => {
    for (const palace of example.expected.palaces) {
        assert.deepEqual(readingDecadeRanges[palace.branch], palace.decadeRange);
        const age = readingYearlyAges[palace.branch];
        assert.equal(age === undefined ? null : `${age} 歲`, palace.yearlyAge);
    }
});
