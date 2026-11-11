
export function viewportSize() {
    return globalThis.FloatingUIDOM.size({
        padding: 8,
        apply({ availableHeight, elements }) {
            elements.floating.style.maxHeight = `${Math.max(0, availableHeight)}px`;
        },
    });
}
