// 每条箭头只保留一个绘制任务，切换落点时同步清理临时箭头尖端。
const drawings = new WeakMap<SVGPathElement, () => void>();
const activeDrawings = new Set<() => void>();
const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)');

/** 停止绘制并还原完整箭头，避免快速切换或隐藏页面后留下半条线。 */
export function finishArrowDrawing(path: SVGPathElement): void {
    drawings.get(path)?.();
}

/** 按弧长匀速出线，尖端沿曲线切向前进，完成后交回原 SVG 标记。 */
export function drawArrow(path: SVGPathElement, duration = 480): void {
    finishArrowDrawing(path);
    if (reducedMotion.matches || !path.parentElement) return;
    const length = path.getTotalLength();
    const marker = path.getAttribute('marker-end');
    if (!length || !marker) return;

    const properties = ['marker-end', 'stroke-dasharray', 'stroke-dashoffset'];
    const original = properties.map(property => ({
        property,
        value: path.style.getPropertyValue(property),
        priority: path.style.getPropertyPriority(property),
    }));
    const normalizedLength = Number(path.getAttribute('pathLength')) || length;
    const appearance = getComputedStyle(path);
    const tip = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    tip.setAttribute('marker-end', marker);
    tip.setAttribute('aria-hidden', 'true');
    tip.style.stroke = 'none';
    tip.style.fill = 'none';
    tip.style.strokeWidth = appearance.strokeWidth;
    tip.style.vectorEffect = appearance.vectorEffect;
    tip.style.color = appearance.color;
    tip.style.visibility = 'hidden';
    path.after(tip);
    path.style.markerEnd = 'none';
    path.style.strokeDasharray = `${normalizedLength} ${normalizedLength}`;
    path.style.strokeDashoffset = String(normalizedLength);
    let frame = 0;
    let started: number | undefined;

    /** 正常结束与中断共用还原路径，不依赖下一帧一定执行。 */
    const finish = () => {
        cancelAnimationFrame(frame);
        tip.remove();
        original.forEach(({ property, value, priority }) => {
            if (value) path.style.setProperty(property, value, priority);
            else path.style.removeProperty(property);
        });
        drawings.delete(path);
        activeDrawings.delete(finish);
    };
    drawings.set(path, finish);
    activeDrawings.add(finish);

    /** 线身与尖端共用同一进度，短切向线只承载标记，不额外绘制线身。 */
    const tick = (time: number) => {
        started ??= time;
        const progress = Math.min((time - started) / duration, 1);
        if (progress >= 1) {
            finish();
            return;
        }
        const distance = length * progress;
        const end = path.getPointAtLength(distance);
        const before = path.getPointAtLength(Math.max(0, distance - .5));
        path.style.strokeDashoffset = String(normalizedLength * (1 - progress));
        tip.setAttribute('d', `M${before.x} ${before.y}L${end.x} ${end.y}`);
        tip.style.visibility = progress > 0 ? 'visible' : 'hidden';
        frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
}

/** 页面离开、切后台或改为减少动态效果时，立即释放所有逐帧任务。 */
function finishAllDrawings(): void {
    activeDrawings.forEach(finish => finish());
}
reducedMotion.addEventListener('change', finishAllDrawings);
window.addEventListener('pagehide', finishAllDrawings);
document.addEventListener('visibilitychange', () => {
    if (document.hidden) finishAllDrawings();
});
