import * as wasm from "../pkg/eozin"

import svs_file from "../assets/CMU-1-Small-Region.svs?url";

await wasm.default();

export async function startApp(blob: Blob): Promise<void> {
  const file = new File([blob], "blob.file", { type: blob.type, lastModified: Date.now() });
  startAppWithFile(file);
}

export async function startAppWithFile(file: File): Promise<void> {
  const worker1 = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  const worker2 = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  const worker3 = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  const worker4 = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  let builder = (new wasm.EozinViewerBuilder())
    .set_canvas_id("eozin-wasm-viewer")
    .set_worker(worker1)
    .set_worker(worker2)
    .set_worker(worker3)
    .set_worker(worker4);
  const app = await builder.build_with_file(file);
  app.start();
}

async function downloadCmuAndStartApp(): Promise<void> {
  const img = await fetch(svs_file);
  const blob = await img.blob();
  await startApp(blob);
}

function resizeCanvas() {
  const canvas = document.getElementById("eozin-wasm-viewer") as HTMLCanvasElement;
  const parent = canvas.parentElement;
  if (parent) {
    const observer = new ResizeObserver(entries => {
      for (let entry of entries) {
        const { width } = entry.contentRect;
        if (width < 1000) {
          canvas.width = width;
          canvas.style.width = `${width}px`;
        }
      }
    });
    observer.observe(parent);
  }
}

async function loadFileAndStartApp(ev: Event): Promise<void> {
  const target = ev.target as HTMLInputElement;
  if (target.files && target.files[0]) {
    const f = target.files[0];
    await startAppWithFile(f);
  }
}

async function main(): Promise<void> {
  // console.log("Start app");
  resizeCanvas();
  const fileInput = document.getElementById("eozin-viewer-file-input") as HTMLInputElement;
  fileInput?.addEventListener("change", loadFileAndStartApp);
  downloadCmuAndStartApp();

}

main().catch(console.error);
