import test from 'node:test';
import assert from 'node:assert/strict';
import { PRELUDE_DURATION, RETURN_DURATION, FLIGHT_DURATION, preludeFrame } from '../src/scripts/scene-choreography.ts';

// 开场不能提前闪出命盘，阴阳以持续流动的雾场停留。
test('cinematic prelude starts in chaos and resolves into a readable yin yang', () => {
    const start = preludeFrame(0);
    assert.equal(start.phase,'chaos');
    assert.ok(start.chaos > 0);
    assert.equal(start.chart,0);
    assert.equal(start.condense,0);
    const yinYang = preludeFrame(5.5);
    assert.equal(yinYang.phase,'yin-yang');
    assert.equal(yinYang.condense,1);
    assert.equal(yinYang.chaos,1);
    assert.equal(yinYang.chart,0);
    assert.equal(yinYang.grid,0);
});

// 阴阳直接化成一盘，开场不得提前展开三盘；交接姿态与原飞化保持一致。
test('yin yang becomes one flat chart before the existing flight sequence', () => {
    const chart = preludeFrame(9.3);
    assert.equal(chart.phase,'merge');
    assert.equal(chart.chart,1);
    assert.equal(chart.spread,0);
    assert.equal(chart.chaos,0);
    const handoff = preludeFrame(PRELUDE_DURATION);
    assert.equal(handoff.active,false);
    assert.equal(handoff.phase,'flight');
    assert.equal(handoff.chart,1);
    assert.equal(handoff.spread,0);
    assert.equal(handoff.twist,0);
    assert.equal(handoff.scale,1);
    assert.equal(handoff.cameraZ,14);
    assert.equal(handoff.chaos,0);
    assert.equal(handoff.particles,0);
    const before = preludeFrame(PRELUDE_DURATION - .001);
    assert.ok(Math.abs(before.cameraZ - handoff.cameraZ) < .001);
    assert.equal(before.spread,handoff.spread);
});

// 密集采样防止透明度、尺度或时间轴接点出现超调及非有限数值。
test('every prelude frame stays within the rendering envelope', () => {
    for(let time=0;time<=PRELUDE_DURATION+1;time+=.02) {
        const frame = preludeFrame(time);
        for(const value of [frame.chaos,frame.chart,frame.particles,frame.polarity,frame.condense,frame.grid,frame.spread,frame.pulse]) {
            assert.ok(Number.isFinite(value) && value>=0 && value<=1);
        }
        assert.ok(frame.scale>=.75 && frame.scale<=1);
        assert.equal(frame.spread,0);
        if(time<7.2) assert.equal(frame.chart,0);
    }
});

// 云气保持到命盘显现，两者交接不能出现空场。
test('the flowing yin yang stays present until the chart begins to emerge', () => {
    const forming = preludeFrame(3.8);
    assert.ok(forming.condense>.8);
    assert.equal(forming.chaos,1);
    assert.equal(preludeFrame(6.5).chaos,1);
    for(let time=6.5;time<=8.9;time+=.02) {
        const frame = preludeFrame(time);
        assert.ok(Math.max(frame.chaos,frame.chart)>.3);
    }
    assert.equal(preludeFrame(8.9).chart,1);
    assert.equal(preludeFrame(8.9).chaos,0);
});

// 循环前消去盘面，并使下一轮雾场、粒子、镜头与第一帧精确衔接。
test('the full sequence returns smoothly to chaos with the actual flight duration', () => {
    const flightDuration = FLIGHT_DURATION;
    const returnStart = PRELUDE_DURATION + flightDuration;
    const start = preludeFrame(0,flightDuration);
    const end = preludeFrame(returnStart + RETURN_DURATION,flightDuration);
    assert.equal(preludeFrame(returnStart-.001,flightDuration).phase,'flight');
    assert.equal(preludeFrame(returnStart,flightDuration).phase,'return');
    for(const key of ['chaos','condense','chart','particles','cameraZ','effectTime'] as const) {
        assert.ok(Math.abs(start[key]-end[key])<1e-10,key);
    }
    const middle = preludeFrame(returnStart+RETURN_DURATION/2,flightDuration);
    assert.ok(middle.chart>0 && middle.chart<1);
    assert.ok(middle.chaos>0 && middle.chaos<1);
    assert.equal(preludeFrame(PRELUDE_DURATION+2,2).phase,'return');
});
