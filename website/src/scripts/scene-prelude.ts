import * as THREE from 'three/src/Three.Core.js';
import MeshBasicNodeMaterial from 'three/src/materials/nodes/MeshBasicNodeMaterial.js';
import { uv, uniform, vec2, vec3, sin, cos, mix, smoothstep, mx_fractal_noise_float } from 'three/src/Three.TSL.js';
import { chartExample } from '../data/chart-example.ts';
import { preludeFrame } from './scene-choreography.ts';

interface Resources {
    geometries: THREE.BufferGeometry[];
    materials: THREE.Material[];
    textures: THREE.Texture[];
}

/** 使用流动雾场与形变粒子表现混沌和阴阳，不创建实体太极圆盘。 */
export function createPrelude(parent: THREE.Group, resources: Resources, glow: THREE.Texture | null) {
    const root = new THREE.Group();
    parent.add(root);
    const clock = uniform(0);
    const cloudVisibility = uniform(1);
    const cohesion = uniform(0);
    const cloudHalfSize = 4.8;
    // 雾场、粒子和化盘轮廓共享太极半径，凝形后的占幅接近混沌旋流。
    const symbolRadius = 3.1;
    const symbolEyeRatio = .11;
    const cloudGeometry = new THREE.PlaneGeometry(cloudHalfSize * 2, cloudHalfSize * 2);
    const cloudMaterial = new MeshBasicNodeMaterial({transparent:true,depthWrite:false,depthTest:false});
    const p = uv().sub(.5).mul(2);
    const radius = p.length();
    const swirl = clock.mul(.13).add(radius.mul(2.2));
    const rotated = vec2(p.x.mul(cos(swirl)).sub(p.y.mul(sin(swirl))),p.x.mul(sin(swirl)).add(p.y.mul(cos(swirl))));
    const turbulence = mx_fractal_noise_float(vec3(rotated.mul(2.8),clock.mul(.11)),4,2.1,.52);
    const filaments = sin(turbulence.mul(5).add(radius.mul(8)).sub(clock.mul(.5))).abs().oneMinus().pow(3);
    const envelope = smoothstep(.12,.98,radius).oneMinus();
    const density = turbulence.add(.55).clamp(0,1).mul(envelope);
    // 阴阳边界始终保留湍流和软边，只显现两股云气，不形成实心表面。
    const q = p.div(symbolRadius / cloudHalfSize);
    const softness = mix(.3,.075,cohesion);
    const upper = q.sub(vec2(0,.5)).length();
    const lower = q.sub(vec2(0,-.5)).length();
    const boundary = q.x.max(upper.mul(-1).add(.5)).min(lower.sub(.5));
    const disruption = turbulence.mul(cohesion.oneMinus().pow(2).mul(.5).add(.14));
    let lightRegion = smoothstep(softness.negate(),softness,boundary.add(disruption));
    const eyeRadius = smoothstep(.6,1,cohesion).mul(symbolEyeRatio);
    const upperEye = smoothstep(eyeRadius.sub(softness),eyeRadius.add(softness),upper).oneMinus();
    const lowerEye = smoothstep(eyeRadius.sub(softness),eyeRadius.add(softness),lower).oneMinus();
    lightRegion = lightRegion.mul(upperEye.oneMinus()).add(lowerEye).clamp(0,1);
    const outerRadius = mix(1.9,1,cohesion);
    const body = smoothstep(outerRadius.sub(softness),outerRadius.add(softness),q.length().add(disruption)).oneMinus();
    const mistColor = mix(vec3(.06,.025,.12),vec3(.6,.4,.84),density).add(filaments.mul(envelope).mul(.14));
    const bodyColor = mix(vec3(.055,.025,.12),vec3(.6,.43,.82),lightRegion)
        .mul(turbulence.mul(.3).add(.7)).add(filaments.mul(.15));
    const bodyMist = body.mul(density.mul(.45).add(filaments.mul(.5)))
        .mul(lightRegion.mul(.6).add(.35));
    cloudMaterial.colorNode = mix(mistColor,bodyColor,cohesion);
    cloudMaterial.opacityNode = mix(density.mul(.65).add(filaments.mul(.14).mul(envelope)),bodyMist,cohesion).mul(cloudVisibility);
    const cloud = new THREE.Mesh(cloudGeometry,cloudMaterial);
    cloud.position.z = .1;
    cloud.renderOrder = -3;
    resources.geometries.push(cloudGeometry);
    resources.materials.push(cloudMaterial);

    // 雾与粒子共用转动坐标，随后直接迁移至命盘。
    const formation = new THREE.Group();
    formation.add(cloud);
    root.add(formation);

    // 固定随机种子让重播的形变路径一致，不在每帧生成随机位置。
    let seed = 137;
    const random = () => { seed = (Math.imul(seed,1664525)+1013904223)>>>0; return seed/4294967296; };
    const count = 1800;
    const particleGeometry = new THREE.PlaneGeometry(.045,.045);
    const particleMaterial = new THREE.MeshBasicMaterial({map:glow,transparent:true,depthWrite:false,depthTest:false,blending:THREE.AdditiveBlending,opacity:.7});
    const particles = new THREE.InstancedMesh(particleGeometry,particleMaterial,count);
    particles.instanceMatrix.setUsage(THREE.DynamicDrawUsage);
    particles.frustumCulled = false;
    particles.renderOrder = 5;
    const symbolTargets = new Float32Array(count*3);
    const squareTargets = new Float32Array(count*3);
    const noise = new Float32Array(count*4);
    const layerIndices = new Uint8Array(count);
    const cellIndices = chartExample.cells.flatMap((cell,index)=>cell ? [index] : []);
    const color = new THREE.Color();
    for(let i=0;i<count;i+=1) {
        const angle=random()*Math.PI*2;
        const r=Math.sqrt(random())*symbolRadius;
        const x=Math.cos(angle)*r;
        const y=Math.sin(angle)*r;
        symbolTargets.set([x,y,(random()-.5)*.06],i*3);
        let isLight=x>0;
        if(Math.hypot(x,y-symbolRadius/2)<symbolRadius/2) isLight=true;
        if(Math.hypot(x,y+symbolRadius/2)<symbolRadius/2) isLight=false;
        if(Math.hypot(x,y-symbolRadius/2)<symbolRadius*symbolEyeRatio) isLight=false;
        if(Math.hypot(x,y+symbolRadius/2)<symbolRadius*symbolEyeRatio) isLight=true;
        particles.setColorAt(i,color.set(isLight ? 0xe4d7fa : 0x4e2d77));
        const cell=cellIndices[i%cellIndices.length]!;
        const side=Math.floor(random()*4);
        const edge=random()*1.35-.675;
        squareTargets.set([
            ((cell%4)-1.5)*1.46+(side<2 ? edge : side===2 ? -.675 : .675),
            (1.5-Math.floor(cell/4))*1.46+(side>=2 ? edge : side===0 ? -.675 : .675),
            .09,
        ],i*3);
        layerIndices[i]=i%3;
        noise.set([random(),random(),random(),random()],i*4);
    }
    root.add(particles);
    resources.geometries.push(particleGeometry);
    resources.materials.push(particleMaterial);

    // 单条轮廓从圆变方，只与中央命盘共用矩阵，避免重叠描边。
    const contours = [1].map(index=>{
        const geometry = new THREE.BufferGeometry();
        const positions = new Float32Array(129*3);
        geometry.setAttribute('position',new THREE.BufferAttribute(positions,3).setUsage(THREE.DynamicDrawUsage));
        const material=new THREE.LineBasicMaterial({color:index===1 ? 0xe6d2ff : 0x9b77cb,transparent:true,opacity:0,depthWrite:false});
        const line = new THREE.Line(geometry,material);
        line.frustumCulled=false;
        root.add(line);
        resources.geometries.push(geometry);
        resources.materials.push(material);
        return {line,positions,index};
    });
    const pulseMaterial = new THREE.SpriteMaterial({map:glow,color:0xd8c0ff,transparent:true,opacity:0,depthTest:false,depthWrite:false,blending:THREE.AdditiveBlending});
    const pulse = new THREE.Sprite(pulseMaterial);
    pulse.scale.set(9,9,1);
    pulse.renderOrder=6;
    root.add(pulse);
    resources.materials.push(pulseMaterial);
    const dummy = new THREE.Object3D();
    const target = new THREE.Vector3();
    const circlePoint = new THREE.Vector3();
    const symbolMatrix = new THREE.Matrix4();

    /** 所有位置均在命盘局部空间中更新，适配尺寸、暂停和主时间轴重播。 */
    function update(time: number, layerMatrices: readonly THREE.Matrix4[], flightDuration: number) {
        const state=preludeFrame(time,flightDuration);
        time=state.effectTime;
        root.visible=state.active;
        if(!state.active) return;
        clock.value=time;
        cloudVisibility.value=state.chaos;
        cloud.visible=state.chaos>.001;
        cohesion.value=state.condense;
        formation.visible=state.chaos>.001;
        formation.rotation.set((.26+Math.sin(time*.6)*.1)*state.polarity,Math.sin(time*.32)*.3*state.polarity,time*.32);
        formation.updateMatrix();
        symbolMatrix.copy(formation.matrix);
        particleMaterial.opacity=state.particles;
        particles.visible=state.particles>.001;
        pulseMaterial.opacity=state.pulse*.23;
        pulse.scale.setScalar(7+state.pulse*3);
        for(let i=0;particles.visible && i<count;i+=1) {
            const n=i*4;
            const radiusChaos=.8+noise[n+2]!*3.2;
            target.fromArray(symbolTargets,i*3);
            // 沿同向螺旋落入目标位置，保持旋流方向，避免粒子直线横穿太极。
            const spin=Math.atan2(target.y,target.x)+(1-state.polarity)*(1.6+noise[n]!*2+time*.12);
            const radius=THREE.MathUtils.lerp(radiusChaos,Math.hypot(target.x,target.y),state.polarity);
            dummy.position.set(Math.cos(spin)*radius,Math.sin(spin)*radius,THREE.MathUtils.lerp((noise[n+1]!-.5)*3,target.z,state.polarity)).applyMatrix4(symbolMatrix);
            target.fromArray(squareTargets,i*3).applyMatrix4(layerMatrices[layerIndices[i]!]!);
            dummy.position.lerp(target,state.grid);
            dummy.quaternion.identity();
            dummy.scale.setScalar((.6+noise[n+3]!)*(.85+state.grid*.45));
            dummy.updateMatrix();
            particles.setMatrixAt(i,dummy.matrix);
        }
        particles.instanceMatrix.needsUpdate=true;
        contours.forEach(({line,positions,index})=>{
            line.material.opacity=state.grid*.44*(1-Math.min(1,Math.max(0,(time-8.3)/1.1)));
            line.visible=line.material.opacity>.001;
            if(!line.visible) return;
            for(let j=0;j<=128;j+=1) {
                const angle=j/128*Math.PI*2;
                const x=Math.cos(angle),y=Math.sin(angle);
                circlePoint.set(x*(symbolRadius+.1),y*(symbolRadius+.1),.02).applyMatrix4(symbolMatrix);
                const edge=2.865/Math.max(Math.abs(x),Math.abs(y));
                target.set(x*edge,y*edge,.08).applyMatrix4(layerMatrices[index]!);
                circlePoint.lerp(target,state.grid);
                circlePoint.toArray(positions,j*3);
            }
            line.geometry.attributes.position!.needsUpdate=true;
        });
    }
    return {update};
}
