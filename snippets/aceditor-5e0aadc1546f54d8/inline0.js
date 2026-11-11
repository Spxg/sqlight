
export function defineVimWrite() {
    const { CodeMirror } = ace.require("ace/keyboard/vim");
    CodeMirror.Vim.defineEx("write", "w", cm => cm.ace.execCommand("executeCode"));
}

export function bindCommandShortcut(editor, command) {
    const { HashHandler } = ace.require("ace/keyboard/hash_handler");
    const { keyCodeToString } = ace.require("ace/lib/keys");
    const shortcuts = new HashHandler([command], editor.commands.platform);
    const onKeyDown = event => {
        const modifiers = (event.ctrlKey ? 1 : 0)
            | (event.altKey ? 2 : 0)
            | (event.shiftKey ? 4 : 0)
            | (event.metaKey ? 8 : 0);
        if (!shortcuts.findKeyCommand(modifiers, keyCodeToString(event.keyCode))) return;
        event.preventDefault();
        event.stopPropagation();
        editor.execCommand(command.name);
    };
    // Capture application shortcuts before an editor preset handles the key.
    editor.container.addEventListener("keydown", onKeyDown, true);
    return () => editor.container.removeEventListener("keydown", onKeyDown, true);
}

export function setWorkspaceValue(editor, value) {
    editor.setValue(value, -1);
    editor.session.getUndoManager().reset();
}
