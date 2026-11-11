import ace from 'ace-builds/src-noconflict/ace';
import 'ace-builds/src-noconflict/mode-sql';
import 'ace-builds/src-noconflict/theme-github';
import 'ace-builds/src-noconflict/theme-github_dark';
import 'ace-builds/src-noconflict/theme-github_light_default';
import 'ace-builds/src-noconflict/theme-gruvbox';
import 'ace-builds/src-noconflict/theme-gruvbox_dark_hard';
import 'ace-builds/src-noconflict/theme-gruvbox_light_hard';
import 'ace-builds/src-noconflict/keybinding-emacs';
import 'ace-builds/src-noconflict/keybinding-sublime';
import 'ace-builds/src-noconflict/keybinding-vim';
import 'ace-builds/src-noconflict/keybinding-vscode';
import * as FloatingUIDOM from '@floating-ui/dom';
import Split from 'split-grid';
import { packZip, unpackZip } from './database-archive';
import './vendor.css';

// Keep the globals used by the Rust bindings. Every editor mode, theme and
// keybinding exposed in the settings menu is bundled above.
Object.assign(globalThis, {
  ace,
  FloatingUIDOM,
  Split,
  SQLightArchive: { packZip, unpackZip },
});
