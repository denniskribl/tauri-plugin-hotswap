import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightClientMermaid from '@pasqal-io/starlight-client-mermaid';
import { ion } from 'starlight-ion-theme';

export default defineConfig({
  site: 'https://hotswap.kribl.io',
  integrations: [
    starlight({
      title: '\u{1F525}\u{1F504} tauri-plugin-hotswap',
      description: 'Open-source OTA frontend updates for Tauri v2',
      plugins: [
        starlightClientMermaid(),
        ion({
          icons: { iconDir: './src/icons' },
          footer: {
            text: '\u{1F525}\u{1F504} ',
            links: [
              {
                text: 'GitHub',
                href: 'https://github.com/denniskribl/tauri-plugin-hotswap',
              },
              {
                text: 'npm',
                href: 'https://www.npmjs.com/package/tauri-plugin-hotswap-api',
              },
            ],
          },
        }),
      ],
      social: [
        {
          icon: 'github',
          label: 'GitHub',
          href: 'https://github.com/denniskribl/tauri-plugin-hotswap',
        },
      ],
      editLink: {
        baseUrl:
          'https://github.com/denniskribl/tauri-plugin-hotswap/edit/main/',
      },
      customCss: ['./src/styles/custom.css'],
      sidebar: [
        { label: 'Introduction', slug: 'index' },
        { label: 'Readme', slug: 'readme' },
        {
          label: 'Guides',
          items: [
            { label: 'Configuration', slug: 'configuration' },
            { label: 'Creating Bundles', slug: 'creating-bundles' },
            { label: 'Server Contract', slug: 'server-contract' },
          ],
        },
        {
          label: 'Reference',
          items: [
            { label: 'API Reference', slug: 'api-reference' },
            { label: 'Architecture', slug: 'architecture' },
            { label: 'Security', slug: 'security' },
            { label: 'Design Philosophy', slug: 'philosophy' },
          ],
        },
      ],
    }),
  ],
});
