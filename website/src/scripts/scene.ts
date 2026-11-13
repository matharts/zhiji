import { gsap } from "gsap";
import { chartExample } from "../data/chart-example.ts";
import type { Branch, ChartCell, TransformationKind } from "../data/chart-example.ts";
import type * as Three from "three/src/Three.WebGPU.js";

interface SceneCell { x: number; y: number; stars: string[]; }
interface FlightPath {
    curve: Three.QuadraticBezierCurve3;
    source: Three.Vector3;
    target: Three.Vector3;
    line: Three.InstancedMesh<Three.CylinderGeometry, Three.MeshBasicMaterial>;
    arrow: Three.Mesh<Three.ConeGeometry, Three.MeshBasicMaterial>;
    spark: Three.Mesh<Three.SphereGeometry, Three.MeshBasicMaterial>;
    badge: Three.Sprite | null;
    index: number;
}
interface TimeLink {
    anchor: Three.Vector3;
    curve: Three.QuadraticBezierCurve3;
    line: Three.Line<Three.BufferGeometry, Three.LineBasicMaterial>;
    dot: Three.Mesh<Three.SphereGeometry, Three.MeshBasicMaterial>;
    positions: Float32Array;
    index: number;
}
// Three 的公共 Backend 类型尚未暴露这些 WebGPU 专属字段。
interface WebGpuBackendFields { isWebGPUBackend?: boolean; device?: { lost: Promise<unknown> }; }

