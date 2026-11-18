import { gsap } from "gsap";
import { ScrollTrigger } from "gsap/ScrollTrigger";
import Lenis from "lenis";
import "lenis/dist/lenis.css";
import { mountScene } from "./scene.ts";
import { fetchLatestRelease } from "./release.ts";
import "./flight-demo.ts";

gsap.registerPlugin(ScrollTrigger);

const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
const finePointer = window.matchMedia("(pointer: fine)");
const releaseSummaries = document.querySelectorAll("[data-release-summary]");
const replayButton = document.querySelector<HTMLButtonElement>("[data-replay-marks]");
// 功能尚未挂载时使用空回调，允许偏好变化和页面退出统一执行清理。
let cleanMotion = () => {};
// 异步场景挂载成功后替换为实际 GPU 资源清理函数。
let cleanScene = () => {};
// 标注时间轴可用时替换为重播动作。
let replayMarks = () => {};
let sceneRevision = 0;

replayButton?.addEventListener("click", () => replayMarks());
// 展厅切换后重测滚动触发位置，标注主题从完整笔迹重新播放。
window.addEventListener("zhiji:panel-change", (event) => {
    ScrollTrigger.refresh();
    if (event instanceof CustomEvent && event.detail === "annotation") replayMarks();
});

/** 将发布状态同步到首页所有版本摘要，不把失败显示为已有安装包。 */
async function updateReleaseSummary() {
    releaseSummaries.forEach((item) => { item.textContent = "正在查询版本…"; });
    const release = await fetchLatestRelease();
    let message = "版本信息暂不可用，请查看下载页";
    if (release.state === "unpublished") message = "正式版尚未发布";
    if (release.state === "published") {
        const hasAssets = Object.values(release.groups).some((assets) => assets.length);
        message = hasAssets
            ? `最新正式版 ${release.tag} · 查看安装包`
            : `最新正式版 ${release.tag} · 安装包信息见下载页`;
    }
    releaseSummaries.forEach((item) => { item.textContent = message; });
}

/** 清理旧动画后按动效和指针偏好安装滚动效果，并保留可逆清理入口。 */
function setupMotion() {
    cleanMotion();
    cleanMotion = () => {};
    replayMarks = () => {};
    if (replayButton) replayButton.hidden = true;
    if (reduced.matches) return;
    const context = gsap.context(() => {
        gsap.from(".hero-copy > *", {
            y: 18,
            opacity: 0.65,
            duration: 0.85,
            stagger: 0.09,
            ease: "power3.out",
        });
        const marks = document.querySelectorAll<SVGGeometryElement>(".marking-chart svg ellipse, .marking-chart svg path");
        const marksTimeline = gsap.timeline({ paused: true });
        marks.forEach((mark, index) => {
            const length = mark.getTotalLength();
            marksTimeline.fromTo(mark,
                { strokeDasharray: `${length} ${length}`, strokeDashoffset: length },
                {
                    strokeDashoffset: 0,
                    duration: 0.8,
                    ease: "power2.out",
                },
                index * 0.2,
            );
        });
        if (marks.length) {
            ScrollTrigger.create({ trigger: ".marking-chart", start: "top 80%", once: true, onEnter: () => marksTimeline.restart() });
            replayMarks = () => marksTimeline.restart();
            if (replayButton) replayButton.hidden = false;
        }
    });
    const lenis = finePointer.matches
        ? new Lenis({ lerp: 0.12, anchors: true, autoRaf: false })
        : null;
    // GSAP 提供秒，Lenis 接收毫秒；两者共用同一帧调度。
    const tick = (time: number) => lenis?.raf(time * 1000);
    if (lenis) {
        lenis.on("scroll", ScrollTrigger.update);
        gsap.ticker.add(tick);
    }
    document.fonts?.ready.then(() => ScrollTrigger.refresh());
    cleanMotion = () => {
        context.revert();
        if (lenis) {
            gsap.ticker.remove(tick);
            lenis.destroy();
        }
    };
}

/** 用递增版本拒绝过期异步挂载，偏好已变化时立即释放晚到场景。 */
async function setupScene() {
    const revision = ++sceneRevision;
    cleanScene();
    cleanScene = () => {};
    if (!reduced.matches) {
        const dispose = await mountScene();
        if (revision !== sceneRevision || reduced.matches) dispose();
        else cleanScene = dispose;
    }
}

/** 将最新媒体偏好同时应用到页面动效和三维场景。 */
function updatePreferences() {
    setupMotion();
    setupScene();
}
reduced.addEventListener("change", updatePreferences);
finePointer.addEventListener("change", setupMotion);
window.addEventListener("pagehide", () => {
    sceneRevision += 1;
    cleanMotion();
    cleanScene();
});
window.addEventListener("pageshow", (event) => {
    if (event.persisted) updatePreferences();
});
updatePreferences();
updateReleaseSummary();
