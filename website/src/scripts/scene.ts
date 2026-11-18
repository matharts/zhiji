import { gsap } from "gsap";
import { PRELUDE_DURATION, RETURN_DURATION, FLIGHT_DURATION, preludeFrame } from "./scene-choreography.ts";
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
    trail: Three.InstancedMesh<Three.SphereGeometry, Three.MeshBasicMaterial>;
    halo: Three.Sprite | null;
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

    host.classList.add("is-loading");
    let cleanupRenderer: Three.WebGPURenderer | undefined;
    let cleanupResources = () => {};
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
            host.classList.remove("is-loading");
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
        cleanupResources = () => {
            geometries.forEach(geometry => geometry.dispose());
            materials.forEach(material => material.dispose());
            textures.forEach(texture => texture.dispose());
        };
        const cells = new Map<Branch, SceneCell>();
        const labels = new Map<Branch, Three.CanvasTexture | null>();
        const box = new THREE.BoxGeometry(1.35, 1.35, 0.12);
        const edges = new THREE.EdgesGeometry(box);
        const labelGeometry = new THREE.PlaneGeometry(1.3, 1.3);
        const layerNameGeometry = new THREE.PlaneGeometry(1.45, 0.72);
        const perimeterPlane = new THREE.PlaneGeometry(5.73, 5.73);
        const perimeterEdges = new THREE.EdgesGeometry(perimeterPlane);
        geometries.push(
            box,
            edges,
            labelGeometry,
            layerNameGeometry,
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

        /** 使用独立纹理标识三层；文字仅用于示意，不参与排盘计算。 */
        function makeLayerName(name: string): Three.CanvasTexture | null {
            const surface = document.createElement("canvas");
            surface.width = 1024;
            surface.height = 512;
            const context = surface.getContext("2d");
            if (!context) return null;
            context.strokeStyle = "rgba(199, 174, 236, 0.55)";
            context.lineWidth = 2;
            context.beginPath();
            context.arc(512, 256, 160, 0, Math.PI * 2);
            context.stroke();
            context.fillStyle = "#eee2ff";
            context.font = '500 84px "PingFang SC", "Microsoft YaHei", sans-serif';
            context.textAlign = "center";
            context.textBaseline = "middle";
            context.fillText(name, 512, 258);
            const texture = new THREE.CanvasTexture(surface);
            texture.colorSpace = THREE.SRGBColorSpace;
            texture.anisotropy = Math.min(16, renderer.getMaxAnisotropy());
            textures.push(texture);
            return texture;
        }

        const layers = [0, 1, 2].map((index) => {
            const center = index === 1;
            const group = new THREE.Group();
            const plate = new THREE.MeshStandardMaterial({
                color: center ? 0x19151f : index === 0 ? 0x342a42 : 0x21182e,
                roughness: 0.7,
                metalness: 0.18,
                transparent: true,
                opacity: center ? 1 : 0,
                depthWrite: center,
            });
            const originPlate = new THREE.MeshStandardMaterial({
                color: center ? 0x33253f : 0x6a4b83,
                roughness: 0.64,
                metalness: 0.18,
                transparent: true,
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
            const nameTexture = makeLayerName(["天盘", "地盘", "人盘"][index]!);
            let nameMaterial: Three.MeshBasicMaterial | undefined;
            if (nameTexture) {
                nameMaterial = new THREE.MeshBasicMaterial({
                    map: nameTexture,
                    transparent: true,
                    opacity: 0,
                    depthWrite: false,
                });
                const name = new THREE.Mesh(layerNameGeometry, nameMaterial);
                name.position.z = 0.075;
                name.renderOrder = 3;
                group.add(name);
                materials.push(nameMaterial);
            }
            chart.add(group);
            return {
                group,
                plate,
                originPlate,
                edge,
                labelMaterials,
                nameMaterial,
                center,
                index,
            };
        });

        // 两条低亮度轨道为三盘提供深度参照，避免与飞化路径争抢视线。
        const orbitGeometry = new THREE.BufferGeometry().setFromPoints(
            new THREE.EllipseCurve(0, 0, 4.6, 3.2, 0, Math.PI * 2).getPoints(128)
                .map((point) => new THREE.Vector3(point.x, point.y, 0)),
        );
        const orbitMaterial = new THREE.LineBasicMaterial({
            color: 0x9978c9,
            transparent: true,
            opacity: 0.17,
            depthWrite: false,
        });
        const orbitGroup = new THREE.Group();
        const firstOrbit = new THREE.Line(orbitGeometry, orbitMaterial);
        const secondOrbit = new THREE.Line(orbitGeometry, orbitMaterial);
        firstOrbit.rotation.x = 0.95;
        secondOrbit.rotation.x = -0.95;
        secondOrbit.rotation.z = Math.PI / 3;
        orbitGroup.add(firstOrbit, secondOrbit);
        chart.add(orbitGroup);
        geometries.push(orbitGeometry);
        materials.push(orbitMaterial);

        const glowColors = [0xe2d2ff, 0xc2a6ff, 0x9e83e4, 0xefe3ff];
        const segmentCount = 48;
        const segmentGeometry = new THREE.CylinderGeometry(0.012, 0.012, 1, 6);
        const arrowGeometry = new THREE.ConeGeometry(0.055, 0.16, 8);
        const timePointGeometry = new THREE.SphereGeometry(0.03, 10, 8);
        const trailLength = 4;
        const arrowUp = new THREE.Vector3(0, 1, 0);
        const tangent = new THREE.Vector3();
        const pointA = new THREE.Vector3();
        const pointB = new THREE.Vector3();
        const direction = new THREE.Vector3();
        const dummy = new THREE.Object3D();
        geometries.push(segmentGeometry, arrowGeometry, timePointGeometry);

        /** 共用柔光纹理；运动光点只在飞化经过时点亮。 */
        function makeGlowTexture(): Three.CanvasTexture | null {
            const surface = document.createElement("canvas");
            surface.width = 128;
            surface.height = 128;
            const context = surface.getContext("2d");
            if (!context) return null;
            const gradient = context.createRadialGradient(64, 64, 0, 64, 64, 64);
            gradient.addColorStop(0, "rgba(255,255,255,0.95)");
            gradient.addColorStop(0.16, "rgba(222,204,255,0.6)");
            gradient.addColorStop(1, "rgba(174,133,255,0)");
            context.fillStyle = gradient;
            context.fillRect(0, 0, 128, 128);
            const texture = new THREE.CanvasTexture(surface);
            texture.colorSpace = THREE.SRGBColorSpace;
            textures.push(texture);
            return texture;
        }
        const glowTexture = makeGlowTexture();
        const { createPrelude } = await import("./scene-prelude.ts");
        const prelude = createPrelude(chart, { geometries, materials, textures }, glowTexture);
        const layerMatrices = layers.map(layer => layer.group.matrix);

        // 轨道上只放两颗缓慢移动的光点，为旋转提供空间参照。
        const orbitMarkers = [firstOrbit, secondOrbit].map((orbit, index) => {
            const material = new THREE.MeshBasicMaterial({
                color: index === 0 ? 0xe0d1ff : 0xb59be9,
                transparent: true,
                opacity: 0,
                depthWrite: false,
            });
            const marker = new THREE.Mesh(timePointGeometry, material);
            marker.scale.setScalar(1.4);
            orbit.add(marker);
            materials.push(material);
            return marker;
        });

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
                transparent: true,
                depthWrite: false,
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
                transparent: true,
                depthWrite: false,
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
            const trailMaterial = new THREE.MeshBasicMaterial({
                color: glowColors[index],
                transparent: true,
                opacity: 0,
                depthTest: false,
                depthWrite: false,
            });
            const trail = new THREE.InstancedMesh(timePointGeometry, trailMaterial, trailLength);
            trail.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
            trail.frustumCulled = false;
            trail.visible = false;
            trail.renderOrder = 11;
            let halo: Three.Sprite | null = null;
            if (glowTexture) {
                const haloMaterial = new THREE.SpriteMaterial({
                    map: glowTexture,
                    color: glowColors[index],
                    transparent: true,
                    opacity: 0,
                    depthTest: false,
                    depthWrite: false,
                    blending: THREE.AdditiveBlending,
                });
                halo = new THREE.Sprite(haloMaterial);
                halo.scale.set(0.28, 0.28, 1);
                halo.renderOrder = 12;
                spark.add(halo);
                materials.push(haloMaterial);
            }
            const badge = makeBadge(item.kind);
            if (badge) {
                badge.visible = false;
                chart.add(badge);
            }
            chart.add(line, arrow, spark, trail);
            materials.push(lineMaterial, arrowMaterial, sparkMaterial, trailMaterial);
            paths.push({
                curve,
                source,
                target,
                line,
                arrow,
                spark,
                trail,
                halo,
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
        const motionLoop = gsap
            .timeline()
            .set(
                motion,
                { top: 0, middle: 0, bottom: 0, twist: 0, flight: 0 },
                0,
            )
            // 分盘几乎同时启动，以短促加速和柔和减速替代逐层等待。
            .to(motion, { top: 1, duration: 1.25, ease: "power2.inOut" }, 0.18)
            .to(motion, { middle: 1, duration: 1.25, ease: "power2.inOut" }, 0.24)
            .to(motion, { bottom: 1, duration: 1.25, ease: "power2.inOut" }, 0.30)
            .to(motion, { twist: 1, duration: 8.2, ease: "sine.inOut" }, 0.3)
            .to(motion, { flight: 4, duration: 5.2, ease: "none" }, 1.45)
            .to(
                motion,
                {
                    top: 0,
                    middle: 0,
                    bottom: 0,
                    duration: 2.15,
                    ease: "sine.inOut",
                },
                8.8,
            )
            .to(motion, { flight: 0, duration: 0.8, ease: "sine.inOut" }, 10.5)
            // 合盘后保留短暂平面停留，不再反向倒转整段旋转。
            .to({}, { duration: FLIGHT_DURATION - 11.3 }, 11.3);

        // 原飞化只运行一遍；主时间轴将混沌、阴阳化盘、飞化和回归混沌一起循环。
        const choreography = gsap.timeline({ paused: true, repeat: -1 })
            .to({ value: 0 }, { value: 1, duration: PRELUDE_DURATION, ease: "none" }, 0)
            .add(motionLoop, PRELUDE_DURATION)
            .to({ value: 0 }, { value: 1, duration: RETURN_DURATION, ease: "none" });

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
        const replayButton = document.querySelector<HTMLButtonElement>("[data-scene-replay]");
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
                14;
            const visibleWidth = visibleHeight * camera.aspect;
            if (host.closest("[data-centered-scene]")) {
                chart.position.set(0, 0, 0);
                baseY = 0;
                baseScale = Math.min(0.92, (visibleWidth * 0.84) / 7.5);
            } else if (width <= 560) {
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
            const introTime = choreography.time();
            const intro = preludeFrame(introTime, motionLoop.duration());
            if (host.dataset.scenePhase !== intro.phase) host.dataset.scenePhase = intro.phase;
            const spread = intro.active ? intro.spread : Math.max(motion.top, motion.middle, motion.bottom);
            const twist = intro.active ? intro.twist : motion.twist;
            const reveal = intro.chart;
            camera.position.z = intro.cameraZ;
            key.intensity = 2 + intro.pulse * 1.8;
            const linkStrength = Math.min(
                motion.top,
                motion.middle,
                motion.bottom,
            );
            const time = motionLoop.time();
            // 只在段落变化时记录运动阶段，便于检查分盘、交错与合盘的实际画面。
            const motionPhase = intro.active ? 'prelude' : time < 1.55 ? 'split'
                : time < 4.4 ? 'orbit' : time < 8.8 ? 'crossing' : time < 10.95 ? 'collapse' : 'flat';
            if (host.dataset.sceneMotion !== motionPhase) host.dataset.sceneMotion = motionPhase;
            chart.scale.setScalar(baseScale * (1 - 0.25 * spread));
            chart.position.y = baseY;
            chart.rotation.y =
                spread *
                (Math.sin(twist * Math.PI * 2) * 0.3 + pointerX * 0.04);
            chart.rotation.x = Math.sin(twist * Math.PI * 2) * 0.1 * spread;
            chart.rotation.z = -0.08 * spread;
            orbitGroup.rotation.z = twist * 1.2;
            orbitMaterial.opacity = 0.13 * spread * reveal;
            orbitMarkers.forEach((marker, index) => {
                const angle = time * 0.18 + index * Math.PI;
                marker.position.set(4.6 * Math.cos(angle), 3.2 * Math.sin(angle), 0);
                marker.material.opacity = 0.55 * spread * reveal;
            });

            layers.forEach(
                ({
                    group,
                    plate,
                    originPlate,
                    edge,
                    labelMaterials,
                    nameMaterial,
                    center,
                    index,
                }) => {
                    const layerSpread = intro.active ? intro.spread :
                        index === 0
                            ? motion.top
                            : index === 1
                              ? motion.middle
                              : motion.bottom;
                    const sideVisibility = THREE.MathUtils.smoothstep(
                        layerSpread,
                        0.015,
                        0.2,
                    );
                    const spinDirection = index === 0 ? 1 : index === 1 ? -0.4 : -1;
                    const travel = spinDirection * Math.PI * 1.1;
                    const angle = travel * twist;
                    // 上下盘反向转动并向前后错位，中盘缓转，保持纵向三层可辨。
                    group.position.x = ((index - 1) * 0.13 + Math.sin(angle) * 0.3) * layerSpread;
                    group.position.y = (1 - index) * 2.85 * layerSpread;
                    group.position.z = ((index - 1) * 0.85 + Math.sin(angle) * 0.3) * layerSpread;
                    group.rotation.x = (-0.98 + Math.sin(angle + index) * 0.12 + pointerY * 0.035) * layerSpread;
                    group.rotation.y = Math.sin(angle * 0.8) * 0.38 * layerSpread;
                    // 合拢时接向最近的整周，避免把已完成的大角度转动倒放回零。
                    const aligned = Math.round(travel / (Math.PI * 2)) * Math.PI * 2;
                    group.rotation.z = angle + (aligned - travel) * twist * (1 - layerSpread)
                        + (index - 1) * 0.06 * layerSpread;
                    if (nameMaterial) nameMaterial.opacity = 0.85 * layerSpread * reveal;
                    group.updateMatrix();
                    group.visible = reveal > 0.001 && (center || sideVisibility > 0.001);
                    plate.opacity = (center ? 1 : 0.56 * sideVisibility) * reveal;
                    originPlate.opacity = (center ? 1 : 0.65 * sideVisibility) * reveal;
                    edge.opacity = (center ? 0.75 : 0.65 * sideVisibility) * reveal;
                    labelMaterials.forEach((material) => {
                        material.opacity = (center ? 1 : 0.8 * sideVisibility) * reveal;
                    });
                },
            );

            prelude.update(introTime, layerMatrices, motionLoop.duration());
            const middle = layers[1].group;
            timeLinks.forEach(
                ({ anchor, curve, line, dot, positions, index }) => {
                    line.visible = !intro.active && linkStrength > 0.02;
                    dot.visible = !intro.active && linkStrength > 0.02;
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
                    line.material.opacity = 0.22 * linkStrength;
                    dot.material.opacity = 0.9 * linkStrength;
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
                    trail,
                    halo,
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

                    const phase = (intro.active ? 0 : motion.flight) - index;
                    const progress = THREE.MathUtils.clamp(phase, 0, 1);
                    // 当前飞化最亮；已完成的路径保留为低亮度背景。
                    const pathOpacity = THREE.MathUtils.clamp(1 - Math.max(0, phase - 1) * 0.26, 0.32, 1);
                    line.material.opacity = pathOpacity;
                    arrow.material.opacity = pathOpacity;
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
                        badge.visible = progress > 0.7;
                        badge.material.opacity = pathOpacity * THREE.MathUtils.smoothstep(progress, 0.7, 0.95);
                        if (badge.visible) curve.getPoint(0.58, badge.position);
                    }
                    spark.visible = progress > 0.02 && progress < 0.99 && spread > 0.05;
                    trail.visible = spark.visible;
                    if (spark.visible) {
                        const pulse = Math.sin(progress * Math.PI) * Math.min(1, spread * 2);
                        spark.material.opacity = pulse;
                        trail.material.opacity = pulse * 0.65;
                        if (halo) halo.material.opacity = pulse * 0.8;
                        curve.getPoint(progress, spark.position);
                        for (let trailIndex = 0; trailIndex < trailLength; trailIndex += 1) {
                            const trailProgress = Math.max(0, progress - (trailIndex + 1) * 0.025);
                            curve.getPoint(trailProgress, dummy.position);
                            dummy.quaternion.identity();
                            dummy.scale.setScalar(1.5 - trailIndex * 0.3);
                            dummy.updateMatrix();
                            trail.setMatrixAt(trailIndex, dummy.matrix);
                        }
                        trail.instanceMatrix.needsUpdate = true;
                    }
                },
            );

            renderer.render(scene, camera);
            if (!shown) {
                host.classList.add("is-ready");
                host.classList.remove("is-loading");
                if (motionToggle) motionToggle.hidden = false;
                if (replayButton) replayButton.hidden = false;
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
        // 显式重播会从混沌重新起步；离屏时仍由统一运行条件保持暂停。
        const replayScene = () => {
            pausedByUser = false;
            if (motionLabel) motionLabel.textContent = "暂停动画";
            if (motionIcon) motionIcon.dataset.state = "playing";
            choreography.restart();
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
        replayButton?.addEventListener("click", replayScene);

        // 幂等撤销时间轴、监听器和全部 GPU 资源，设备丢失也复用该入口。
        const dispose = () => {
            if (disposed) return;
            disposed = true;
            cancelAnimationFrame(frame);
            choreography.kill();
            motionLoop.kill();
            observer.disconnect();
            densityQuery.removeEventListener("change", onDensityChange);
            window.removeEventListener("resize", onResize);
            window.removeEventListener("pointermove", onPointer);
            document.removeEventListener("visibilitychange", restart);
            motionToggle?.removeEventListener("click", toggleMotion);
            replayButton?.removeEventListener("click", replayScene);
            if (replayButton) replayButton.hidden = true;
            delete host.dataset.scenePhase;
            delete host.dataset.sceneMotion;
            if (motionToggle) {
                motionToggle.hidden = true;
                if (motionLabel) motionLabel.textContent = "暂停动画";
                if (motionIcon) motionIcon.dataset.state = "playing";
            }
            host.classList.remove("is-ready", "is-loading");
            cleanupResources();
            renderer.dispose();
        };
        backend.device?.lost.then(dispose);
        resize();
        // 提前编译节点材质，避免开场首次显现阴阳或粒子时临时建立渲染管线。
        await renderer.compileAsync(scene, camera);
        restart();
        return dispose;
    } catch (error) {
        cleanupResources();
        cleanupRenderer?.dispose();
        host.classList.remove("is-ready", "is-loading");
        console.warn(
            "WebGPU background unavailable; using the static chart example.",
            error,
        );
        return () => {};
    }
}
