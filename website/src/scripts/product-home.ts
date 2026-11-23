import { chartExample, type Branch } from '../data/chart-example.ts';

const experience = document.querySelector<HTMLElement>('[data-chart-experience]');

/** 五种视图复用同一个参考盘和飞化状态，深链接可直接打开指定视图。 */
function mountChartExperience(root: HTMLElement): void {
    const modes = [...root.querySelectorAll<HTMLAnchorElement>('[data-experience-mode]')];
    const navigation = root.querySelector<HTMLElement>('[data-experience-navigation]');
    const panes = [...root.querySelectorAll<HTMLElement>('[data-experience-pane]')];
    const cells = [...root.querySelectorAll<HTMLButtonElement>('[data-palace]')];
    const ring = root.querySelector('[data-experience-ring]');
    const marks = [...root.querySelectorAll<HTMLElement>('[data-experience-observation]')];

    /** 保留过去公开的章节锚点，并在关掉飞宫时停止演示计时。 */
    function selectMode(hash: string): void {
        const requested = hash.replace(/^#/, '');
        const mode = requested === 'annotation' ? 'chart-annotation'
            : requested === 'save-and-export' ? 'chart-outputs'
            : requested;
        const selected = !mode || mode === 'workspace' ? modes[0] : modes.find(link => link.dataset.experienceMode === mode);
        const view = selected?.dataset.experienceMode;
        if (!view) return;
        root.dataset.view = view;
        modes.forEach(link => {
            if (link === selected) link.setAttribute('aria-current', 'true');
            else link.removeAttribute('aria-current');
        });
        panes.forEach(pane => { pane.hidden = pane.dataset.experiencePane !== view; });
        cells.forEach(cell => { cell.disabled = view !== 'exhibit-flight'; });
        root.dispatchEvent(new Event('zhiji:workspace-step'));
    }

    modes.forEach(link => link.addEventListener('click', event => {
        if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
        event.preventDefault();
        // 视图切换保持原滚动位置，不交给 Lenis 的页面锚点处理。
        event.stopPropagation();
        selectMode(link.hash);
        window.history.replaceState(null, '', link.hash);
    }));
    window.addEventListener('hashchange', () => selectMode(window.location.hash));
    selectMode(window.location.hash);
    if (navigation) navigation.hidden = false;

    root.addEventListener('zhiji:flight-change', event => {
        if (!(event instanceof CustomEvent)) return;
        const destination = (event.detail as { destination: Branch }).destination;
        const index = chartExample.cells.findIndex(cell => cell?.branch === destination);
        if (index >= 0 && ring) {
            ring.setAttribute('cx', String((index % 4) * 100 + 50));
            ring.setAttribute('cy', String(Math.floor(index / 4) * 100 + 55));
        }
        const result = root.querySelector('[data-flight-result]')?.textContent;
        if (result) marks.forEach(mark => { mark.textContent = result; });
    });

    root.querySelectorAll<HTMLButtonElement>('[data-experience-toggle]').forEach(button => {
        button.addEventListener('click', () => {
            const attribute = `data-${button.dataset.experienceToggle}`;
            button.setAttribute('aria-pressed', String(root.toggleAttribute(attribute)));
        });
    });
    root.querySelectorAll<HTMLButtonElement>('[data-experience-direction]').forEach(button => {
        const mark = root.querySelector(`[data-self-mark="${button.dataset.experienceDirection}"]`);
        if (!mark) return;
        button.addEventListener('click', () => {
            const visible = button.getAttribute('aria-pressed') === 'false';
            button.setAttribute('aria-pressed', String(visible));
            mark.toggleAttribute('data-muted', !visible);
        });
    });

    const formats = [...root.querySelectorAll<HTMLButtonElement>('[data-experience-format]')];
    const descriptions = root.querySelectorAll<HTMLElement>('[data-experience-format-description]');
    const guide = root.querySelector<HTMLAnchorElement>('[data-experience-file-guide]');
    formats.forEach(button => button.addEventListener('click', () => {
        const format = button.dataset.experienceFormat ?? 'draft';
        root.dataset.format = format;
        formats.forEach(item => item.setAttribute('aria-pressed', String(item === button)));
        descriptions.forEach(description => { description.hidden = description.dataset.experienceFormatDescription !== format; });
        if (guide) {
            guide.href = format === 'png' ? '/guide/#export' : '/guide/#save';
            if (guide.firstChild) guide.firstChild.textContent = format === 'png' ? '图片导出 ' : '草稿操作 ';
        }
    }));
}

if (experience) mountChartExperience(experience);
