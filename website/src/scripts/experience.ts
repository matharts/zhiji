/** 展厅通过显式操作切换主题；无脚本时所有主题按顺序可读。 */
const exhibit = document.querySelector<HTMLElement>('[data-exhibit]');
if (exhibit) {
    const tabs = [...exhibit.querySelectorAll<HTMLButtonElement>('[data-exhibit-tab]')];
    const panels = [...exhibit.querySelectorAll<HTMLElement>('[data-exhibit-panel]')];
    const list = exhibit.querySelector<HTMLElement>('[role="tablist"]');
    const select = (id: string, focus = false) => {
        if (!panels.some(panel => panel.dataset.exhibitPanel === id)) return;
        tabs.forEach(tab => {
            const selected = tab.dataset.exhibitTab === id;
            tab.setAttribute('aria-selected', String(selected));
            tab.tabIndex = selected ? 0 : -1;
            if (selected && focus) tab.focus();
        });
        panels.forEach(panel => { panel.hidden = panel.dataset.exhibitPanel !== id; });
        window.dispatchEvent(new CustomEvent('zhiji:panel-change', { detail: id }));
    };
    tabs.forEach((tab, index) => {
        tab.addEventListener('click', () => {
            const id = tab.dataset.exhibitTab!;
            select(id);
            history.replaceState(null, '', `#exhibit-${id}`);
        });
        tab.addEventListener('keydown', event => {
            let next = index;
            if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = (index + 1) % tabs.length;
            else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = (index + tabs.length - 1) % tabs.length;
            else if (event.key === 'Home') next = 0;
            else if (event.key === 'End') next = tabs.length - 1;
            else return;
            event.preventDefault();
            select(tabs[next]!.dataset.exhibitTab!, true);
        });
    });
    const readHash = () => {
        const id = location.hash.replace('#exhibit-', '');
        const valid = panels.some(panel => panel.dataset.exhibitPanel === id);
        select(valid ? id : 'input');
        if (valid) requestAnimationFrame(() => exhibit.scrollIntoView({ block: 'start' }));
    };
    panels.forEach(panel => {
        panel.setAttribute('role', 'tabpanel');
        panel.setAttribute('aria-labelledby', `tab-${panel.dataset.exhibitPanel}`);
    });
    if (list) list.hidden = false;
    exhibit.classList.add('is-interactive');
    readHash();
    window.addEventListener('hashchange', readHash);
}

// 官网只改变参考数据的呈现方式，不触发新的排盘计算。
const detail = document.querySelector<HTMLElement>('[data-palace-view]');
const detailSwitch = document.querySelector<HTMLFieldSetElement>('[data-detail-switch]');
if (detail && detailSwitch) {
    detailSwitch.hidden = false;
    detailSwitch.addEventListener('change', event => {
        if (event.target instanceof HTMLInputElement) detail.dataset.palaceView = event.target.value;
    });
}
const shortcutSwitch = document.querySelector<HTMLFieldSetElement>('[data-shortcut-switch]');
const shortcutRows = document.querySelector<HTMLElement>('[data-shortcut-platform]');
if (shortcutSwitch && shortcutRows) {
    shortcutSwitch.hidden = false;
    shortcutSwitch.addEventListener('change', event => {
        if (event.target instanceof HTMLInputElement) shortcutRows.dataset.shortcutPlatform = event.target.value;
    });
}

// 故事目录按实际进入视口的章节更新，点击仍使用原生锚点导航。
const chapters = [...document.querySelectorAll<HTMLElement>('.story > section[id]')];
const progressLinks = [...document.querySelectorAll<HTMLAnchorElement>('.journey-progress a')];
if (chapters.length && progressLinks.length) {
    const observer = new IntersectionObserver(entries => {
        const current = entries.find(entry => entry.isIntersecting);
        if (!current) return;
        progressLinks.forEach(link => {
            if (link.hash === `#${current.target.id}`) link.setAttribute('aria-current', 'step');
            else link.removeAttribute('aria-current');
        });
    }, { rootMargin: '-25% 0px -50% 0px' });
    chapters.forEach(chapter => observer.observe(chapter));
    window.addEventListener('pagehide', () => observer.disconnect(), { once: true });
}
