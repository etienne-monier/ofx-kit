import { readdir, readFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';

const root = path.resolve('src/content/docs');
const contentExtensions = new Set(['.md', '.mdx']);
const linkPattern = /!?\[[^\]]*\]\(([^)]+)\)/g;

async function contentFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await contentFiles(entryPath)));
    } else if (contentExtensions.has(path.extname(entry.name))) {
      files.push(entryPath);
    }
  }

  return files;
}

function isExternal(target) {
  return /^(?:[a-z][a-z\d+.-]*:|\/)/i.test(target);
}

function candidates(target) {
  const normalized = target.endsWith('/')
    ? path.join(target, 'index.md')
    : target;
  const resolved = path.resolve(normalized);
  const extension = path.extname(resolved);

  if (extension) {
    return [resolved];
  }

  return [
    resolved,
    `${resolved}.md`,
    `${resolved}.mdx`,
    path.join(resolved, 'index.md'),
    path.join(resolved, 'index.mdx'),
  ];
}

const files = await contentFiles(root);
const errors = [];

for (const file of files) {
  const source = await readFile(file, 'utf8');
  for (const match of source.matchAll(linkPattern)) {
    const target = match[1].trim().split(/[?#]/, 1)[0];
    if (!target || isExternal(target)) {
      continue;
    }

    const resolvedTarget = path.resolve(path.dirname(file), target);
    const targetCandidates = candidates(resolvedTarget);
    if (!targetCandidates.some((candidate) => existsSync(candidate))) {
      errors.push(`${path.relative(process.cwd(), file)} -> ${target}`);
    }
  }
}

if (errors.length > 0) {
  console.error('Broken documentation links:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exitCode = 1;
} else {
  console.log(`Checked internal links in ${files.length} documentation files.`);
}
