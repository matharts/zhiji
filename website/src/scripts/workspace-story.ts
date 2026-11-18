import { chartExample } from '../data/chart-example.ts';
import type { Branch } from '../data/chart-example.ts';

const workspace = document.querySelector<HTMLElement>('[data-workspace]');
if (workspace) {
    const steps = [...workspace.querySelectorAll<HTMLButtonElement>('[data-workspace-step]')];
    const panels = [...workspace.querySelectorAll<HTMLElement>('[data-workspace-panel]')];
    const guide = workspace.querySelector<HTMLAnchorElement>('[data-workspace-guide]');
    const demo = workspace.querySelector<HTMLElement>('[data-flight-demo]');
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)');
    const hashSteps: Record<string, string> = { '#exhibit-flight':'flight', '#annotation':'annotation', '#save-and-export':'save', '#workspace':'input' };

    /** 仅切换同一参考盘的工作步骤，不运行取数、不生成文件，也不重建命盘。 */
    function selectStep(id: string, animate = true): void {
        const selected = steps.find(button => button.dataset.workspaceStep === id);
        if (!selected) return;
        workspace!.dataset.step = id;
        steps.forEach(button => button.setAttribute('aria-pressed', String(button === selected)));
        panels.forEach(panel => {
            panel.hidden = panel.dataset.workspacePanel !== id;
            panel.getAnimations().forEach(animation => animation.cancel());
            if (!panel.hidden && animate && !reduced.matches) panel.animate([
                { opacity: .55, transform: 'translateY(8px)' },
                { opacity: 1, transform: 'translateY(0)' },
            ], { duration: 320, easing: 'cubic-bezier(.16,1,.3,1)' });
        });
        // 只有飞化步骤开放宫位选择，其他步骤保留已选结果。
        demo?.querySelectorAll<HTMLButtonElement>('[data-palace]').forEach(button => { button.disabled = id !== 'flight'; });
        if (guide) guide.href = `/guide/#${selected.dataset.guide}`;
        workspace!.dispatchEvent(new CustomEvent('zhiji:workspace-step'));
    }

    steps.forEach(button => {
        button.disabled = false;
        button.addEventListener('click', () => {
            const id = button.dataset.workspaceStep;
            if (id && workspace.dataset.step !== id) selectStep(id);
        });
    });

    /** 标注跟随当前四化的实际落宫，后续步骤保留同一观察结果。 */
    workspace.addEventListener('zhiji:flight-change', event => {
        if (!(event instanceof CustomEvent)) return;
        const destination = (event.detail as { destination: Branch }).destination;
        const index = chartExample.cells.findIndex(cell => cell?.branch === destination);
        const circle = workspace.querySelector('[data-workspace-mark="circle"]');
        if (index >= 0 && circle) {
            const x = (index % 4) * 100 + 50;
            const y = Math.floor(index / 4) * 100 + 50;
            circle.setAttribute('cx', String(x));
            circle.setAttribute('cy', String(y));
            workspace.querySelector('[data-workspace-mark="line"]')?.setAttribute('d', `M${x + (x < 200 ? 40 : -40)} ${y - 15} Q200 220 265 245m-12-6 12 6-12 8`);
        }
        const observation = workspace.querySelector('[data-workspace-observation]');
        const result = demo?.querySelector('[data-flight-result]');
        if (observation && result) observation.textContent = result.textContent;
    });

    workspace.querySelectorAll<HTMLButtonElement>('[data-workspace-toggle]').forEach(button => {
        button.disabled = false;
        button.addEventListener('click', () => {
            const shown = button.getAttribute('aria-pressed') !== 'true';
            button.setAttribute('aria-pressed', String(shown));
            const mark = workspace.querySelector<SVGElement | HTMLElement>(`[data-workspace-mark="${button.dataset.workspaceToggle}"]`);
            if (mark) mark.style.visibility = shown ? 'visible' : 'hidden';
        });
    });
    workspace.querySelectorAll<HTMLButtonElement>('[data-workspace-scale]').forEach(button => {
        button.disabled = false;
        button.addEventListener('click', () => {
            workspace.querySelectorAll('[data-workspace-scale]').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
            const status = workspace.querySelector('[data-workspace-output-status]');
            if (status) status.textContent = `${button.dataset.workspaceScale}× 图片示意 · 复制图片固定使用 2×`;
        });
    });

    /** 保留既有深链接；到达标注、保存或飞化入口时呈现对应步骤。 */
    function followHash(): void {
        const id = hashSteps[window.location.hash];
        if (id) selectStep(id, false);
    }
    window.addEventListener('hashchange', followHash);
    // 重复点击当前锚点也能回到对应演示，不依赖 hashchange 再次发生。
    document.querySelectorAll<HTMLAnchorElement>('a[href]').forEach(link => {
        if (link.origin !== window.location.origin || link.pathname !== window.location.pathname || !hashSteps[link.hash]) return;
        link.addEventListener('click', () => {
            const id = hashSteps[link.hash];
            if (id) selectStep(id, false);
        });
    });
    selectStep('input', false);
    followHash();
    workspace.dataset.ready = 'true';
}
