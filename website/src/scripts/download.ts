import { fetchLatestRelease } from "./release.ts";
import { isPlatform } from "./download-data.ts";

const status = document.querySelector<HTMLElement>("[data-release-status]");
const containers = document.querySelectorAll<HTMLElement>("[data-platform]");

/** 为各平台统一设置查询状态，必要时提供发布页回退链接。 */
function setPlatformMessage(message: string, linkToReleases = false): void {
    containers.forEach((container) => {
        const target = container.querySelector<HTMLElement>("[data-assets]");
        if (!target) return;
        if (!linkToReleases) {
            target.textContent = message;
            return;
        }
        const link = document.createElement("a");
        link.className = "asset-fallback";
        link.href = "https://github.com/matharts/zhiji/releases";
        link.textContent = message;
        target.replaceChildren(link);
    });
}

/** 将发布查询结果渲染为分平台下载列表，未发布和失败均保留明确下一步。 */
async function loadRelease(status: HTMLElement): Promise<void> {
    status.textContent = "正在查询最新版本…";
    setPlatformMessage("正在查询…");
    const release = await fetchLatestRelease();
    if (release.state === "unpublished") {
        status.textContent = "正式版本尚未发布，可在 GitHub 发布页查看后续更新。";
        setPlatformMessage("尚未提供安装包");
        return;
    }
    if (release.state === "unavailable") {
        status.textContent = "暂时无法获取版本信息，请前往 GitHub 发布页查看。";
        setPlatformMessage("请查看 GitHub 发布页", true);
        return;
    }
    const groups = release.groups;
    const assetCount = Object.values(groups).reduce(
        (sum, items) => sum + items.length,
        0,
    );
    status.textContent = assetCount
        ? `最新正式版 ${release.tag}，请选择对应系统的安装包。`
        : "最新正式版暂无可识别的安装包，请前往 GitHub 发布页查看。";
    containers.forEach((container) => {
        const target = container.querySelector<HTMLElement>("[data-assets]");
        if (!target) return;
        const platform = container.dataset.platform;
        const assets = isPlatform(platform) ? groups[platform] : [];
        if (!assets.length) {
            target.textContent = "暂无此平台安装包";
            return;
        }
        const list = document.createElement("ul");
        list.className = "download-list";
        assets.forEach((asset) => {
            const item = document.createElement("li");
            const link = document.createElement("a");
            link.href = asset.url;
            link.textContent = `下载 ${asset.name}`;
            item.append(link);
            list.append(item);
        });
        target.replaceChildren(list);
    });
}

if (status && containers.length) void loadRelease(status);
