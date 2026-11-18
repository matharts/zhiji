import { drawArrow, finishArrowDrawing } from './arrow-motion.ts';

/** 新增产品流程只切换展示内容；不运行历法、不保存文件、不发起图片导出。 */
const reducedStories = window.matchMedia('(prefers-reduced-motion: reduce)');

/** 用同一短过渡承接用户选择，连续切换时取消旧动画。 */
function revealStory(panel: HTMLElement): void {
    panel.getAnimations().forEach(animation => animation.cancel());
    if (!reducedStories.matches) panel.animate([
        { opacity: .65, transform: 'translateY(6px)' },
        { opacity: 1, transform: 'translateY(0)' },
    ], { duration: 260, easing: 'cubic-bezier(.16,1,.3,1)' });
}

const birthStory = document.querySelector('[data-birth-story]');
if (birthStory) {
    const buttons = [...birthStory.querySelectorAll<HTMLButtonElement>('[data-birth-mode]')];
    const panels = [...birthStory.querySelectorAll<HTMLElement>('[data-birth-panel]')];
    buttons.forEach(button => {
        button.disabled = false;
        button.addEventListener('click', () => {
            if (button.getAttribute('aria-pressed') === 'true') return;
            buttons.forEach(item => item.setAttribute('aria-pressed', String(item === button)));
            panels.forEach(panel => {
                panel.hidden = panel.dataset.birthPanel !== button.dataset.birthMode;
                if (!panel.hidden) revealStory(panel);
            });
        });
    });
}

const selfStory = document.querySelector('[data-self-story]');
if (selfStory) {
    const buttons = [...selfStory.querySelectorAll<HTMLButtonElement>('[data-self-direction]')];
    const status = selfStory.querySelector('[data-self-status]');
    const selected = { inward: true, outward: true };
    /** 向心、离心可以分别开启、同时开启或全部隐藏，与桌面筛选规则一致。 */
    function syncDirections(animate: boolean): void {
        const all = selected.inward && selected.outward;
        buttons.forEach(button => {
            const key = button.dataset.selfDirection;
            button.setAttribute('aria-pressed', String(key === 'all' ? all : key === 'inward' ? selected.inward : selected.outward));
        });
        selfStory!.querySelectorAll<SVGGElement>('[data-self-mark]').forEach(mark => {
            const show = mark.dataset.selfMark === 'inward' ? selected.inward : selected.outward;
            const wasHidden = mark.style.visibility === 'hidden';
            mark.style.visibility = show ? 'visible' : 'hidden';
            mark.querySelectorAll<SVGPathElement>('[data-self-path]').forEach(path => {
                if (!show) finishArrowDrawing(path);
                else if (wasHidden && animate) drawArrow(path, 420);
            });
        });
        if (status) status.textContent = all ? '向心与离心同时显示' : selected.inward ? '仅显示向心自化' : selected.outward ? '仅显示离心自化' : '自化标记已隐藏';
    }
    buttons.forEach(button => {
        button.disabled = false;
        button.addEventListener('click', event => {
            const key = button.dataset.selfDirection;
            if (key === 'all') selected.inward = selected.outward = !(selected.inward && selected.outward);
            else if (key === 'inward' || key === 'outward') selected[key] = !selected[key];
            syncDirections(event.detail > 0);
        });
    });
}

const outputStory = document.querySelector<HTMLElement>('[data-output-preview]');
outputStory?.querySelectorAll<HTMLButtonElement>('[data-output-size]').forEach(button => {
    button.disabled = false;
    button.addEventListener('click', () => {
        const size = button.dataset.outputSize ?? '2';
        outputStory.dataset.outputScale = size;
        outputStory.querySelectorAll('[data-output-size]').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
        const factor = outputStory.querySelector('[data-output-factor]');
        if (factor) factor.textContent = `${size}×`;
    });
});
