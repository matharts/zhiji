import test from "node:test";
import assert from "node:assert/strict";
import { fetchLatestRelease } from "../src/scripts/release.ts";

// 未发布、服务器错误和离线分别返回对应状态，不把网络失败当作尚未发布。
test("release lookup distinguishes an unpublished release from a failed request", async () => {
    assert.deepEqual(await fetchLatestRelease(async () => new Response(null, { status: 404 })), { state: "unpublished" });
    assert.deepEqual(await fetchLatestRelease(async () => new Response(null, { status: 503 })), { state: "unavailable" });
    assert.deepEqual(await fetchLatestRelease(async () => { throw new Error("offline"); }), { state: "unavailable" });
});

// 已发布版本只暴露可识别的项目下载资产，过滤无关外部文件。
test("release lookup exposes only recognized project assets", async () => {
    const result = await fetchLatestRelease(async () => new Response(
        JSON.stringify({
            tag_name: "v1.0.0",
            assets: [
                { name: "ziwei-macos.dmg", browser_download_url: "https://github.com/matharts/zhiji/releases/download/v1.0.0/ziwei-macos.dmg" },
                { name: "other.exe", browser_download_url: "https://example.com/other.exe" },
            ],
        }), { status: 200 },
    ));
    assert.ok(result.state === "published");
    assert.equal(result.tag, "v1.0.0");
    assert.equal(result.groups.macos.length, 1);
    assert.deepEqual(result.groups.windows, []);
});
