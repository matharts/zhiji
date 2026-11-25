import { fetchLatestRelease } from "./release.ts";
import { updateReleaseLinks } from "./release-links.ts";
import { isPlatform, type Platform } from "./download-data.ts";

const status = document.querySelector<HTMLElement>("[data-release-status]");
const title = document.querySelector<HTMLElement>("[data-install-title]");
const options = document.querySelector<HTMLElement>("[data-install-options]");
const fallback = document.querySelector<HTMLElement>("[data-download-fallback]");
const containers = document.querySelectorAll<HTMLElement>("[data-platform]");
const buttons = document.querySelectorAll<HTMLButtonElement>("[data-select-platform]");
const retry = document.querySelector<HTMLButtonElement>("[data-release-retry]");
const userAgent = navigator.userAgent;
const detected: Platform = /Windows/i.test(userAgent) ? "windows"
    : /Linux/i.test(userAgent) && !/Android/i.test(userAgent) ? "linux" : "macos";
let available: Platform[] = [];

/** 优先打开有安装包的深链接平台，其次选择当前系统或首个可用平台。 */
function selectFromLocation(): void {
    const linked = window.location.hash.replace(/^#platform-/, "");
    const preferred = isPlatform(linked) && available.includes(linked) ? linked : detected;
    const platform = available.includes(preferred) ? preferred : available[0];
    if (platform) selectPlatform(platform, false);
}

/** 平台选择只切换面板，由用户决定是否下载文件。 */
function selectPlatform(platform: Platform, animate: boolean): void {
    buttons.forEach(button => button.setAttribute("aria-pressed", String(button.dataset.selectPlatform === platform)));
    containers.forEach(container => {
        container.hidden = container.dataset.platform !== platform;
        // 只为用户切换的可见面板播放一次短过渡。
        container.getAnimations().forEach(animation => animation.cancel());
        if (!container.hidden && animate && !window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
            container.animate(
                [{ opacity: 0.65, transform: "translateY(6px)" }, { opacity: 1, transform: "translateY(0)" }],
                { duration: 240, easing: "cubic-bezier(.16,1,.3,1)" },
            );
        }
    });
}

/** 查询成功且有可用安装包时才展示系统选择和安装说明。 */
async function loadRelease(status: HTMLElement): Promise<void> {
    if (retry) { retry.hidden = true; retry.disabled = true; }
    status.textContent = "正在查询最新版本…";
    const release = await fetchLatestRelease();
    updateReleaseLinks(release);
    if (release.state === "unpublished") {
        status.textContent = "知几正式版尚未发布。";
        return;
    }
    if (release.state === "unavailable") {
        status.textContent = "暂时无法获取版本信息。";
        if (retry) { retry.hidden = false; retry.disabled = false; }
        return;
    }

    available = [];
    buttons.forEach(button => {
        const platform = button.dataset.selectPlatform;
        button.hidden = !isPlatform(platform) || release.groups[platform].length === 0;
        if (!button.hidden && isPlatform(platform)) available.push(platform);
    });
    if (!available.length) {
        status.textContent = `正式版 ${release.tag} 暂无可用安装包。`;
        return;
    }

    containers.forEach(container => {
        const target = container.querySelector<HTMLElement>("[data-assets]");
        const platform = container.dataset.platform;
        if (!target || !isPlatform(platform) || !available.includes(platform)) return;
        const list = document.createElement("ul");
        list.className = "download-list";
        release.groups[platform].forEach(asset => {
            const item = document.createElement("li");
            const link = document.createElement("a");
            link.href = asset.url;
            link.textContent = `下载 ${asset.name}`;
            item.append(link);
            list.append(item);
        });
        target.replaceChildren(list);
    });
    if (title) title.textContent = "选择桌面系统";
    status.textContent = `最新正式版 ${release.tag}`;
    selectFromLocation();
    if (options) options.hidden = false;
    if (fallback) fallback.hidden = true;
}

buttons.forEach(button => button.addEventListener("click", () => {
    const platform = button.dataset.selectPlatform;
    if (!isPlatform(platform) || !available.includes(platform)) return;
    selectPlatform(platform, true);
    history.replaceState(null, "", `#platform-${platform}`);
}));
window.addEventListener("hashchange", selectFromLocation);

if (status) void loadRelease(status);
retry?.addEventListener("click", () => { if (status && !retry.disabled) void loadRelease(status); });
