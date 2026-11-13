import test from "node:test";
import assert from "node:assert/strict";
import { groupReleaseAssets } from "../src/scripts/download-data.ts";

const root =
    "https://github.com/matharts/zhiji/releases/download/v1.0.0/";
// 构造发布资产元数据，允许覆盖地址以验证外部或非法下载链接。
const asset = (name: string, url = root + name) => ({
    name,
    browser_download_url: url,
});

// 仅认可本仓库的桌面安装包，排除校验文件、外部仓库和危险地址协议。
test("only trusted, recognizable desktop packages become download links", () => {
    const groups = groupReleaseAssets({
        assets: [
            asset("ziwei-macos-arm64.dmg"),
            asset("ziwei-windows-x64.exe"),
            asset("ziwei-linux-x64.tar.gz"),
            asset("ziwei-macos-x64.zip"),
            asset("ziwei.zip"),
            asset("ziwei-windows-x64.exe.sha256"),
            asset("fake.exe", "https://example.com/fake.exe"),
            asset(
                "other.exe",
                "https://github.com/other/repo/releases/download/v1/other.exe",
            ),
            asset("bad.exe", "javascript:alert(1)"),
        ],
    });
    assert.deepEqual(
        groups.macos.map((item) => item.name),
        ["ziwei-macos-arm64.dmg", "ziwei-macos-x64.zip"],
    );
    assert.deepEqual(
        groups.windows.map((item) => item.name),
        ["ziwei-windows-x64.exe"],
    );
    assert.deepEqual(
        groups.linux.map((item) => item.name),
        ["ziwei-linux-x64.tar.gz"],
    );
});

// 发布数据缺失时返回各平台空列表，供界面呈现无可用下载状态。
test("missing release assets produce empty platform lists", () => {
    assert.deepEqual(groupReleaseAssets(null), {
        macos: [],
        windows: [],
        linux: [],
    });
    assert.deepEqual(groupReleaseAssets({ assets: null }), {
        macos: [],
        windows: [],
        linux: [],
    });
});

// Linux 优先提供支持应用内更新的 AppImage，同时保留系统包渠道。
test("Linux offers AppImage before system packages", () => {
    const groups = groupReleaseAssets({
        assets: [asset("zhiji.deb"), asset("zhiji-linux.tar.gz"), asset("zhiji.AppImage")],
    });
    assert.deepEqual(groups.linux.map(item => item.name),
        ["zhiji.AppImage", "zhiji.deb", "zhiji-linux.tar.gz"]);
});
