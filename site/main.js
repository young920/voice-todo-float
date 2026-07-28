(function () {
  'use strict';

  // ====== Hero dithering shader — 手写 WebGL warp 4x4 noise ======
  // 参考 21st.dev hero-dithering-card（@paper-design/shaders-react Dithering）
  // shape="warp" / type="4x4" / speed 0.2 闲置，0.6 hover
  // 颜色：#C9A84C 金色（品牌色）
  function initHeroDither() {
    const canvas = document.querySelector('.hero-dither-bg');
    if (!canvas) return;
    const gl = canvas.getContext('webgl', { premultipliedAlpha: false, alpha: true }) ||
               canvas.getContext('experimental-webgl', { premultipliedAlpha: false, alpha: true });
    if (!gl) {
      // WebGL 不可用就降级显示静态背景
      canvas.style.background = 'radial-gradient(ellipse 80% 60% at 50% 50%, rgba(201, 168, 76, 0.18) 0%, transparent 70%)';
      return;
    }

    const vsSource = `
      attribute vec2 a_position;
      void main() { gl_Position = vec4(a_position, 0.0, 1.0); }
    `;

    // Warp 4x4 Bayer 抖动 + 域扭曲（domain warping）
    // 双色调：桃色 #E8A598 + 深梅红 #C53F3F 混合
    // 输出 alpha = dither 强度
    // GLSL ES 1.0 不支持数组构造器，用 if-else 表达 Bayer 4x4 矩阵
    const fsSource = `
      precision highp float;
      uniform vec2 u_resolution;
      uniform float u_time;
      uniform vec3 u_color1;          /* 桃色/salmon */
      uniform vec3 u_color2;          /* 深梅红 */
      uniform float u_speed;

      // hash 随机
      float hash(vec2 p) {
        return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
      }

      // noise 2D
      float noise(vec2 p) {
        vec2 i = floor(p);
        vec2 f = fract(p);
        vec2 u = f * f * (3.0 - 2.0 * f);
        return mix(
          mix(hash(i + vec2(0.0, 0.0)), hash(i + vec2(1.0, 0.0)), u.x),
          mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x),
          u.y
        );
      }

      // FBM (4 octaves)
      float fbm(vec2 p) {
        float v = 0.0;
        float a = 0.5;
        for (int i = 0; i < 4; i++) {
          v += a * noise(p);
          p *= 2.05;
          a *= 0.55;
        }
        return v;
      }

      // 4x4 Bayer 矩阵 — GLSL ES 1.0 兼容（用 mod 取索引）
      float bayer4x4(int x, int y) {
        int idx = y * 4 + x;
        if (idx == 0) return 0.0/16.0;
        if (idx == 1) return 8.0/16.0;
        if (idx == 2) return 2.0/16.0;
        if (idx == 3) return 10.0/16.0;
        if (idx == 4) return 12.0/16.0;
        if (idx == 5) return 4.0/16.0;
        if (idx == 6) return 14.0/16.0;
        if (idx == 7) return 6.0/16.0;
        if (idx == 8) return 3.0/16.0;
        if (idx == 9) return 11.0/16.0;
        if (idx == 10) return 1.0/16.0;
        if (idx == 11) return 9.0/16.0;
        if (idx == 12) return 15.0/16.0;
        if (idx == 13) return 7.0/16.0;
        if (idx == 14) return 13.0/16.0;
        return 5.0/16.0;
      }

      void main() {
        vec2 uv = gl_FragCoord.xy / u_resolution;
        float t = u_time * u_speed;

        // 更强的 domain warping：FBM 驱动 3 层扭曲
        vec2 q = vec2(
          fbm(uv * 3.0 + vec2(0.0, t * 0.4)),
          fbm(uv * 3.0 + vec2(5.2, t * 0.4))
        );
        vec2 r = vec2(
          fbm(uv * 3.0 + 4.0 * q + vec2(1.7, 9.2) + t * 0.25),
          fbm(uv * 3.0 + 4.0 * q + vec2(8.3, 2.8) + t * 0.2)
        );
        float f = fbm(uv * 3.0 + 4.0 * r);

        // 4x4 Bayer 抖动：f 与阈值比较
        int x = int(mod(floor(gl_FragCoord.x / 3.0), 4.0));  /* 3px 单元格让密度更高 */
        int y = int(mod(floor(gl_FragCoord.y / 3.0), 4.0));
        float threshold = bayer4x4(x, y);

        // dither: f > threshold 时点亮，smoothstep 软化（加宽过渡让点更柔）
        float d = smoothstep(threshold - 0.35, threshold + 0.1, f);

        // 中心衰减（卡片中央稍亮，边缘柔过渡）
        float dist = distance(uv, vec2(0.5));
        float vignette = smoothstep(1.05, 0.15, dist);

        // 双色调 + 对角线流动：根据 uv.x + uv.y 在桃色和深梅红之间过渡
        // 上左 = 桃色（亮），下右 = 深梅红（暗），形成流动方向
        float diag = uv.x * 0.7 + uv.y * 0.5;            /* 对角线因子 */
        vec3 col = mix(u_color1, u_color2, smoothstep(0.25, 0.85, diag * (0.6 + f * 0.8)));

        // 提升暗部饱和度：f 低值时让梅红更红（避免单色感）
        col = mix(col, u_color2, (1.0 - f) * 0.35);

        // 输出：颜色 × dither × 中心衰减（alpha 提高让 dither 看得见）
        gl_FragColor = vec4(col * d * vignette, d * vignette * 0.95);
      }
    `;

    function compile(type, src) {
      const sh = gl.createShader(type);
      gl.shaderSource(sh, src);
      gl.compileShader(sh);
      if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
        console.warn('Shader compile error:', gl.getShaderInfoLog(sh));
        gl.deleteShader(sh);
        return null;
      }
      return sh;
    }
    const vs = compile(gl.VERTEX_SHADER, vsSource);
    const fs = compile(gl.FRAGMENT_SHADER, fsSource);
    if (!vs || !fs) return;
    const prog = gl.createProgram();
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      console.warn('Program link error:', gl.getProgramInfoLog(prog));
      return;
    }
    gl.useProgram(prog);

    // 全屏 quad
    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([
      -1, -1, 1, -1, -1, 1,
      -1, 1, 1, -1, 1, 1,
    ]), gl.STATIC_DRAW);
    const aPos = gl.getAttribLocation(prog, 'a_position');
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);

    const uRes = gl.getUniformLocation(prog, 'u_resolution');
    const uTime = gl.getUniformLocation(prog, 'u_time');
    const uColor1 = gl.getUniformLocation(prog, 'u_color1');
    const uColor2 = gl.getUniformLocation(prog, 'u_color2');
    const uSpeed = gl.getUniformLocation(prog, 'u_speed');

    // 双色调：桃色/salmon #E8A598 + 深梅红 #C53F3F
    gl.uniform3f(uColor1, 0.910, 0.647, 0.596);
    gl.uniform3f(uColor2, 0.773, 0.247, 0.247);

    // 自适应尺寸
    function resize() {
      const rect = canvas.getBoundingClientRect();
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.max(1, Math.floor(rect.width * dpr));
      canvas.height = Math.max(1, Math.floor(rect.height * dpr));
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.uniform2f(uRes, canvas.width, canvas.height);
    }
    resize();
    window.addEventListener('resize', resize);

    // 速度：hover 加速（闲置 0.35 → hover 0.9，比之前 0.2→0.6 明显）
    let speed = 0.35;
    let targetSpeed = 0.35;
    const card = canvas.closest('.hero-card');
    if (card) {
      card.addEventListener('mouseenter', () => { targetSpeed = 0.9; });
      card.addEventListener('mouseleave', () => { targetSpeed = 0.35; });
    }

    const t0 = performance.now();
    function frame(now) {
      // 平滑速度过渡（lerp 0.05 — 缓慢柔和切换）
      speed += (targetSpeed - speed) * 0.05;
      gl.uniform1f(uSpeed, speed);
      gl.uniform1f(uTime, (now - t0) / 1000);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
      requestAnimationFrame(frame);
    }
    requestAnimationFrame(frame);
  }

  // 启动 hero dither（DOMContentLoaded 或即时）
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', initHeroDither);
  } else {
    initHeroDither();
  }

  // ====== Voices 跑马灯：JS 复制一份内容用于无缝循环 ======
  // 把 aria-hidden 的占位卡删掉，重新克隆一份原始内容塞进去
  document.querySelectorAll('.voices-track').forEach((track) => {
    // 先清掉所有 aria-hidden 的占位卡
    track.querySelectorAll('[aria-hidden="true"]').forEach((el) => el.remove());
    // 取一份原始内容（剩下的全是真内容）
    const originals = Array.from(track.children);
    if (originals.length === 0) return;
    originals.forEach((card) => {
      const clone = card.cloneNode(true);
      clone.setAttribute('aria-hidden', 'true');
      track.appendChild(clone);
    });
    // 测量第一份宽度，写入 CSS 变量，让 keyframe 平移刚好一份
    requestAnimationFrame(() => {
      const first = originals[0];
      const lastOfFirst = originals[originals.length - 1];
      const firstRect = first.getBoundingClientRect();
      const lastRect = lastOfFirst.getBoundingClientRect();
      const firstHalfWidth = Math.round(lastRect.right - firstRect.left);
      track.style.setProperty('--half-width', firstHalfWidth + 'px');
    });
  });
// ====== Changelog 竖向任务流：克隆一份内容用于无缝循环 ======
  const clTrack = document.getElementById('cl-loop-track');
  if (clTrack) {
    const originals = Array.from(clTrack.children);
    if (originals.length > 0) {
      originals.forEach((task) => {
        const clone = task.cloneNode(true);
        clone.setAttribute('aria-hidden', 'true');
        clTrack.appendChild(clone);
      });
      // 测量第一份高度写入 CSS 变量
      requestAnimationFrame(() => {
        const first = originals[0];
        const last = originals[originals.length - 1];
        const fr = first.getBoundingClientRect();
        const lr = last.getBoundingClientRect();
        const halfHeight = Math.round(lr.bottom - fr.top);
        clTrack.style.setProperty('--cl-half', halfHeight + 'px');
      });
    }
  }
})();