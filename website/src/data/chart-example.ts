export type Branch = "子" | "丑" | "寅" | "卯" | "辰" | "巳" | "午" | "未" | "申" | "酉" | "戌" | "亥";
export type TransformationKind = "禄" | "权" | "科" | "忌";
export interface ChartCell { branch: Branch; palace: string; stars: string[]; }
export interface Transformation { kind: TransformationKind; star: string; branch: Branch; palace: string; }

interface ChartExample {
    year: string;
    origin: Branch;
    originStem: string;
    cells: (ChartCell | null)[];
    transformations: Transformation[];
}

// 展示数据取自 tests/fixtures/simulator-oracle.json 的 jiazi-male-month-hour 样例。
// 此处保持固定参考值，不执行实时排盘计算。
export const chartExample: ChartExample = {
    year: "甲子年",
    origin: "寅",
    originStem: "丙",
    cells: [
        { branch: "巳", palace: "田宅", stars: ["天梁"] },
        { branch: "午", palace: "官禄", stars: ["七杀"] },
        { branch: "未", palace: "交友", stars: [] },
        { branch: "申", palace: "迁移", stars: ["廉贞"] },
        {
            branch: "辰",
            palace: "福德",
            stars: ["紫微", "天相", "左辅", "文曲"],
        },
        null,
        null,
        { branch: "酉", palace: "疾厄", stars: [] },
        { branch: "卯", palace: "父母", stars: ["天机", "巨门"] },
        null,
        null,
        { branch: "戌", palace: "财帛", stars: ["破军", "右弼", "文昌"] },
        { branch: "寅", palace: "命宫", stars: ["贪狼"] },
        { branch: "丑", palace: "兄弟", stars: ["太阳", "太阴"] },
        { branch: "子", palace: "夫妻", stars: ["武曲", "天府"] },
        { branch: "亥", palace: "子女", stars: ["天同"] },
    ],
    transformations: [
        { kind: "禄", star: "天同", branch: "亥", palace: "子女" },
        { kind: "权", star: "天机", branch: "卯", palace: "父母" },
        { kind: "科", star: "文昌", branch: "戌", palace: "财帛" },
        { kind: "忌", star: "廉贞", branch: "申", palace: "迁移" },
    ],
};
