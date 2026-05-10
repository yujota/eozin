import * as wasm from "../pkg/eozin"

let wasmInitPromise: Promise<any> | null = null;
let decoderPromise: Promise<wasm.Eozin> | null = null;
// console.log("worker.js is called");

interface InitWithFile {
  type: "InitWithFile";
  file: File;
}
interface ReadTile {
  type: "ReadTile";
  x: number;
  y: number;
  lv: number;
}
export type Msg = InitWithFile | ReadTile;

interface LoadedTile {
  img: ImageBitmap;
  lv: number;
  x: number;
  y: number;
}

const ctx = self as unknown as DedicatedWorkerGlobalScope;

ctx.onmessage = async (event: MessageEvent<Msg>) => {
  if (!wasmInitPromise) {
    wasmInitPromise = wasm.default();
  }
  await wasmInitPromise;

  const msg = event.data;

  switch (msg.type) {
    case "InitWithFile":
      // console.log("Initializing decoder...");
      decoderPromise = wasm.Eozin.withFile(msg.file);
      await decoderPromise;
      break;

    case "ReadTile":
      if (!decoderPromise) {
        // console.error("Decoder not initialized yet!");
        return;
      }

      const decoder = await decoderPromise;

      // console.log(`Reading tile: ${msg.x}, ${msg.y}, ${msg.lv}`);
      try {
        const tile = await decoder.readTile(msg.lv, msg.x, msg.y);
        const blob = tile.toBlob();

        if (blob) {
          const img = await createImageBitmap(blob);
          const result: LoadedTile = { img, x: msg.x, y: msg.y, lv: msg.lv };
          ctx.postMessage(result, [result.img]);
        }
      } catch (e) {
        console.error("Decode error:", e);
      }
      break;
  }
};
