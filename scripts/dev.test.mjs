import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { supervise } from "./dev.mjs";

test("a failed service stops its sibling and propagates the exit status", { timeout: 10000 }, async () => {
  const directory = await mkdtemp(join(tmpdir(), "astra-dev-test-"));
  const pidfile = join(directory, "pid");
  try {
    const code = await supervise([
      { command: process.execPath, args: ["-e", `require('fs').writeFileSync(${JSON.stringify(pidfile)},String(process.pid)); setInterval(()=>{},1000);`] },
      { command: process.execPath, args: ["-e", `const t=setInterval(()=>{if(require('fs').existsSync(${JSON.stringify(pidfile)})){clearInterval(t);process.exit(7)}},20);`] },
    ]);
    assert.equal(code, 7);
    const pid = Number(await readFile(pidfile, "utf8"));
    assert.throws(() => process.kill(pid, 0), { code: "ESRCH" });
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("a missing executable fails instead of leaving another service running", { timeout: 10000 }, async () => {
  assert.equal(await supervise([
    { command: "astra-nonexistent-command-for-test", args: [] },
    { command: process.execPath, args: ["-e", "setInterval(()=>{},1000)"] },
  ]), 1);
});
