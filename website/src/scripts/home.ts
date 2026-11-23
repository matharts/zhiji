import { gsap } from "gsap";
import Lenis from "lenis";
import "lenis/dist/lenis.css";
import { mountScene } from "./scene.ts";
import { fetchLatestRelease } from "./release.ts";
import "./flight-demo.ts";

const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
const finePointer = window.matchMedia("(pointer: fine)");
const releaseSummaries = document.querySelectorAll("[data-release-summary]");
// 功能尚未挂载时使用空回调，允许偏好变化和页面退出统一执行清理。
let cleanMotion = () => {};
// 异步场景挂载成功后替换为实际 GPU 资源清理函数。
let cleanScene = () => {};
let sceneRevision = 0;

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
    if (reduced.matches) return;
    const context = gsap.context(() => {
        gsap.from(".hero-copy > *", {
            y: 18,
            opacity: 0.65,
            duration: 0.85,
            stagger: 0.09,
            ease: "power3.out",
        });
    });
    const lenis = finePointer.matches
        ? new Lenis({ lerp: 0.12, anchors: true, autoRaf: false })
        : null;
    // GSAP 提供秒，Lenis 接收毫秒；两者共用同一帧调度。
    const tick = (time: number) => lenis?.raf(time * 1000);
    if (lenis) gsap.ticker.add(tick);
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
