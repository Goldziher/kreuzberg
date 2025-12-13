import { extractBytes, initWasm } from "@kreuzberg/wasm";

async function detectAndExtract(bytes: Uint8Array) {
  await initWasm();

  const magic = bytes.slice(0, 8);
  const magicStr = Array.from(magic).map((b) => b.toString(16).padStart(2, "0")).join("");

  console.log("Magic bytes:", magicStr);

  let mimeType = "application/octet-stream";

  if (magicStr.startsWith("25504446")) mimeType = "application/pdf";
  else if (magicStr.startsWith("504b0304")) mimeType = "application/zip";
  else if (magicStr.startsWith("ffd8ff")) mimeType = "image/jpeg";
  else if (magicStr.startsWith("89504e47")) mimeType = "image/png";
  else if (magicStr.startsWith("474946")) mimeType = "image/gif";

  console.log("Detected MIME type:", mimeType);

  const result = await extractBytes(bytes, mimeType);
  return result;
}

const testBytes = new Uint8Array([0x25, 0x50, 0x44, 0x46]);
detectAndExtract(testBytes)
  .then((r) => console.log(r))
  .catch(console.error);
