import { chartExample } from "./chart-example.ts";
import type { Branch, Transformation } from "./chart-example.ts";

// 与 chart-example.ts 使用同一固定参考命盘，此处不执行实时排盘。
export const palaceStems: Record<Branch, string> = {
    寅: "丙", 卯: "丁", 辰: "戊", 巳: "己", 午: "庚", 未: "辛",
    申: "壬", 酉: "癸", 戌: "甲", 亥: "乙", 子: "丙", 丑: "丁",
};

export const selfTransformationExample: { branch: Branch; palace: string; star: string; inward: string; outward: string } = {
    branch: "卯", palace: "父母", star: "巨门", inward: "权", outward: "忌",
};

export const readingDecadeRanges: Partial<Record<Branch, readonly [number, number]>> = {
    辰: [26, 35], 卯: [16, 25], 寅: [6, 15],
};

const flyingStarsByPalace: Record<Branch, readonly string[]> = {
    寅: ["天同", "天机", "文昌", "廉贞"],
    卯: ["太阴", "天同", "天机", "巨门"],
    辰: ["贪狼", "太阴", "右弼", "天机"],
    巳: ["武曲", "贪狼", "天梁", "文曲"],
    午: ["太阳", "武曲", "太阴", "天同"],
    未: ["巨门", "太阳", "文曲", "文昌"],
    申: ["天梁", "紫微", "左辅", "武曲"],
    酉: ["破军", "巨门", "太阴", "贪狼"],
    戌: ["廉贞", "破军", "武曲", "太阳"],
    亥: ["天机", "天梁", "紫微", "太阴"],
    子: ["天同", "天机", "文昌", "廉贞"],
    丑: ["太阴", "天同", "天机", "巨门"],
};

/** 根据固定宫干表校验地支，防止任意页面字符串索引领域数据。 */
export function isBranch(value: string): value is Branch {
    return Object.hasOwn(palaceStems, value);
}

/** 从固定参考星表取得该宫四化落点，未知地支不生成结果。 */
export function transformationsFor(branch: string): Transformation[] {
    if (!isBranch(branch)) return [];
    const stars = flyingStarsByPalace[branch];
    return stars.flatMap((star, index) => {
        const target = chartExample.cells.find((cell) => cell?.stars.includes(star));
        if (!target) return [];
        return [{
            kind: chartExample.transformations[index].kind,
            star,
            branch: target.branch,
            palace: target.palace,
        }];
    });
}
