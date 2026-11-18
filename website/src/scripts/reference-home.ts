const referenceReducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

/** 用户切换时用短暂显现连接内容；同一选择不重复播放，减少动效时直接呈现。 */
function revealPreview(element: HTMLElement, delay = 0): void {
    element.getAnimations().forEach(animation => animation.cancel());
    if (referenceReducedMotion.matches) return;
    element.animate([
        { opacity: .6, transform: 'translateY(8px) scale(.99)' },
        { opacity: 1, transform: 'translateY(0) scale(1)' },
    ], { duration: 380, delay, easing: 'cubic-bezier(.16,1,.3,1)' });
}

/** 分类与横向翻页保持原生滚动；减少动态效果时立即定位。 */
const gallery = document.querySelector<HTMLElement>('[data-gallery]');
if (gallery) {
    const track = gallery.querySelector<HTMLElement>('[data-gallery-track]');
    const filters = gallery.querySelector<HTMLElement>('[data-gallery-filters]');
    const arrows = gallery.querySelector<HTMLElement>('[data-gallery-arrows]');
    const previous = gallery.querySelector<HTMLButtonElement>('[data-gallery-prev]');
    const next = gallery.querySelector<HTMLButtonElement>('[data-gallery-next]');
    const count = gallery.querySelector<HTMLElement>('[data-gallery-count]');
    const cards = [...gallery.querySelectorAll<HTMLElement>('[data-gallery-group]')];
    if (track && filters && arrows && previous && next) {
        filters.hidden = false;
        arrows.hidden = false;
        track.dataset.filteredCount = String(cards.length);
        const updateArrows = () => {
            previous.disabled = track.scrollLeft < 2;
            next.disabled = track.scrollLeft >= track.scrollWidth - track.clientWidth - 2;
            // 只显示当前可见范围，不用轮播计时器推动内容，也不播报每帧滚动。
            const shown = cards.filter(card => !card.hidden);
            const origin = shown[0]?.offsetLeft ?? 0;
            const visible = shown.map((card, index) => ({card,index})).filter(({card}) => {
                const left = card.offsetLeft - origin - track.scrollLeft;
                return left < track.clientWidth - 2 && left + card.offsetWidth > 2;
            });
            if (count && visible.length) count.textContent = `${visible[0]!.index + 1}–${visible.at(-1)!.index + 1} / ${shown.length}`;
        };
        filters.querySelectorAll<HTMLButtonElement>('[data-gallery-filter]').forEach(button => {
            button.addEventListener('click', () => {
                if (button.getAttribute('aria-pressed') === 'true') return;
                filters.querySelectorAll('button').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
                cards.forEach(card => { card.hidden = button.dataset.galleryFilter !== 'all' && card.dataset.galleryGroup !== button.dataset.galleryFilter; });
                track.dataset.filteredCount = String(cards.filter(card => !card.hidden).length);
                track.scrollLeft = 0;
                cards.filter(card => !card.hidden).forEach((card, index) => revealPreview(card, Math.min(index, 2) * 45));
                updateArrows();
            });
        });
        const step = (direction: number) => track.scrollBy({
            left: direction * track.clientWidth,
            behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth',
        });
        previous.addEventListener('click', () => step(-1));
        next.addEventListener('click', () => step(1));
        track.addEventListener('keydown', event => {
            // 内部按钮仍使用自己的键盘行为，只有轨道获得焦点时才接管方向键。
            if (event.target !== track || !['ArrowLeft','ArrowRight','Home','End'].includes(event.key)) return;
            event.preventDefault();
            if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') step(event.key === 'ArrowLeft' ? -1 : 1);
            else track.scrollTo({left:event.key === 'Home' ? 0 : track.scrollWidth,behavior:window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth'});
        });
        track.addEventListener('scroll', updateArrows, { passive: true });
        const observer = new ResizeObserver(updateArrows);
        observer.observe(track);
        updateArrows();
        window.addEventListener('pagehide', () => observer.disconnect(), { once: true });
    }
}

/** 键盘场景只展示快捷键，不触发页面外的文件动作。 */
const keywallButtons = [...document.querySelectorAll<HTMLButtonElement>('[data-keywall]')];
keywallButtons.forEach(button => {
    button.disabled = false;
    button.addEventListener('click', () => {
        keywallButtons.forEach(item => item.setAttribute('aria-pressed', String(item === button)));
        const status = document.querySelector('[data-keywall-status]');
        if (status) status.textContent = `${button.querySelector('strong')?.textContent?.replace('。', '')} · ${button.dataset.keywall}`;
    });
});

/** 文字选择器同步演示窗口和指南链接，切换由显式操作触发。 */
const abilities = document.querySelector('[data-abilities]');
if (abilities) {
    const buttons = [...abilities.querySelectorAll<HTMLButtonElement>('[data-ability]')];
    const panels = [...abilities.querySelectorAll<HTMLElement>('[data-ability-panel]')];
    const more = abilities.querySelector<HTMLAnchorElement>('[data-ability-link]');
    buttons.forEach(button => {
        button.disabled = false;
        button.addEventListener('click', () => {
            if (button.getAttribute('aria-pressed') === 'true') return;
            buttons.forEach(item => item.setAttribute('aria-pressed', String(item === button)));
            panels.forEach(panel => {
                const wasHidden = panel.hidden;
                panel.hidden = panel.dataset.abilityPanel !== button.dataset.ability;
                if (!panel.hidden && wasHidden && !window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
                    revealPreview(panel);
                }
            });
            if (more && button.dataset.abilityHref) more.href = button.dataset.abilityHref;
        });
    });
}

/** 大幅演示中的倍率和侧栏布局仅改变示意图，控制始终由用户触发。 */
const imagePreview = document.querySelector<HTMLElement>('[data-image-preview]');
imagePreview?.querySelectorAll<HTMLButtonElement>('[data-image-scale]').forEach(button => {
    button.disabled = false;
    button.addEventListener('click', () => {
        const scale = button.dataset.imageScale ?? '2';
        imagePreview.dataset.scale = scale;
        imagePreview.querySelectorAll('[data-image-scale]').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
        const factor = imagePreview.querySelector('[data-image-factor]');
        if (factor) factor.textContent = `${scale}×`;
    });
});
const desktopPreview = document.querySelector<HTMLElement>('[data-desktop-preview]');
desktopPreview?.querySelectorAll<HTMLButtonElement>('[data-preview-layout]').forEach(button => {
    button.disabled = false;
    button.addEventListener('click', () => {
        desktopPreview.dataset.layout = button.dataset.previewLayout ?? 'single';
        desktopPreview.querySelectorAll('[data-preview-layout]').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
    });
});
