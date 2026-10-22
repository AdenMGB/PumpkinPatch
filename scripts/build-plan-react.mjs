#!/usr/bin/env node
/**
 * Build upstream Plan React dashboard and copy into patch-plan-http static assets.
 * Source: https://github.com/plan-player-analytics/Plan (LGPL-3.0-or-later)
 */
import { execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const cloneDir = path.join(root, '.cache', 'plan-upstream');
const dashboard = path.join(cloneDir, 'Plan', 'react', 'dashboard');
const out = path.join(root, 'crates', 'patch-plan-http', 'static', 'plan-react');

if (!fs.existsSync(path.join(dashboard, 'package.json'))) {
  fs.mkdirSync(path.dirname(cloneDir), { recursive: true });
  if (!fs.existsSync(cloneDir)) {
    execSync(
      'git clone --depth 1 --filter=blob:none --sparse https://github.com/plan-player-analytics/Plan.git .',
      { cwd: cloneDir, stdio: 'inherit' },
    );
    execSync('git sparse-checkout set Plan/react/dashboard', { cwd: cloneDir, stdio: 'inherit' });
  }
}

execSync('corepack enable && yarn install --immutable', { cwd: dashboard, stdio: 'inherit' });
execSync('yarn build', { cwd: dashboard, stdio: 'inherit' });

fs.rmSync(out, { recursive: true, force: true });
fs.cpSync(path.join(dashboard, 'build'), out, { recursive: true });
fs.writeFileSync(
  path.join(out, 'SOURCE.txt'),
  'Built from https://github.com/plan-player-analytics/Plan Plan/react/dashboard (LGPL-3.0-or-later)\n',
);
console.log('Plan React dashboard copied to', out);
