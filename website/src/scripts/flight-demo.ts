import { chartExample } from "../data/chart-example.ts";
import type { Branch } from "../data/chart-example.ts";
import { isBranch, palaceStems, transformationsFor } from "../data/flight-example.ts";

const demo = document.querySelector<HTMLElement>("[data-flight-demo]");

/** 将固定参考命盘绑定为可操作的飞化演示，不修改排盘数据。 */
function mountDemo(demo: HTMLElement): void {
    const buttons = [...demo.querySelectorAll<HTMLButtonElement>("[data-palace]")];
    const choices = [...demo.querySelectorAll<HTMLInputElement>('.flight-choice input')];
    const path = demo.querySelector<SVGPathElement>("[data-flight-path]");
    const originLabel = demo.querySelector<HTMLElement>("[data-flight-origin]");
    const stemLabel = demo.querySelector<HTMLElement>("[data-flight-stem]");
    const result = demo.querySelector<HTMLElement>("[data-flight-result]");
    if (!path || !originLabel || !stemLabel || !result) return;
    let origin: Branch = chartExample.origin;

    /** 返回宫位内缘的连接锚点，未知宫位返回空值。 */
    function pointFor(branch: Branch): { x: number; y: number } | null {
        const index = chartExample.cells.findIndex((cell) => cell?.branch === branch);
        if (index < 0) return null;
        const col = index % 4;
        const row = Math.floor(index / 4);
        const x = col * 100 + 50;
        const y = row * 100 + 50;
        return {
            x: col === 0 ? 92 : col === 3 ? 308 : x,
            y: row === 0 ? 92 : row === 3 ? 308 : y,
        };
    }

    /** 生成内收曲线，同宫飞化使用可见回环。 */
    function lineFor(from: Branch, to: Branch): string {
        const start = pointFor(from);
        const end = pointFor(to);
        if (!start || !end) return "";
        if (from === to) {
            const inwardX = Math.sign(200 - start.x);
            const inwardY = Math.sign(200 - start.y);
            return `M${start.x} ${start.y - 8} q${inwardX * 38} ${inwardY * 38 - 20} 0 16`;
        }
        const dx = end.x - start.x;
        const dy = end.y - start.y;
        const mx = (start.x + end.x) / 2;
        const my = (start.y + end.y) / 2;
        const pull = Math.min(0.38, 40 / Math.hypot(dx, dy));
        const controlX = mx + (200 - mx) * pull;
        const controlY = my + (200 - my) * pull;
        return `M${start.x} ${start.y} Q${controlX} ${controlY} ${end.x} ${end.y}`;
    }

    // 同步宫干、落点、可访问状态和路径；键盘或减少动效时直接显示结果。
    const update = (animate = false): void => {
        const selected = choices.find((choice) => choice.checked)?.value ?? "禄";
        const source = chartExample.cells.find((cell) => cell?.branch === origin);
        const transformations = transformationsFor(origin);
        const active = transformations.find((item) => item.kind === selected);
        if (!source || !active) return;

        originLabel.textContent = `${origin} · ${source.palace}`;
        stemLabel.textContent = `宫干 ${palaceStems[origin]}`;
        choices.forEach((choice, index) => {
            const item = transformations[index];
            const label = choice.closest("label");
            const star = label?.querySelector<HTMLElement>("[data-flight-star]");
            const destination = label?.querySelector<HTMLElement>("[data-flight-destination]");
            if (item && star && destination) {
                star.textContent = item.star;
                destination.textContent = `${item.palace} · ${item.branch}宫`;
            }
        });
        buttons.forEach((button) => {
            button.setAttribute("aria-pressed", String(button.dataset.palace === origin));
            button.classList.toggle("is-target", button.dataset.palace === active.branch);
        });
        path.setAttribute("d", lineFor(origin, active.branch));
        path.getAnimations().forEach((animation) => animation.cancel());
        if (animate && !window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
            path.animate(
                [{ strokeDasharray: "1 1", strokeDashoffset: "1" }, { strokeDasharray: "1 1", strokeDashoffset: "0" }],
                { duration: 480, easing: "cubic-bezier(0.22, 1, 0.36, 1)" },
            );
        }
        result.textContent = `${origin}宫化${active.kind}，落入${active.branch}宫${active.star}。`;
    };

    buttons.forEach((button) => {
        button.disabled = false;
        button.addEventListener("click", (event) => {
            const branch = button.dataset.palace;
            if (!branch || !isBranch(branch)) return;
            origin = branch;
            update(event.detail > 0);
        });
    });
    choices.forEach((choice) => {
        choice.disabled = false;
        let pointerActivated = false;
        choice.closest("label")?.addEventListener("pointerdown", () => { pointerActivated = true; });
        choice.addEventListener("keydown", () => { pointerActivated = false; });
        choice.addEventListener("change", () => {
            update(pointerActivated);
            pointerActivated = false;
        });
    });
    update();
}

if (demo) mountDemo(demo);
