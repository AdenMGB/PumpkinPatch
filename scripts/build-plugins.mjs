import { spawnSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const repoRoot = join(root, '..')
const pumpkinRoot = join(repoRoot, 'pumpkin')
const artifactsDir = join(root, 'plugins', 'artifacts')

const plugins = [
  {
    dir: 'patch-plan-analytics',
    wasm: 'patch_plan_analytics.wasm',
    artifact: 'patch-plan-analytics.wasm',
  },
  {
    dir: 'patch-hub-lobby',
    wasm: 'patch_hub_lobby.wasm',
    artifact: 'patch-hub-lobby.wasm',
  },
]

function run(command, args, cwd = root) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: 'inherit',
  })
  if (result.status !== 0) {
    process.exit(result.status ?? 1)
  }
}

function runCapture(command, args, cwd) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: 'utf8',
  })
  if (result.status !== 0) {
    console.error(`[patch] ${command} ${args.join(' ')} failed`)
    process.exit(result.status ?? 1)
  }
  return (result.stdout ?? '').trim()
}

mkdirSync(artifactsDir, { recursive: true })

if (existsSync(join(pumpkinRoot, '.git'))) {
  console.log('[patch] Syncing pumpkin submodule (master) for plugin API…')
  const fetch = spawnSync('git', ['fetch', 'origin', 'master'], {
    cwd: pumpkinRoot,
    stdio: 'inherit',
  })
  if (fetch.status === 0) {
    run('git', ['pull', '--ff-only', 'origin', 'master'], pumpkinRoot)
  } else {
    console.warn(
      '[patch] git fetch failed — building against the current pumpkin checkout (offline?).',
    )
  }
  const rev = runCapture('git', ['rev-parse', 'HEAD'], pumpkinRoot)
  writeFileSync(join(artifactsDir, 'pumpkin-api-rev.txt'), `${rev}\n`, 'utf8')
  console.log(`[patch] pumpkin API pin → ${rev.slice(0, 12)}`)
} else {
  console.warn('[patch] pumpkin/ submodule missing — building plugins against Cargo.toml paths only')
}

console.log('[patch] Ensuring wasm32-wasip2 target…')
run('rustup', ['target', 'add', 'wasm32-wasip2'])

for (const plugin of plugins) {
  const crateDir = join(root, 'plugins', plugin.dir)
  const built = join(crateDir, 'target', 'wasm32-wasip2', 'release', plugin.wasm)
  const dest = join(artifactsDir, plugin.artifact)

  console.log(`[patch] Building ${plugin.dir}…`)
  run('cargo', ['build', '--release'], crateDir)

  if (!existsSync(built)) {
    console.error(`[patch] Missing build output: ${built}`)
    process.exit(1)
  }

  copyFileSync(built, dest)
  console.log(`[patch] → plugins/artifacts/${plugin.artifact}`)
}

console.log('[patch] Bundled plugins ready.')
console.log('[patch] Restart networks after this — cached Pumpkin nightly will refresh when the API pin changes.')
