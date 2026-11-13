import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';

// 网站使用 Astro 静态构建，并由 Vite 接入现有 Tailwind 样式管线。
export default defineConfig({
  vite: {
    plugins: [tailwindcss()],
    build: {
      rolldownOptions: {
        output: {
          // Three 的循环依赖跨分块后仍须按源码顺序初始化。
          strictExecutionOrder: true,
          codeSplitting: {
            groups: [
              {
                name: 'three',
                test: /[\\/]node_modules[\\/]three[\\/]src[\\/]/,
                // 按源码模块拆分延迟加载的三维依赖，不提高默认告警阈值。
                maxSize: 1_200_000,
              },
            ],
          },
        },
      },
    },
  },
});
