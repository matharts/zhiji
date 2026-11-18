/** 官网演示只切换呈现状态，不运行排盘、不修改用户资料。 */
const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

/** 分组按钮共享可访问选中状态，原生按钮保留 Tab 与空格操作。 */
function setPressed(group: Element, selected: HTMLButtonElement): void {
    group.querySelectorAll('button').forEach(button => {
        button.setAttribute('aria-pressed', String(button === selected));
    });
}

const dateScene = document.querySelector<HTMLElement>('[data-date-scene]');
const dateModes = dateScene?.querySelector<HTMLElement>('[data-date-modes]');
if (dateScene && dateModes) {
    const examples = [
        { fields: [['年', '2000'], ['月', '06'], ['日', '15']], result: '公历日期 · 结合出生地校时' },
        { fields: [['农历年', '2000'], ['月', '四月'], ['日', '十四']], result: '农历日期 · 按输入资料确认月份与闰月' },
        { fields: [['年柱', '年干支'], ['月柱', '月干支'], ['日柱', '日干支']], result: '输入四柱与时间范围 · 选择候选后确认' },
    ];
    dateModes.hidden = false;
    dateModes.querySelectorAll<HTMLButtonElement>('[data-date-mode]').forEach(button => {
        button.addEventListener('click', () => {
            const example = examples[Number(button.dataset.dateMode)];
            if (!example) return;
            setPressed(dateModes, button);
            dateScene.querySelectorAll('[data-date-fields]>div').forEach((field, index) => {
                const values = example.fields[index];
                const label = field.querySelector('small');
                const value = field.querySelector('strong');
                if (values && label && value) {
                    label.textContent = values[0] ?? '';
                    value.textContent = values[1] ?? '';
                }
            });
            const timeLabel = dateScene.querySelector('.date-time>span');
            const timeValue = dateScene.querySelector('.date-time>strong');
            if (timeLabel && timeValue) {
                const reverse = button.dataset.dateMode === '2';
                timeLabel.textContent = reverse ? '时柱' : '出生时间';
                timeValue.textContent = reverse ? '时干支' : '10 : 30';
            }
            const result = dateScene.querySelector('[data-date-result]');
            if (result) result.textContent = example.result;
        });
    });
}

const annotation = document.querySelector<HTMLElement>('[data-annotation-scene]');
const markTools = annotation?.querySelector<HTMLElement>('[data-mark-tools]');
if (annotation && markTools) {
    markTools.hidden = false;
    markTools.querySelectorAll<HTMLButtonElement>('[data-mark-toggle]').forEach(button => {
        button.addEventListener('click', () => {
            const enabled = button.getAttribute('aria-pressed') !== 'true';
            button.setAttribute('aria-pressed', String(enabled));
            const mark = annotation.querySelector<HTMLElement | SVGElement>(`[data-mark="${button.dataset.markToggle}"]`);
            // SVG 没有 HTML hidden 属性，统一使用 visibility 切换。
            if (mark) mark.style.visibility = enabled ? 'visible' : 'hidden';
        });
    });
}

const exportScene = document.querySelector<HTMLElement>('[data-export-scene]');
const exportControls = exportScene?.querySelector<HTMLElement>('[data-export-controls]');
if (exportScene && exportControls) {
    exportControls.hidden = false;
    exportControls.querySelectorAll<HTMLButtonElement>('[data-export-scale]').forEach(button => {
        button.addEventListener('click', () => {
            const scale = button.dataset.exportScale ?? '2';
            setPressed(exportControls, button);
            exportScene.dataset.scale = scale;
            const factor = exportScene.querySelector('[data-export-factor]');
            if (factor) factor.textContent = `${scale}×`;
        });
    });
}

const appearance = document.querySelector<HTMLElement>('[data-appearance]');
const appearanceControls = appearance?.querySelector<HTMLElement>('[data-appearance-controls]');
if (appearance && appearanceControls) {
    appearanceControls.hidden = false;
    appearanceControls.querySelectorAll<HTMLButtonElement>('[data-appearance-mode]').forEach(button => {
        button.addEventListener('click', () => {
            setPressed(appearanceControls, button);
            appearance.dataset.appearance = button.dataset.appearanceMode;
        });
    });
}

const keyboard = document.querySelector<HTMLElement>('[data-keyboard-scene]');
const keyControls = keyboard?.querySelector<HTMLElement>('[data-key-controls]');
if (keyboard && keyControls) {
    keyControls.hidden = false;
    keyControls.querySelectorAll<HTMLButtonElement>('[data-key]').forEach(button => {
        button.addEventListener('click', () => {
            const letter = keyboard.querySelector('[data-key-letter]');
            const caption = keyboard.querySelector('[data-key-caption]');
            const key = button.dataset.key ?? 'S';
            setPressed(keyControls, button);
            if (letter) {
                letter.textContent = key;
                letter.getAnimations().forEach(animation => animation.cancel());
                if (!reducedMotion.matches) letter.animate([
                    { transform: 'rotateX(17deg) translateY(0)' },
                    { transform: 'rotateX(17deg) translateY(5px)' },
                    { transform: 'rotateX(17deg) translateY(0)' },
                ], { duration: 220, easing: 'ease-out' });
            }
            if (caption) caption.textContent = `${button.textContent?.trim()}${key === 'Z' ? '标注' : '草稿'} · ⌘ ${key}`;
        });
    });
}

// 重点产品场景只在首次入屏时轻微展开；不隐藏内容，也不持续占用动画帧。
const stage = document.querySelector<HTMLElement>('[data-stage-reveal]');
if (stage) {
    const observer = new IntersectionObserver(entries => {
        if (!entries.some(entry => entry.isIntersecting)) return;
        observer.disconnect();
        if (!reducedMotion.matches) stage.animate([
            { transform: 'translateY(24px)', opacity: .75 },
            { transform: 'translateY(0)', opacity: 1 },
        ], { duration: 650, easing: 'cubic-bezier(.2,.8,.2,1)' });
    }, { threshold: .12 });
    observer.observe(stage);
    window.addEventListener('pagehide', () => observer.disconnect(), { once: true });
}
