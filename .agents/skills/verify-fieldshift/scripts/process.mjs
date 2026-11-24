// AI-assisted (OpenBot agent)
import { spawn } from 'node:child_process';
import { openSync, closeSync } from 'node:fs';

export class CheckError extends Error {
  constructor(message, code = 1) { super(message); this.code = code; }
}
export const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
export function alive(pid) {
  try { process.kill(-pid, 0); return true; } catch (e) {
    if (e.code === 'ESRCH') return false;
    throw e;
  }
}
export async function stop(child) {
  if (!alive(child.pid)) return { pid: child.pid, gone: true };
  process.kill(-child.pid, 'SIGTERM');
  const until = Date.now() + 20_000;
  while (alive(child.pid) && Date.now() < until) await delay(100);
  if (alive(child.pid)) process.kill(-child.pid, 'SIGKILL');
  const hard = Date.now() + 5_000;
  while (alive(child.pid) && Date.now() < hard) await delay(100);
  return { pid: child.pid, gone: !alive(child.pid) };
}
export function launch(command, args, options, log, children) {
  const fd = openSync(log, 'a');
  const child = spawn(command, args, { ...options, detached: true, stdio: ['ignore', fd, fd] });
  closeSync(fd);
  children.push(child);
  child.result = new Promise(resolve => {
    child.once('error', error => resolve({ error }));
    child.once('exit', (code, signal) => resolve({ code, signal }));
  });
  return child;
}
export async function command(command, args, options, log, children, deadline) {
  const child = launch(command, args, options, log, children);
  let timer;
  const result = await Promise.race([
    child.result,
    new Promise(resolve => { timer = setTimeout(() => resolve({ timeout: true }), Math.max(1, deadline - Date.now())); })
  ]);
  clearTimeout(timer);
  if (result.timeout) { await stop(child); throw new CheckError(`Timeout: ${command}`); }
  if (result.error) throw new CheckError(`${command}: ${result.error.message}`, result.error.code === 'ENOENT' ? 3 : 1);
  if (result.code !== 0) throw new CheckError(`${command} exited ${result.code ?? result.signal}`);
}
