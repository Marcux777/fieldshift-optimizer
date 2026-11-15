import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: '200.html'
    }),
    // Inline bootstrap scripts are allowed by hash; the server adds the remaining CSP directives
    // (frame-ancestors, base-uri, ...) as an HTTP header (backend/crates/fieldshift-api/src/lib.rs).
    csp: {
      mode: 'hash',
      directives: {
        'script-src': ['self'],
        'object-src': ['none'],
        'base-uri': ['self']
      }
    }
  }
};
export default config;
