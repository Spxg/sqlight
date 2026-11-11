import { AsyncUnzipInflate, AsyncZipDeflate, Unzip, unzip, Zip, ZipDeflate } from 'fflate';

function databaseName(path) {
  const parts = path.replaceAll('\\', '/').split('/');
  const filename = parts.at(-1);
  if (!filename || parts.includes('__MACOSX') || filename === '.DS_Store'
      || filename.startsWith('._')) {
    return null;
  }
  return filename;
}

function joinChunks(chunks) {
  if (chunks.length === 1) return chunks[0];
  const data = new Uint8Array(chunks.reduce((size, chunk) => size + chunk.length, 0));
  let offset = 0;
  for (const chunk of chunks) {
    data.set(chunk, offset);
    offset += chunk.length;
  }
  return data;
}

export async function unpackZip(data) {
  const files = new Map();
  await new Promise((resolve, reject) => {
    const names = new Set();
    let duplicate;
    // Validate the central directory without letting fflate put filenames in
    // its plain output object. Names such as __proto__ must remain ordinary files.
    unzip(data, {
      filter(file) {
        const filename = databaseName(file.name);
        if (!filename) return false;
        if (names.has(filename)) {
          duplicate = new Error(`Duplicate database filename: ${filename}`);
          return false;
        }
        names.add(filename);
        files.set(file.name, { filename, originalSize: file.originalSize });
        return false;
      },
    }, (error) => {
      if (error || duplicate) {
        reject(error || duplicate);
        return;
      }
      if (names.size === 0) {
        reject(new Error('ZIP archive contains no database files.'));
        return;
      }
      resolve();
    });
  });

  return new Promise((resolve, reject) => {
    const started = new Set();
    const active = new Set();
    let remaining = files.size;
    let parsed = false;
    let failed = false;
    const fail = (error) => {
      failed = true;
      for (const file of active) file.terminate();
      reject(error);
    };
    const finish = () => {
      if (parsed && remaining === 0 && !failed) {
        resolve(Array.from(files.values(), ({ filename, data }) => ({ filename, data })));
      }
    };
    const archive = new Unzip((entry) => {
      const file = files.get(entry.name);
      if (!file || failed) return;
      if (started.has(entry.name)) {
        fail(new Error(`Duplicate database filename: ${file.filename}`));
        return;
      }
      started.add(entry.name);
      active.add(entry);
      const chunks = [];
      entry.ondata = (error, chunk, final) => {
        if (error) {
          fail(error);
          return;
        }
        chunks.push(chunk);
        if (final) {
          file.data = joinChunks(chunks);
          active.delete(entry);
          remaining--;
          finish();
        }
      };
      entry.start();
    });
    // Streaming headers may omit sizes. Use the central directory to keep
    // small entries synchronous and large decompressed files off the main thread.
    class DatabaseInflate extends AsyncUnzipInflate {
      constructor(filename) {
        super(filename, files.get(filename).originalSize);
      }
    }
    archive.register(DatabaseInflate);
    try {
      archive.push(data, true);
      parsed = true;
      if (started.size !== files.size) {
        fail(new Error('Invalid ZIP archive: missing database files.'));
      } else {
        finish();
      }
    } catch (error) {
      fail(error);
    }
  });
}

export async function packZip(files) {
  const names = new Set();
  for (const { filename } of files) {
    if (names.has(filename)) {
      throw new Error(`Duplicate database filename: ${filename}`);
    }
    names.add(filename);
  }

  return new Promise((resolve, reject) => {
    const chunks = [];
    let failed = false;
    const archive = new Zip((error, chunk, final) => {
      if (error) {
        failed = true;
        archive.terminate();
        reject(error);
      } else {
        chunks.push(chunk);
        if (final) resolve(joinChunks(chunks));
      }
    });
    try {
      for (const { filename, data } of files) {
        const asynchronous = data.length >= 160000;
        const entry = asynchronous
          ? new AsyncZipDeflate(filename)
          : new ZipDeflate(filename);
        archive.add(entry);
        if (failed) {
          archive.terminate();
          return;
        }
        // AsyncZipDeflate transfers its input buffer to its worker.
        entry.push(asynchronous ? data.slice() : data, true);
        if (failed) return;
      }
      archive.end();
    } catch (error) {
      archive.terminate();
      reject(error);
    }
  });
}
