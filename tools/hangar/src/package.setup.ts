import assert from "node:assert";
import { execa } from "execa";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import { npmBin, npmCacheDir, tmpDir, wingBin } from "./paths";

const shellEnv = {
  ...process.env,
  npm_config_audit: "false",
  npm_config_progress: "false",
  npm_config_yes: "true",
  npm_config_cache: npmCacheDir,
  npm_config_color: "false",
  npm_config_foreground_scripts: "true",
};

export default async function () {
  Object.assign(process.env, shellEnv);

  // reset tmpDir
  await fs.rm(tmpDir, { recursive: true, force: true });
  await fs.mkdir(tmpDir, { recursive: true });
  await execa(npmBin, ["init", "-y"], {
    cwd: tmpDir,
  });

  // use execSync to install npm deps in tmpDir
  const tarballsDir = path.resolve(`${__dirname}/../../../dist`);
  const tarballs = (await fs.readdir(tarballsDir))
    .filter((filename) => filename.endsWith(".tgz"))
    .map((tarball) => `file:${tarballsDir}/${tarball}`);
  console.debug(`Installing npm deps into ${tmpDir}...`);
  const installArgs = [
    "install",
    "--no-package-lock",
    "--install-links=false",
    ...tarballs,
  ];
  const installResult = await execa(npmBin, installArgs, {
    cwd: tmpDir,
  });

  const allowedInstallHooks: RegExp[] = []; // Leaving this mechanism in place in case we need it in the future

  const installHooks =
    installResult.stdout.match(/>.*/g)?.filter((hook) => {
      return !allowedInstallHooks.some((allowedHook) => {
        return allowedHook.test(hook);
      });
    }) ?? [];

  assert.equal(
    installHooks.length,
    0,
    `Install contains unexpected script hooks: \n${installHooks}`
  );
  assert.equal(
    installResult.exitCode,
    0,
    `Failed to install npm deps: \n${installResult.stderr}`
  );
  assert.doesNotMatch(
    installResult.stdout,
    / warn /,
    `Install contains unexpected warning: \n${installResult.stdout}`
  );

  console.debug(`Done!`);

  const versionOutput = await execa(wingBin, ["--version"], {
    cwd: tmpDir,
  });

  assert.equal(
    versionOutput.exitCode,
    0,
    `Failed to get wing version: ${versionOutput.stderr}`
  );

  assert.match(
    versionOutput.stdout,
    /^(\d+\.)?(\d+\.)?(\*|\d+)(-.+)?/,
    `Wing version invalid: ${versionOutput.stderr}`
  );
}
