const downloadRoot =
    "https://github.com/matharts/zhiji/releases/download/";

export type Platform = "macos" | "windows" | "linux";
export interface DownloadAsset { name: string; url: string; }
export type AssetGroups = Record<Platform, DownloadAsset[]>;

/** 校验页面数据属性是否为支持的平台，避免用任意键索引资源组。 */
export function isPlatform(value: string | undefined): value is Platform {
    return value === "macos" || value === "windows" || value === "linux";
}

/** 按安装包扩展名和归档文件的平台标识分类，无法识别时忽略。 */
function platformFor(name: string): Platform | null {
    const file = name.toLowerCase();
    if (/\.(dmg|pkg)$/.test(file)) return "macos";
    if (/\.(exe|msi)$/.test(file)) return "windows";
    if (/\.(deb|rpm|appimage)$/.test(file)) return "linux";
    if (!/\.(zip|tar\.gz|tgz)$/.test(file)) return null;
    if (/(^|[-_.])(macos|darwin|osx|mac)([-_.]|$)/.test(file)) return "macos";
    if (/(^|[-_.])(windows|win32|win64|win)([-_.]|$)/.test(file))
        return "windows";
    if (/(^|[-_.])(linux)([-_.]|$)/.test(file)) return "linux";
    return null;
}

/** 校验不可信发布数据，只接收本仓库发布下载目录中的可识别安装包。 */
export function groupReleaseAssets(release: unknown): AssetGroups {
    const groups: AssetGroups = { macos: [], windows: [], linux: [] };
    if (typeof release !== "object" || release === null || !("assets" in release) || !Array.isArray(release.assets)) return groups;
    const assets: unknown[] = release.assets;
    for (const asset of assets) {
        if (
            typeof asset !== "object" || asset === null ||
            !("name" in asset) || !("browser_download_url" in asset) ||
            typeof asset.name !== "string" ||
            typeof asset.browser_download_url !== "string"
        )
            continue;
        const platform = platformFor(asset.name);
        let url: URL;
        try {
            url = new URL(asset.browser_download_url);
        } catch {
            continue;
        }
        if (!platform || !url.href.startsWith(downloadRoot)) continue;
        groups[platform].push({ name: asset.name, url: url.href });
    }
    groups.linux.sort((left, right) =>
        Number(right.name.toLowerCase().endsWith(".appimage")) -
        Number(left.name.toLowerCase().endsWith(".appimage")),
    );
    return groups;
}
