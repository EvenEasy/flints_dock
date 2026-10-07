import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { PHONE_MIN_WIDTH, PHONE_MAX_WIDTH, PHONE_DEFAULT_WIDTH } from '../src/app/display.ts';

// Pass only display preferences to Vite; no credentials are accepted as command arguments.
const [mode, ...args] = process.argv.slice(2);
let width = process.env.VITE_DOCK_WIDTH ?? String(PHONE_DEFAULT_WIDTH);
let backdrop = process.env.VITE_DOCK_BACKDROP ?? '0';
const forwarded: string[] = [];
for (let index = 0; index < args.length; index += 1) {
  const arg = args[index]!;
  if (arg === '--backdrop') backdrop = '1';
  else if (arg === '--no-backdrop') backdrop = '0';
  else if (arg === '--width' || arg.startsWith('--width=')) {
    width = arg === '--width' ? (args[++index] ?? '') : arg.slice('--width='.length);
  } else forwarded.push(arg);
}
if (!/^\d+$/.test(width) || Number(width) < PHONE_MIN_WIDTH || Number(width) > PHONE_MAX_WIDTH) {
  process.stderr.write(
    `Phone width must be between ${PHONE_MIN_WIDTH} and ${PHONE_MAX_WIDTH} CSS px.\n`,
  );
  process.exit(1);
}
const env = { ...process.env, VITE_DOCK_WIDTH: width, VITE_DOCK_BACKDROP: backdrop };
let binary: string;
let command: string[];
if (mode === 'dev' || mode === 'preview') {
  binary = fileURLToPath(new URL('../node_modules/vite/bin/vite.js', import.meta.url));
  command = mode === 'dev' ? [] : ['preview'];
} else if (mode === 'desktop' || mode === 'desktop:build') {
  binary = fileURLToPath(new URL('../node_modules/@tauri-apps/cli/tauri.js', import.meta.url));
  const windowConfig = {
    app: {
      windows: [
        {
          label: 'main',
          title: 'Flint’s Dock',
          width: backdrop === '1' ? 1100 : Number(width),
          height: backdrop === '1' ? 1000 : Math.round((Number(width) * 844) / 390),
          minWidth: PHONE_MIN_WIDTH,
          minHeight: 600,
        },
      ],
    },
  };
  command = [
    mode === 'desktop' ? 'dev' : 'build',
    '--config',
    'tauri.frontend.conf.json',
    '--config',
    JSON.stringify(windowConfig),
  ];
} else throw new Error('Unknown frontend launch mode');

// An argv array avoids shell evaluation of forwarded tool arguments.
const child = spawn(process.execPath, [binary, ...command, ...forwarded], {
  env,
  stdio: 'inherit',
});
for (const signal of ['SIGINT', 'SIGTERM'] as const) process.on(signal, () => child.kill(signal));
child.on('error', () => {
  process.stderr.write('Could not start the frontend launcher.\n');
  process.exitCode = 1;
});
child.on('exit', (code, signal) => {
  process.exitCode = signal ? 130 : (code ?? 1);
});
