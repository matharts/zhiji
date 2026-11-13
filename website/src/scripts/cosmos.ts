const canvas = document.querySelector<HTMLCanvasElement>("[data-cosmos]");
const context = canvas?.getContext("2d");

/** 由整数坐标生成稳定归一化噪声种子，使重绘星空保持一致。 */
function hash(x: number, y: number): number {
    let value = Math.imul(x, 374761393) + Math.imul(y, 668265263);
    value = Math.imul(value ^ (value >>> 13), 1274126177);
    return ((value ^ (value >>> 16)) >>> 0) / 4294967295;
}

/** 对网格随机值平滑插值，生成连续星云纹理。 */
function noise(x: number, y: number): number {
    const ix = Math.floor(x);
    const iy = Math.floor(y);
    const fx = x - ix;
    const fy = y - iy;
    const sx = fx * fx * (3 - 2 * fx);
    const sy = fy * fy * (3 - 2 * fy);
    const top = hash(ix, iy) * (1 - sx) + hash(ix + 1, iy) * sx;
    const bottom = hash(ix, iy + 1) * (1 - sx) + hash(ix + 1, iy + 1) * sx;
    return top * (1 - sy) + bottom * sy;
}

/** 按视口尺寸绘制静态星空，限制像素倍率以控制背景开销。 */
function drawSky() {
    if (!canvas || !context) return;
    const width = Math.max(1, window.innerWidth);
    const height = Math.max(1, window.innerHeight);
    const ratio = Math.min(window.devicePixelRatio || 1, 1.5);
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, width, height);

    const haze = document.createElement("canvas");
    haze.width = Math.max(100, Math.ceil(width / 7));
    haze.height = Math.max(100, Math.ceil(height / 7));
    const hazeContext = haze.getContext("2d");
    if (hazeContext) {
        const image = hazeContext.createImageData(haze.width, haze.height);
        for (let y = 0; y < haze.height; y += 1) {
            for (let x = 0; x < haze.width; x += 1) {
                const u = x / haze.width;
                const v = y / haze.height;
                const ridge = 0.63 - u * 0.28 + Math.sin(u * 6) * 0.06;
                const band = Math.exp(-((v - ridge) ** 2) / 0.055);
                const cloud =
                    noise(x * 0.025, y * 0.025) * 0.55 +
                    noise(x * 0.05, y * 0.05) * 0.27 +
                    noise(x * 0.1, y * 0.1) * 0.18;
                const alpha =
                    Math.pow(Math.max(0, (cloud - 0.28) * 1.6), 1.6) * band;
                const index = (y * haze.width + x) * 4;
                image.data[index] = 117;
                image.data[index + 1] = 80;
                image.data[index + 2] = 174;
                image.data[index + 3] = Math.min(60, Math.round(alpha * 58));
            }
        }
        hazeContext.putImageData(image, 0, 0);
        context.imageSmoothingEnabled = true;
        context.drawImage(haze, 0, 0, width, height);
    }

    const count = Math.min(
        380,
        Math.max(105, Math.round((width * height) / 3700)),
    );
    for (let i = 0; i < count; i += 1) {
        const x = hash(i + 17, 51) * width;
        const y = hash(i + 31, 83) * height;
        const brightness = hash(i + 79, 13);
        const radius =
            brightness > 0.96 ? 1.15 : brightness > 0.78 ? 0.7 : 0.42;
        const opacity = 0.16 + brightness * 0.48;
        context.beginPath();
        context.fillStyle = `rgba(237, 229, 255, ${opacity})`;
        context.arc(x, y, radius, 0, Math.PI * 2);
        context.fill();
        if (brightness > 0.985) {
            const glow = context.createRadialGradient(x, y, 0, x, y, 5);
            glow.addColorStop(0, "rgba(218, 198, 255, 0.25)");
            glow.addColorStop(1, "rgba(218, 198, 255, 0)");
            context.fillStyle = glow;
            context.fillRect(x - 5, y - 5, 10, 10);
        }
    }
}

if (canvas && context) {
    let resizeFrame = 0;
    // 合并同一帧内的尺寸变化，避免连续重绘背景。
    const resize = () => {
        cancelAnimationFrame(resizeFrame);
        resizeFrame = requestAnimationFrame(drawSky);
    };
    drawSky();
    window.addEventListener("resize", resize, { passive: true });
    window.addEventListener("pagehide", () => {
        cancelAnimationFrame(resizeFrame);
        window.removeEventListener("resize", resize);
    });
    window.addEventListener("pageshow", (event) => {
        if (!event.persisted) return;
        drawSky();
        window.addEventListener("resize", resize, { passive: true });
    });
}
