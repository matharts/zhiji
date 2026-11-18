/** 阴阳直接化为平面命盘；飞化结束后经消隐回到混沌，整段循环。 */
export const PRELUDE_DURATION = 9.8;
export const RETURN_DURATION = 2.4;
export const FLIGHT_DURATION = 12.2;
export type ScenePhase = 'chaos' | 'yin-yang' | 'merge' | 'flight' | 'return';

/** 三次平滑插值在片段接点保持零速度，暂停或重播时不依赖墙上时钟。 */
function ramp(time: number, start: number, end: number): number {
    const value = Math.max(0, Math.min(1, (time - start) / (end - start)));
    return value * value * (3 - 2 * value);
}

/** 同一个周期坐标控制阴阳化盘与循环接缝；飞化时长取实际子时间轴。 */
export function preludeFrame(time: number, flightDuration = FLIGHT_DURATION) {
    const returnStart = PRELUDE_DURATION + flightDuration;
    if (time >= returnStart) {
        const dissolve = ramp(time, returnStart, returnStart + RETURN_DURATION);
        return {
            active: true, phase: 'return' as ScenePhase,
            // 以负时间接近下一轮的零时刻，使雾场与粒子在循环接缝连续。
            effectTime: time - returnStart - RETURN_DURATION,
            polarity: 0, condense: 0, grid: 0, chaos: dissolve,
            chart: 1 - dissolve, particles: .6 * dissolve,
            spread: 0, twist: 0, scale: 1,
            cameraZ: 14 + 1.3 * dissolve, pulse: 0,
        };
    }
    const active = time < PRELUDE_DURATION;
    const polarity = ramp(time, .8, 4.2);
    const condense = ramp(time, 1.8, 4.3);
    const grid = ramp(time, 6.6, 8.8);
    const phase: ScenePhase = time < 5.3 ? 'chaos' : time < 6.6 ? 'yin-yang'
        : active ? 'merge' : 'flight';
    return {
        active, phase, polarity, condense, grid, effectTime: time,
        // 阴阳始终是云气，直到命盘开始显现才消散，不经过实体圆盘。
        chaos: 1 - ramp(time, 6.5, 8.9),
        chart: active ? ramp(time, 7.2, 8.9) : 1,
        particles: (1 - ramp(time, 8.6, 9.4)) * (0.6 + grid * .3) * (1 - condense * .65),
        spread: 0,
        twist: 0,
        scale: 1,
        cameraZ: active ? 14 + 1.3 * (1 - polarity) - .45 * Math.sin(grid * Math.PI) : 14,
        // 合盘时只出现一次柔和光晕，不用高频闪光掩盖转场。
        pulse: ramp(time, 8, 8.6) * (1 - ramp(time, 8.6, 9.5)),
    };
}
