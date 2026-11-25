import { isPlatform } from "./download-data.ts";
import { fetchLatestRelease, type ReleaseResult } from "./release.ts";

/** 根据查询结果显示发布状态及实际可用的下载入口。 */
export function updateReleaseLinks(release: ReleaseResult): void {
    const groups = release.state === "published" ? release.groups : null;
    const hasDownloads = groups !== null && Object.values(groups).some(assets => assets.length > 0);
    document.querySelectorAll<HTMLElement>("[data-download]").forEach(element => {
        const platform = element.dataset.download;
        element.hidden = isPlatform(platform) ? !groups?.[platform].length : !hasDownloads;
    });
    const unpublished = document.querySelector<HTMLElement>("[data-release-unpublished]");
    if (unpublished) unpublished.hidden = release.state !== "unpublished";

    const summary = document.querySelector("[data-release-summary]");
    if (!summary) return;
    switch (release.state) {
        case "unpublished": summary.textContent = "正式版尚未发布"; break;
        case "unavailable": summary.textContent = "暂时无法获取版本信息"; break;
        case "published":
            summary.textContent = hasDownloads
                ? `最新正式版 ${release.tag}`
                : `正式版 ${release.tag} 暂无可用安装包`;
            break;
    }
}

/** 下载页由安装包面板负责查询，其余页面共用一次发布查询。 */
export async function loadReleaseLinks(): Promise<void> {
    if (document.querySelector("[data-release-status]")) return;
    updateReleaseLinks(await fetchLatestRelease());
}
