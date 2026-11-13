async page => {
  const errors = [];
  const results = {};
  const origin = "http://127.0.0.1:4387";

  /** 隔离公开发布源，避免网络状态干扰本地分块与渲染验收。 */
  async function prepare(target) {
    target.on("pageerror", error => errors.push(error.message));
    target.on("console", message => {
      if (message.type() === "error" || message.text().includes("WebGPU background unavailable")) {
        errors.push(message.text());
      }
    });
    await target.route("https://api.github.com/repos/matharts/zhiji/releases/latest", route => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        tag_name: "v0.1.0",
        html_url: "https://github.com/matharts/zhiji/releases/tag/v0.1.0",
        draft: false, prerelease: false, assets: [],
      }),
    }));
  }

  /** 读取真实渲染标记、静态图可见性和本次导航加载的三维模块。 */
  async function inspect(target) {
    return target.evaluate(() => ({
      ready: document.querySelector("[data-hero-art]")?.classList.contains("is-ready") ?? false,
      gpu: !!navigator.gpu,
      posterOpacity: document.querySelector(".hero-poster")
        ? getComputedStyle(document.querySelector(".hero-poster")).opacity : null,
      chunks: performance.getEntriesByType("resource").filter(entry => /\/three\.[^/]+\.js$/.test(entry.name)).length,
    }));
  }

  await prepare(page);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.goto(origin);
  await page.waitForFunction(() => document.querySelector("[data-hero-art]")?.classList.contains("is-ready"), undefined, { timeout: 20000 });
  results.webgpu = await inspect(page);
  if (!results.webgpu.gpu || !results.webgpu.chunks) throw Error("WebGPU must render and load its chunks");
  await page.getByRole("button", { name: "暂停动画", exact: true }).click();
  await page.getByRole("button", { name: "播放动画", exact: true }).waitFor();
  await page.screenshot({ path: "output/playwright/license-bundle-cleanup/webgpu.png" });
  await page.getByRole("button", { name: "播放动画", exact: true }).click();
  await page.getByRole("button", { name: "暂停动画", exact: true }).waitFor();
  results.pauseResume = true;

  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.waitForFunction(() => !document.querySelector("[data-hero-art]")?.classList.contains("is-ready"));
  await page.waitForFunction(() => getComputedStyle(document.querySelector(".hero-poster")).opacity === "1");
  results.liveReduction = await inspect(page);
  await page.emulateMedia({ reducedMotion: "no-preference" });
  await page.waitForFunction(() => document.querySelector("[data-hero-art]")?.classList.contains("is-ready"));
  results.remount = await inspect(page);

  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(origin);
  await page.waitForLoadState("networkidle");
  results.reduced = await inspect(page);
  if (results.reduced.ready || results.reduced.chunks || results.reduced.posterOpacity !== "1") throw Error("Reduced motion must keep the static poster without Three.js");
  await page.screenshot({ path: "output/playwright/license-bundle-cleanup/reduced.png" });

  const unsupported = await page.context().newPage();
  try {
    await prepare(unsupported);
    await unsupported.emulateMedia({ reducedMotion: "no-preference" });
    await unsupported.addInitScript(() => Object.defineProperty(navigator, "gpu", { get: () => undefined, configurable: true }));
    await unsupported.goto(origin);
    await unsupported.waitForLoadState("networkidle");
    results.unsupported = await inspect(unsupported);
    if (results.unsupported.gpu || results.unsupported.ready || results.unsupported.chunks || results.unsupported.posterOpacity !== "1") throw Error("Unsupported browsers must keep the static poster without Three.js");
    await unsupported.screenshot({ path: "output/playwright/license-bundle-cleanup/unsupported.png" });
    await unsupported.goto(`${origin}/download/`);
    await unsupported.waitForLoadState("networkidle");
    results.download = await inspect(unsupported);
    if (results.download.chunks) throw Error("Download page must not load Three.js");
  } finally {
    await unsupported.close();
  }
  if (errors.length) throw Error(JSON.stringify(errors));
  results.errors = errors;
  await page.evaluate(value => { window.__bundleValidation = value; }, results);
}
