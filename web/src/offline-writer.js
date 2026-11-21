"use strict";

let file;
self.onmessage = async ({ data }) => {
  const { id, action, key, name, offset, bytes } = data;
  try {
    let value;
    switch (action) {
      case "open": {
        const root = await navigator.storage.getDirectory();
        const store = await root.getDirectoryHandle("chillcenter-downloads", { create: true });
        const item = await store.getDirectoryHandle(key, { create: true });
        const handle = await item.getFileHandle(name, { create: true });
        file = await handle.createSyncAccessHandle();
        value = file.getSize();
        break;
      }
      case "write":
        if (file.write(new Uint8Array(bytes), { at: offset }) !== bytes.byteLength) throw new Error("Écriture incomplète.");
        file.flush();
        break;
      case "truncate":
        file.truncate(0);
        file.flush();
        break;
      case "close":
        file?.close();
        file = null;
        break;
      default: throw new Error("Commande de stockage inconnue.");
    }
    self.postMessage({ id, value });
  } catch (error) {
    self.postMessage({ id, error: error.message, name: error.name });
  }
};
