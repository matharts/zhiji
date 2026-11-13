import { groupReleaseAssets } from "./download-data.ts";
import type { AssetGroups } from "./download-data.ts";

const endpoint = "https://api.github.com/repos/matharts/zhiji/releases/latest";

export type ReleaseResult =
    | { state: "unpublished" }
    | { state: "unavailable" }
    | { state: "published"; tag: string; groups: AssetGroups };

/** 查询最新正式发布，区分未发布和暂不可用；八秒超时后返回可降级状态。 */
export async function fetchLatestRelease(fetcher: typeof fetch = fetch): Promise<ReleaseResult> {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 8000);
    try {
        const response = await fetcher(endpoint, {
            headers: { Accept: "application/vnd.github+json" },
            signal: controller.signal,
        });
        if (response.status === 404) return { state: "unpublished" };
        if (!response.ok) return { state: "unavailable" };
        const release: unknown = await response.json();
        const tag = typeof release === "object" && release !== null && "tag_name" in release
            ? String(release.tag_name || "") : "";
        return {
            state: "published",
            tag,
            groups: groupReleaseAssets(release),
        };
    } catch {
        return { state: "unavailable" };
    } finally {
        clearTimeout(timeout);
    }
}
