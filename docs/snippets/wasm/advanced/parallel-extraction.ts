import { extractBytes, initWasm, detectRuntime, hasWorkers } from "@kreuzberg/wasm";

async function extractInParallel(documents: Uint8Array[], mimeTypes: string[]) {
  await initWasm();

  const runtime = detectRuntime();
  const canUseWorkers = hasWorkers();

  if (isBrowser() && canUseWorkers) {
    return extractWithWebWorkers(documents, mimeTypes);
  }

  return Promise.all(
    documents.map((bytes, index) => extractBytes(bytes, mimeTypes[index]))
  );
}

function extractWithWebWorkers(documents: Uint8Array[], mimeTypes: string[]) {
  const workerCount = navigator.hardwareConcurrency ?? 2;
  const workers: Worker[] = [];

  for (let i = 0; i < workerCount; i++) {
    workers.push(new Worker("extraction-worker.js"));
  }

  return Promise.all(
    documents.map((bytes, index) =>
      new Promise((resolve, reject) => {
        const worker = workers[index % workers.length];
        worker.postMessage({ bytes, mimeType: mimeTypes[index] });
        worker.onmessage = (e) => resolve(e.data);
        worker.onerror = reject;
      })
    )
  );
}

function isBrowser() {
  return typeof window !== "undefined";
}

extractInParallel([new Uint8Array([1, 2, 3])], ["application/pdf"])
  .then((results) => console.log(results))
  .catch(console.error);
