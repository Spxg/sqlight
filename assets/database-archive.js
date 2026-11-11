import { unzip, zip } from 'fflate';

function databaseName(path) {
  const parts = path.replaceAll('\\', '/').split('/');
  const filename = parts.at(-1);
  if (!filename || parts.includes('__MACOSX') || filename === '.DS_Store'
      || filename.startsWith('._')) {
    return null;
  }
  return filename;
}

export function unpackZip(data) {
  return new Promise((resolve, reject) => {
    const names = new Map();
    let duplicate;
    unzip(data, {
      filter(file) {
        const filename = databaseName(file.name);
        if (!filename) return false;
        if (names.has(filename)) {
          duplicate = new Error(`Duplicate database filename: ${filename}`);
          return false;
        }
        names.set(filename, file.name);
        return true;
      },
    }, (error, files) => {
      if (error || duplicate) {
        reject(error || duplicate);
        return;
      }
      if (names.size === 0) {
        reject(new Error('ZIP archive contains no database files.'));
        return;
      }
      resolve(Array.from(names, ([filename, path]) => ({
        filename,
        data: files[path],
      })));
    });
  });
}

export function packZip(files) {
  return new Promise((resolve, reject) => {
    const archive = Object.create(null);
    for (const { filename, data } of files) {
      if (Object.hasOwn(archive, filename)) {
        reject(new Error(`Duplicate database filename: ${filename}`));
        return;
      }
      archive[filename] = data;
    }
    zip(archive, (error, data) => {
      if (error) reject(error);
      else resolve(data);
    });
  });
}
