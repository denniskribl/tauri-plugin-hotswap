/**
 * Syncs the root README.md into the Astro Starlight docs.
 * Strips GitHub-specific HTML header and prepends Starlight frontmatter.
 *
 * Run automatically via `pnpm dev` / `pnpm build`.
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const readmePath = resolve(__dirname, '..', 'README.md');
const outputPath = resolve(__dirname, 'src', 'content', 'docs', 'readme.md');

const raw = readFileSync(readmePath, 'utf-8');

// Strip everything before the first markdown heading (removes HTML badges/header)
const firstHeading = raw.indexOf('\n## ');
const body = firstHeading !== -1 ? raw.slice(firstHeading + 1) : raw;

const frontmatter = `---
title: README
description: Quickstart, install snippets, and feature overview.
---

> This page mirrors the [GitHub README](https://github.com/denniskribl/tauri-plugin-hotswap). For the full docs, use the sidebar.

`;

writeFileSync(outputPath, frontmatter + body);
console.log('Synced README -> src/content/docs/readme.md');