/** 用已验证参考样例的三份副本组成空间演示；失败保留静态回退，返回完整清理函数。 */
export async function mountScene(): Promise<() => void> {
    const host = document.querySelector<HTMLElement>("[data-hero-art]");
    const canvas = host?.querySelector<HTMLCanvasElement>("[data-hero-canvas]");
    if (!host || !canvas || !navigator.gpu) return () => {};

    let cleanupRenderer: Three.WebGPURenderer | undefined;
    try {
        // 分别加载核心与渲染器源码，避免聚合入口引入未使用的节点导出。
        const [THREE, { default: WebGPURenderer }] = await Promise.all([
            import("three/src/Three.Core.js"),
            import("three/src/renderers/webgpu/WebGPURenderer.js"),
        ]);
        const renderer = new WebGPURenderer({
            canvas,
            alpha: true,
            antialias: true,
        });
        cleanupRenderer = renderer;
        await renderer.init();
        const backend = renderer.backend as unknown as WebGpuBackendFields;
        if (!backend.isWebGPUBackend) {
            renderer.dispose();
            return () => {};
        }

        const scene = new THREE.Scene();
        const camera = new THREE.PerspectiveCamera(38, 1, 0.1, 80);
        camera.position.set(0, 0, 14);
        const chart = new THREE.Group();
        scene.add(chart);

        const geometries: Three.BufferGeometry[] = [];
        const materials: Three.Material[] = [];
        const textures: Three.Texture[] = [];
        const cells = new Map<Branch, SceneCell>();
        const labels = new Map<Branch, Three.CanvasTexture | null>();
        const box = new THREE.BoxGeometry(1.35, 1.35, 0.12);
        const edges = new THREE.EdgesGeometry(box);
        const labelGeometry = new THREE.PlaneGeometry(1.3, 1.3);
        const perimeterPlane = new THREE.PlaneGeometry(5.73, 5.73);
        const perimeterEdges = new THREE.EdgesGeometry(perimeterPlane);
        geometries.push(
            box,
            edges,
            labelGeometry,
            perimeterPlane,
            perimeterEdges,
        );

        /** 将宫名和星名光栅化为纹理，并记录资源以便统一释放。 */
        function makeLabel(cell: ChartCell): Three.CanvasTexture | null {
            const surface = document.createElement("canvas");
            surface.width = 1024;
            surface.height = 1024;
            const context = surface.getContext("2d");
            if (!context) return null;
            // 布局坐标保持不变，仅将字形光栅化精度加倍。
            context.scale(2, 2);
            context.textBaseline = "top";
            context.fillStyle = "#d3c6df";
            context.font =
                '500 44px "PingFang SC", "Microsoft YaHei", sans-serif';
            context.fillText(cell.palace, 30, 28);
            context.textAlign = "right";
            context.fillText(cell.branch, 482, 28);
            context.textAlign = "left";
            context.fillStyle = "#f1eaf8";
            context.font =
                '600 60px "PingFang SC", "Microsoft YaHei", sans-serif';
            cell.stars.forEach((star, index) => {
                context.fillText(
                    star,
                    30 + (index % 2) * 218,
                    130 + Math.floor(index / 2) * 90,
                );
            });
            const texture = new THREE.CanvasTexture(surface);
            texture.colorSpace = THREE.SRGBColorSpace;
            texture.anisotropy = Math.min(16, renderer.getMaxAnisotropy());
            textures.push(texture);
            return texture;
        }

        chartExample.cells.forEach((cell, index) => {
            if (!cell) return;
            cells.set(cell.branch, {
                x: ((index % 4) - 1.5) * 1.46,
                y: (1.5 - Math.floor(index / 4)) * 1.46,
                stars: cell.stars,
            });
            labels.set(cell.branch, makeLabel(cell));
        });

        const layers = [0, 1, 2].map((index) => {
            const center = index === 1;
            const group = new THREE.Group();
            const plate = new THREE.MeshStandardMaterial({
                color: center ? 0x19151f : index === 0 ? 0x342a42 : 0x21182e,
                roughness: 0.7,
                metalness: 0.18,
                transparent: !center,
                opacity: center ? 1 : 0,
                depthWrite: center,
            });
            const originPlate = new THREE.MeshStandardMaterial({
                color: center ? 0x33253f : 0x6a4b83,
                roughness: 0.64,
                metalness: 0.18,
                transparent: !center,
                opacity: center ? 1 : 0,
                depthWrite: center,
            });
            const edge = new THREE.LineBasicMaterial({
                color: center ? 0x89749d : 0xbda5e5,
                transparent: true,
                opacity: center ? 0.75 : 0,
                depthWrite: false,
            });
            const labelMaterials: Three.MeshBasicMaterial[] = [];
            materials.push(plate, originPlate, edge);
            chartExample.cells.forEach((cell) => {
                if (!cell) return;
                const position = cells.get(cell.branch);
                if (!position) return;
                const groupCell = new THREE.Group();
                groupCell.position.set(position.x, position.y, 0);
                const isOrigin = cell.branch === chartExample.origin;
                const base = new THREE.Mesh(
                    box,
                    isOrigin ? originPlate : plate,
                );
                const outline = new THREE.LineSegments(edges, edge);
                base.renderOrder = index === 0 ? 1 : 0;
                groupCell.add(base, outline);
                const texture = labels.get(cell.branch);
                if (texture) {
                    const material = new THREE.MeshBasicMaterial({
                        map: texture,
                        transparent: true,
                        opacity: center ? 1 : 0,
                        depthWrite: false,
                    });
                    const label = new THREE.Mesh(labelGeometry, material);
                    label.position.z = 0.065;
                    label.renderOrder = 3;
                    groupCell.add(label);
                    labelMaterials.push(material);
                    materials.push(material);
                }
                group.add(groupCell);
            });
            const perimeter = new THREE.LineSegments(perimeterEdges, edge);
            perimeter.position.z = 0.065;
            group.add(perimeter);
            chart.add(group);
            return {
                group,
                plate,
                originPlate,
                edge,
                labelMaterials,
                center,
                index,
            };
        });

        const glowColors = [0xe2d2ff, 0xc2a6ff, 0x9e83e4, 0xefe3ff];
        const segmentCount = 48;
        const segmentGeometry = new THREE.CylinderGeometry(0.012, 0.012, 1, 6);
        const arrowGeometry = new THREE.ConeGeometry(0.055, 0.16, 8);
        const timePointGeometry = new THREE.SphereGeometry(0.03, 10, 8);
        const arrowUp = new THREE.Vector3(0, 1, 0);
        const tangent = new THREE.Vector3();
        const pointA = new THREE.Vector3();
        const pointB = new THREE.Vector3();
        const direction = new THREE.Vector3();
        const dummy = new THREE.Object3D();
        geometries.push(segmentGeometry, arrowGeometry, timePointGeometry);

        /** 将宫位中心投影到内缘，作为缺失星名时的连接回退点。 */
        function edgePoint(cell: SceneCell): Three.Vector3 {
            return new THREE.Vector3(
                THREE.MathUtils.clamp(cell.x, -1.5, 1.5),
                THREE.MathUtils.clamp(cell.y, -1.5, 1.5),
                0.13,
            );
        }

        /** 按标签纹理排版定位星曜，未找到时回退到宫位内缘。 */
        function starPoint(cell: SceneCell, star: string): Three.Vector3 {
            const index = cell.stars.indexOf(star);
            if (index < 0) return edgePoint(cell);
            const px = 30 + (index % 2) * 218;
            const py = 130 + Math.floor(index / 2) * 90;
            const x = (px / 512 - 0.5) * 1.3 + (cell.x < 0 ? 0.45 : -0.08);
            const y = (0.5 - (py + 30) / 512) * 1.3;
            return new THREE.Vector3(cell.x + x, cell.y + y, 0.13);
        }

        /** 生成面向镜头的四化标记，并登记纹理及材质所有权。 */
        function makeBadge(kind: TransformationKind): Three.Sprite | null {
            const surface = document.createElement("canvas");
            surface.width = 128;
            surface.height = 128;
            const context = surface.getContext("2d");
            if (!context) return null;
            context.fillStyle = "#d7c5ff";
            context.beginPath();
            context.arc(64, 64, 57, 0, Math.PI * 2);
            context.fill();
            context.fillStyle = "#21182c";
            context.font =
                '700 70px "PingFang SC", "Microsoft YaHei", sans-serif';
            context.textAlign = "center";
            context.textBaseline = "middle";
            context.fillText(kind, 64, 66);
            const texture = new THREE.CanvasTexture(surface);
            texture.colorSpace = THREE.SRGBColorSpace;
            textures.push(texture);
            const material = new THREE.SpriteMaterial({
                map: texture,
                transparent: true,
                depthTest: false,
                depthWrite: false,
            });
            materials.push(material);
            const badge = new THREE.Sprite(material);
            badge.scale.set(0.35, 0.35, 1);
            badge.renderOrder = 12;
            return badge;
        }

        const paths: FlightPath[] = [];
        const timeLinks: TimeLink[] = [];
        const origin = cells.get(chartExample.origin);
        chartExample.transformations.forEach((item, index) => {
            const destination = cells.get(item.branch);
            if (!origin || !destination) return;
            const source = edgePoint(origin);
            const target = starPoint(destination, item.star);
            const curve = new THREE.QuadraticBezierCurve3(
                source.clone(),
                new THREE.Vector3(),
                target.clone(),
            );
            const lineMaterial = new THREE.MeshBasicMaterial({
                color: glowColors[index],
                depthTest: false,
            });
            const line = new THREE.InstancedMesh(
                segmentGeometry,
                lineMaterial,
                segmentCount,
            );
            line.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
            line.visible = false;
            line.renderOrder = 10;
            const arrowMaterial = new THREE.MeshBasicMaterial({
                color: glowColors[index],
                depthTest: false,
            });
            const arrow = new THREE.Mesh(arrowGeometry, arrowMaterial);
            arrow.visible = false;
            arrow.renderOrder = 11;
            const sparkMaterial = new THREE.MeshBasicMaterial({
                color: glowColors[index],
                transparent: true,
                opacity: 0,
                depthTest: false,
                depthWrite: false,
            });
            const spark = new THREE.Mesh(timePointGeometry, sparkMaterial);
            spark.scale.setScalar(2.25);
            spark.visible = false;
            spark.renderOrder = 12;
            const badge = makeBadge(item.kind);
            if (badge) {
                badge.visible = false;
                chart.add(badge);
            }
            chart.add(line, arrow, spark);
            materials.push(lineMaterial, arrowMaterial, sparkMaterial);
            paths.push({
                curve,
                source,
                target,
                line,
                arrow,
                spark,
                badge,
                index,
            });

            const timeGeometry = new THREE.BufferGeometry();
            const timePositions = new Float32Array(25 * 3);
            const timeAttribute = new THREE.BufferAttribute(timePositions, 3);
            timeAttribute.setUsage(THREE.DynamicDrawUsage);
            timeGeometry.setAttribute("position", timeAttribute);
            const timeMaterial = new THREE.LineBasicMaterial({
                color: glowColors[index],
                transparent: true,
                opacity: 0,
                depthTest: false,
            });
            const timeLine = new THREE.Line(timeGeometry, timeMaterial);
            timeLine.frustumCulled = false;
            const timeDotMaterial = new THREE.MeshBasicMaterial({
                color: glowColors[index],
                transparent: true,
                opacity: 0,
                depthTest: false,
            });
            const timeDot = new THREE.Mesh(timePointGeometry, timeDotMaterial);
            timeLine.visible = false;
            timeDot.visible = false;
            timeLine.renderOrder = 7;
            timeDot.renderOrder = 8;
            chart.add(timeLine, timeDot);
            geometries.push(timeGeometry);
            materials.push(timeMaterial, timeDotMaterial);
            timeLinks.push({
                anchor: target.clone(),
                curve: new THREE.QuadraticBezierCurve3(),
                line: timeLine,
                dot: timeDot,
                positions: timePositions,
                index,
            });
        });

        scene.add(new THREE.AmbientLight(0xd8c5eb, 1.35));
        const key = new THREE.DirectionalLight(0xf3e7ff, 2);
        key.position.set(-3, 4, 7);
        scene.add(key);
        const violet = new THREE.DirectionalLight(0x9165c4, 1.4);
        violet.position.set(4, -2, 5);
        scene.add(violet);

        const motion = { top: 0, middle: 0, bottom: 0, twist: 0, flight: 0 };
        const choreography = gsap
            .timeline({ paused: true, repeat: -1 })
            .set(
                motion,
                { top: 0, middle: 0, bottom: 0, twist: 0, flight: 0 },
                0,
            )
            .to(motion, { top: 1, duration: 1.8, ease: "power2.inOut" }, 0.5)
            .to(motion, { middle: 1, duration: 1.8, ease: "power2.inOut" }, 1.2)
            .to(motion, { bottom: 1, duration: 1.8, ease: "power2.inOut" }, 1.9)
            .to(motion, { twist: 1, duration: 4.5, ease: "sine.inOut" }, 3)
            .to(motion, { flight: 4, duration: 3.7, ease: "none" }, 3.1)
            .to(
                motion,
                {
                    top: 0,
                    middle: 0,
                    bottom: 0,
                    duration: 2.8,
                    ease: "power2.inOut",
                },
                7.8,
            )
            .to(motion, { flight: 4, duration: 1.8 }, 10.6);

        let frame = 0;
        let visible = true;
        let disposed = false;
        let pausedByUser = false;
        let pointerX = 0;
        let pointerY = 0;
        let baseScale = 1;
        let baseY = -0.05;
        let shown = false;
        const motionToggle = document.querySelector<HTMLButtonElement>("[data-motion-toggle]");
        const motionLabel = motionToggle?.querySelector<HTMLElement>("[data-motion-label]");
        const motionIcon = motionToggle?.querySelector<HTMLElement>("[data-motion-icon]");

        // 重算渲染分辨率、镜头和场景适配，避免窄视口裁切命盘主体。
        const resize = () => {
            const width = Math.max(1, canvas.clientWidth);
            const height = Math.max(1, canvas.clientHeight);
            // 适度超采样提高透视过滤后的细小字形清晰度。
            const nativeRatio = window.devicePixelRatio || 1;
            renderer.setPixelRatio(Math.min(nativeRatio * 1.25, width > 900 ? 3 : 2));
            renderer.setSize(width, height, false);
            camera.aspect = width / height;
            camera.updateProjectionMatrix();
            const visibleHeight =
                2 *
                Math.tan(THREE.MathUtils.degToRad(camera.fov / 2)) *
                camera.position.z;
            const visibleWidth = visibleHeight * camera.aspect;
            if (width <= 560) {
                baseY = -0.3;
                chart.position.set(0, baseY, 0);
                baseScale = Math.min(0.8, (visibleWidth * 0.88) / 7.5);
            } else if (camera.aspect < 1.3) {
                baseY = -0.05;
                chart.position.set(visibleWidth * 0.1, baseY, 0);
                baseScale = Math.min(0.9, (visibleWidth * 0.75) / 7.5);
            } else {
                baseY = -0.05;
                chart.position.set(visibleWidth * 0.2, baseY, 0);
                baseScale = Math.min(1.1, (visibleWidth * 0.53) / 7.5);
            }
        };

        // 只采集鼠标的归一化视差，触摸和手写笔不驱动持续旋转。
        const onPointer = (event: PointerEvent) => {
            if (event.pointerType !== "mouse") return;
            pointerX = event.clientX / window.innerWidth - 0.5;
            pointerY = event.clientY / window.innerHeight - 0.5;
        };

        // 可见时依据时间轴更新几何并绘制；页面隐藏或资源释放后停止申请帧。
        const render = () => {
            if (disposed || !visible || document.hidden) return;
            const spread = Math.max(motion.top, motion.middle, motion.bottom);
            const linkStrength = Math.min(
                motion.top,
                motion.middle,
                motion.bottom,
            );
            const time = choreography.time();
            chart.scale.setScalar(baseScale * (1 - 0.3 * spread));
            chart.position.y = baseY + 0.28 * spread;
            chart.rotation.y =
                spread *
                (Math.sin(motion.twist * Math.PI) * 0.3 + pointerX * 0.06);
            chart.rotation.z = spread * Math.sin(motion.twist * Math.PI) * 0.04;

            layers.forEach(
                ({
                    group,
                    plate,
                    originPlate,
                    edge,
                    labelMaterials,
                    center,
                    index,
                }) => {
                    const layerSpread =
                        index === 0
                            ? motion.top
                            : index === 1
                              ? motion.middle
                              : motion.bottom;
                    const sideVisibility = THREE.MathUtils.smoothstep(
                        layerSpread,
                        0.04,
                        0.36,
                    );
                    const spinDirection =
                        index === 0 ? 1 : index === 1 ? -0.45 : -1;
                    const orbit =
                        motion.twist * Math.PI * 1.6 +
                        (index * Math.PI * 2) / 3;
                    group.position.x =
                        Math.sin(orbit) * 0.4 * layerSpread * motion.twist;
                    group.position.y = (1 - index) * 2.65 * layerSpread;
                    group.position.z =
                        Math.cos(orbit) * 0.4 * layerSpread * motion.twist;
                    group.rotation.x = (-1.05 + pointerY * 0.05) * layerSpread;
                    group.rotation.y = Math.sin(orbit) * 0.12 * layerSpread;
                    group.rotation.z =
                        layerSpread *
                        ((index - 1) * 0.12 +
                            spinDirection * motion.twist * 1.25);
                    group.updateMatrix();
                    if (!center) {
                        group.visible = sideVisibility > 0.001;
                        plate.opacity = 0.72 * sideVisibility;
                        originPlate.opacity = 0.8 * sideVisibility;
                        edge.opacity = 0.8 * sideVisibility;
                        labelMaterials.forEach((material) => {
                            material.opacity = 0.96 * sideVisibility;
                        });
                    }
                },
            );

            const middle = layers[1].group;
            timeLinks.forEach(
                ({ anchor, curve, line, dot, positions, index }) => {
                    line.visible = linkStrength > 0.02;
                    dot.visible = linkStrength > 0.02;
                    if (!line.visible) return;
                    curve.v0.copy(anchor).applyMatrix4(layers[0].group.matrix);
                    curve.v1.copy(anchor).applyMatrix4(middle.matrix);
                    curve.v2.copy(anchor).applyMatrix4(layers[2].group.matrix);
                    curve.v1
                        .multiplyScalar(2)
                        .addScaledVector(curve.v0, -0.5)
                        .addScaledVector(curve.v2, -0.5);
                    for (let i = 0; i <= 24; i += 1) {
                        curve.getPoint(i / 24, pointA);
                        positions[i * 3] = pointA.x;
                        positions[i * 3 + 1] = pointA.y;
                        positions[i * 3 + 2] = pointA.z;
                    }
                    line.geometry.attributes.position.needsUpdate = true;
                    line.material.opacity = 0.28 * linkStrength;
                    dot.material.opacity = 0.75 * linkStrength;
                    curve.getPoint(
                        (time * 0.18 + index * 0.25) % 1,
                        dot.position,
                    );
                },
            );
            paths.forEach(
                ({
                    curve,
                    source,
                    target,
                    line,
                    arrow,
                    spark,
                    badge,
                    index,
                }) => {
                    curve.v0.copy(source).applyMatrix4(middle.matrix);
                    // 展开时各四化副本抵达外层，收拢为平面命盘时三份副本重合。
                    curve.v2
                        .copy(target)
                        .applyMatrix4(
                            layers[index % 2 === 0 ? 0 : 2].group.matrix,
                        );
                    curve.v1.copy(curve.v0).lerp(curve.v2, 0.5);
                    curve.v1.x += (index - 1.5) * 0.12;
                    curve.v1.y += (index % 2 ? 1 : -1) * 1.6 * spread;
                    curve.v1.z += (index < 2 ? 1.6 : -1.6) * spread;

                    const progress = THREE.MathUtils.clamp(
                        motion.flight - index,
                        0,
                        1,
                    );
                    line.visible = progress > 0;
                    arrow.visible = progress > 0;
                    if (line.visible) {
                        const count = Math.max(
                            1,
                            Math.floor(progress * segmentCount),
                        );
                        for (let i = 0; i < count; i += 1) {
                            curve.getPoint(i / segmentCount, pointA);
                            curve.getPoint((i + 1) / segmentCount, pointB);
                            direction.copy(pointB).sub(pointA);
                            const length = direction.length();
                            dummy.position
                                .copy(pointA)
                                .add(pointB)
                                .multiplyScalar(0.5);
                            dummy.quaternion.setFromUnitVectors(
                                arrowUp,
                                direction.normalize(),
                            );
                            dummy.scale.set(1, length, 1);
                            dummy.updateMatrix();
                            line.setMatrixAt(i, dummy.matrix);
                        }
                        line.count = count;
                        line.instanceMatrix.needsUpdate = true;
                        curve.getPoint(progress, arrow.position);
                        curve
                            .getTangent(Math.min(progress, 0.999), tangent)
                            .normalize();
                        arrow.quaternion.setFromUnitVectors(arrowUp, tangent);
                    }
                    if (badge) {
                        badge.visible = progress > 0.62;
                        if (badge.visible) curve.getPoint(0.58, badge.position);
                    }
                    spark.visible = progress > 0.02 && spread > 0.05;
                    if (spark.visible) {
                        spark.material.opacity = Math.min(1, spread * 1.8);
                        curve.getPoint(
                            progress * ((time * 0.45 + index * 0.27) % 1),
                            spark.position,
                        );
                    }
                },
            );

            renderer.render(scene, camera);
            if (!shown) {
                host.classList.add("is-ready");
                if (motionToggle) motionToggle.hidden = false;
                shown = true;
            }
            frame = requestAnimationFrame(render);
        };

        // 汇总视口、页面和用户暂停状态，保证只有一条动画帧循环。
        const restart = () => {
            cancelAnimationFrame(frame);
            if (!disposed && visible && !document.hidden && !pausedByUser) {
                choreography.resume();
                frame = requestAnimationFrame(render);
            } else {
                choreography.pause();
            }
        };
        // 用户暂停独立于页面可见性，恢复时仍遵守其余运行条件。
        const toggleMotion = () => {
            pausedByUser = !pausedByUser;
            if (motionLabel) motionLabel.textContent = pausedByUser ? "播放动画" : "暂停动画";
            if (motionIcon) motionIcon.dataset.state = pausedByUser ? "paused" : "playing";
            restart();
        };
        // 暂停场景在尺寸变化时只重绘一帧，不意外恢复循环。
        const onResize = () => {
            resize();
            if (pausedByUser && visible && !document.hidden) {
                render();
                cancelAnimationFrame(frame);
            }
        };
        let densityQuery = window.matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`);
        // 屏幕像素密度变化后重建媒体查询并更新渲染分辨率。
        const onDensityChange = () => {
            densityQuery.removeEventListener("change", onDensityChange);
            onResize();
            densityQuery = window.matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`);
            densityQuery.addEventListener("change", onDensityChange);
        };
        densityQuery.addEventListener("change", onDensityChange);
        const observer = new IntersectionObserver(([entry]) => {
            visible = entry.isIntersecting;
            restart();
        });
        observer.observe(host);
        window.addEventListener("resize", onResize, { passive: true });
        window.addEventListener("pointermove", onPointer, { passive: true });
        document.addEventListener("visibilitychange", restart);
        motionToggle?.addEventListener("click", toggleMotion);

        // 幂等撤销时间轴、监听器和全部 GPU 资源，设备丢失也复用该入口。
        const dispose = () => {
            if (disposed) return;
            disposed = true;
            cancelAnimationFrame(frame);
            choreography.kill();
            observer.disconnect();
            densityQuery.removeEventListener("change", onDensityChange);
            window.removeEventListener("resize", onResize);
            window.removeEventListener("pointermove", onPointer);
            document.removeEventListener("visibilitychange", restart);
            motionToggle?.removeEventListener("click", toggleMotion);
            if (motionToggle) {
                motionToggle.hidden = true;
                if (motionLabel) motionLabel.textContent = "暂停动画";
                if (motionIcon) motionIcon.dataset.state = "playing";
            }
            host.classList.remove("is-ready");
            geometries.forEach((geometry) => geometry.dispose());
            materials.forEach((material) => material.dispose());
            textures.forEach((texture) => texture.dispose());
            renderer.dispose();
        };
        backend.device?.lost.then(dispose);
        resize();
        restart();
        return dispose;
    } catch (error) {
        cleanupRenderer?.dispose();
        host.classList.remove("is-ready");
        console.warn(
            "WebGPU background unavailable; using the static chart example.",
            error,
        );
        return () => {};
    }
}
