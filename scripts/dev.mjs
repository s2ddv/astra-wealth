import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

// POSIX process groups include pnpm/Next and Cargo's spawned API process.
// The first exit stops the whole development session; Ctrl-C leaves no servers.
export function supervise(commands) {
  if (commands.length === 0) return Promise.resolve(0);
  const children = [];
  let stopping = false;
  let result = 0;
  let timer;
  let finish;
  const done = new Promise((resolve) => { finish = resolve; });
  const signalGroup = (child, signal) => {
    if (!child.pid) return;
    try { process.kill(-child.pid, signal); }
    catch (error) { if (error.code !== "ESRCH") throw error; }
  };
  const stop = (code) => {
    if (stopping) return;
    stopping = true;
    result = code;
    for (const child of children) signalGroup(child, "SIGTERM");
    timer = setTimeout(() => {
      for (const child of children) signalGroup(child, "SIGKILL");
    }, 5000);
    timer.unref();
  };
  const interrupt = () => stop(130);
  const terminate = () => stop(143);
  process.on("SIGINT", interrupt);
  process.on("SIGTERM", terminate);
  let remaining = commands.length;
  for (const { command, args, cwd } of commands) {
    const child = spawn(command, args, { cwd, stdio: "inherit", detached: true });
    children.push(child);
    child.on("error", (error) => { console.error(`Cannot start ${command}: ${error.message}`); stop(1); });
    child.on("close", (code, signal) => {
      stop(signal ? 1 : (code ?? 1));
      if (--remaining === 0) {
        clearTimeout(timer);
        process.off("SIGINT", interrupt);
        process.off("SIGTERM", terminate);
        finish(result);
      }
    });
  }
  return done;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = fileURLToPath(new URL("../", import.meta.url));
  process.exitCode = await supervise([
    { command: "pnpm", args: ["dev:web"], cwd: root },
    { command: "cargo", args: ["run", "--locked", "-p", "astra-api", "--bin", "astra-api"], cwd: resolve(root, "apps/api-rust") },
  ]);
}
