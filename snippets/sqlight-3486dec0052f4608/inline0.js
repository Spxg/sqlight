
export async function sqlightInitializationMarker(base, filename, write) {
    const root = await navigator.storage.getDirectory();
    const directory = await root.getDirectoryHandle(base, { create: true });
    try {
        const markers = await directory.getDirectoryHandle('.sqlight-initialized', { create: write });
        await markers.getFileHandle(encodeURIComponent(filename), { create: write });
        return true;
    } catch (error) {
        if (!write && error.name === 'NotFoundError') return false;
        throw error;
    }
}
