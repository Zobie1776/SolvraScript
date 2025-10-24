import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const base = dirname(fileURLToPath(import.meta.url));

export default {
  kit: {
    alias: {
      '@nova-ide/ui': resolve(base, '../../packages/ui/src/lib/index.ts')
    }
  },
  preprocess: [vitePreprocess({
    postcss: true
  })]
};
