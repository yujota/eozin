import { execSync } from 'child_process';
import path from 'node:path';
import fs from 'node:fs';
import { remark } from 'remark';
import gfm from 'remark-gfm';
import sortPackageJson from 'sort-package-json'

const pkgName = process.argv[2]

let rustCmd = "";
let pkgDir = "";

console.log(pkgName);
switch (pkgName) {
  case "@eozin/eozin-web":
    pkgDir = "npm-packages/web";
    rustCmd = `wasm-pack build --target web --release --out-name eozin --out-dir ${pkgDir} -- --features web`;
    break;
  case "@eozin/eozin-node":
    pkgDir = "npm-packages/node";
    rustCmd = `wasm-pack build --target nodejs --release --out-name eozin --out-dir ${pkgDir} -- --features node`;
    break;
}

console.log(`Executing: ${rustCmd}`);
execSync(rustCmd, { stdio: 'inherit' });


const packageJsonPath = path.join(pkgDir, 'package.json');
const pkg = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));

pkg.name = pkgName;
pkg.description = "Digital pathology image decoder",
  pkg.repository = {
    "type": "git",
    "url": "git+https://github.com/yujota/eozin",
  };
pkg.bugs = {
  "url": "git+https://github.com/yujota/eozin/issues",
};
pkg.homepage = "https://github.com/yujota/eozin";
pkg.keywords = ["pathology", "wsi", "svs", "vsi", "ndpi", "tiff"]
pkg.publishConfig = { "access": "public" };
pkg.sideEffects = false;

fs.writeFileSync(packageJsonPath, JSON.stringify(sortPackageJson(pkg), null, 2));
console.log('package.json has been updated!');


const packageReadmePath = path.join(pkgDir, 'README.md');
execSync(`cp README.md ${packageReadmePath}`, { stdio: 'inherit' });
const processor = remark().use(gfm);
const rawContent = fs.readFileSync('README.md', 'utf8');
const ast = processor.parse(rawContent);
let hdrFlag = true;
ast.children = ast.children.filter((node, index) => {
  if (node.type === 'heading') {
    if (node.children[0]?.value === '@eozin/eozin-web (Browser)' && pkgName !== '@eozin/eozin-web') {
      hdrFlag = false;
      return false
    }
    if (node.children[0]?.value === '@eozin/eozin-node (Node.js / Bun)' && pkgName !== '@eozin/eozin-node') {
      hdrFlag = false;
      return false
    }
    hdrFlag = true;
  }
  return hdrFlag;
});

const newContent = processor.stringify(ast);
fs.writeFileSync(packageReadmePath, newContent);

console.log('Done!');
