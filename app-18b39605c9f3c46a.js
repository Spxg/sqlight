import { defineVimWrite, setWorkspaceValue } from './snippets/aceditor-5e0aadc1546f54d8/inline0.js';
import * as import1 from "./snippets/floating-ui-fe1d6f75754df512/inline0.js"
import * as import2 from "./snippets/aceditor-5e0aadc1546f54d8/inline0.js"


export class IntoUnderlyingByteSource {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingByteSourceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingbytesource_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get autoAllocateChunkSize() {
        const ret = wasm.intounderlyingbytesource_autoAllocateChunkSize(this.__wbg_ptr);
        return ret >>> 0;
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingbytesource_cancel(ptr);
    }
    /**
     * @param {ReadableByteStreamController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingbytesource_pull(this.__wbg_ptr, controller);
        return ret;
    }
    /**
     * @param {ReadableByteStreamController} controller
     */
    start(controller) {
        wasm.intounderlyingbytesource_start(this.__wbg_ptr, controller);
    }
    /**
     * @returns {ReadableStreamType}
     */
    get type() {
        const ret = wasm.intounderlyingbytesource_type(this.__wbg_ptr);
        return __wbindgen_enum_ReadableStreamType[ret];
    }
}
if (Symbol.dispose) IntoUnderlyingByteSource.prototype[Symbol.dispose] = IntoUnderlyingByteSource.prototype.free;

export class IntoUnderlyingSink {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSinkFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsink_free(ptr, 0);
    }
    /**
     * @param {any} reason
     * @returns {Promise<any>}
     */
    abort(reason) {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_abort(ptr, reason);
        return ret;
    }
    /**
     * @returns {Promise<any>}
     */
    close() {
        const ptr = this.__destroy_into_raw();
        const ret = wasm.intounderlyingsink_close(ptr);
        return ret;
    }
    /**
     * @param {any} chunk
     * @returns {Promise<any>}
     */
    write(chunk) {
        const ret = wasm.intounderlyingsink_write(this.__wbg_ptr, chunk);
        return ret;
    }
}
if (Symbol.dispose) IntoUnderlyingSink.prototype[Symbol.dispose] = IntoUnderlyingSink.prototype.free;

export class IntoUnderlyingSource {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        IntoUnderlyingSourceFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_intounderlyingsource_free(ptr, 0);
    }
    cancel() {
        const ptr = this.__destroy_into_raw();
        wasm.intounderlyingsource_cancel(ptr);
    }
    /**
     * @param {ReadableStreamDefaultController} controller
     * @returns {Promise<any>}
     */
    pull(controller) {
        const ret = wasm.intounderlyingsource_pull(this.__wbg_ptr, controller);
        return ret;
    }
}
if (Symbol.dispose) IntoUnderlyingSource.prototype[Symbol.dispose] = IntoUnderlyingSource.prototype.free;
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_208446807d0d1e30: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_Number_ff192a72a0dcdf6d: function(arg0) {
            const ret = Number(arg0);
            return ret;
        },
        __wbg_Split_110715f21dd3e09f: function(arg0) {
            const ret = globalThis.Split(arg0);
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_bigint_get_as_i64_11da4b52cd0aedef: function(arg0, arg1) {
            const v = arg1;
            const ret = typeof(v) === 'bigint' ? v : undefined;
            getDataViewMemory0().setBigInt64((arg0 >>> 0) + 8 * 1, isLikeNone(ret) ? BigInt(0) : ret, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_boolean_get_4f1baf9a00be3c74: function(arg0) {
            const v = arg0;
            const ret = typeof(v) === 'boolean' ? v : undefined;
            return isLikeNone(ret) ? 0xFFFFFF : ret ? 1 : 0;
        },
        __wbg___wbindgen_debug_string_f249a2a76caa9a55: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_in_3f58a9d3b3a7eed2: function(arg0, arg1) {
            const ret = arg0 in arg1;
            return ret;
        },
        __wbg___wbindgen_is_bigint_5fa10641e039d3ef: function(arg0) {
            const ret = typeof(arg0) === 'bigint';
            return ret;
        },
        __wbg___wbindgen_is_falsy_d864b623500ae65d: function(arg0) {
            const ret = !arg0;
            return ret;
        },
        __wbg___wbindgen_is_function_741ff2bb612836f5: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_c22311b1cc228e3a: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_object_8b968c3ab8e96595: function(arg0) {
            const val = arg0;
            const ret = typeof(val) === 'object' && val !== null;
            return ret;
        },
        __wbg___wbindgen_is_string_fc456fee5d3317de: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_be5b34249ff1fc15: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_jsval_eq_38d7bfbe4cc84039: function(arg0, arg1) {
            const ret = arg0 === arg1;
            return ret;
        },
        __wbg___wbindgen_jsval_loose_eq_b2f214e2e4b30e36: function(arg0, arg1) {
            const ret = arg0 == arg1;
            return ret;
        },
        __wbg___wbindgen_number_get_934b640d0889af78: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64((arg0 >>> 0) + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_string_get_7ce8eee7fd327545: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_4c88d8bf264fd984: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_09736879a3f67559: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_addCommand_649bed22624fd804: function(arg0, arg1) {
            arg0.addCommand(arg1);
        },
        __wbg_addEventListener_847fd86b8c33ec43: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3);
        }, arguments); },
        __wbg_appendChild_dbf7ad2f48413560: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.appendChild(arg1);
            return ret;
        }, arguments); },
        __wbg_arrayBuffer_5abbcb61bd48f427: function(arg0) {
            const ret = arg0.arrayBuffer();
            return ret;
        },
        __wbg_arrow_07cbaf8ea823dbdc: function(arg0) {
            const ret = globalThis.FloatingUIDOM.arrow(arg0);
            return ret;
        },
        __wbg_attachShadow_ebe844ae64e30ce4: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.attachShadow(arg1);
            return ret;
        }, arguments); },
        __wbg_autoUpdate_2f7fecf0e074d2fc: function(arg0, arg1, arg2, arg3) {
            const ret = globalThis.FloatingUIDOM.autoUpdate(arg0, arg1, arg2, arg3);
            return ret;
        },
        __wbg_body_5f971cbc6950ca4f: function(arg0) {
            const ret = arg0.body;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_buffer_aca05a93c046db1a: function(arg0) {
            const ret = arg0.buffer;
            return ret;
        },
        __wbg_byobRequest_18280c78e462544c: function(arg0) {
            const ret = arg0.byobRequest;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_byteLength_c5f3c188d5252e5b: function(arg0) {
            const ret = arg0.byteLength;
            return ret;
        },
        __wbg_byteOffset_63cdc7b38f33a45e: function(arg0) {
            const ret = arg0.byteOffset;
            return ret;
        },
        __wbg_call_40b008ba32f73991: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_call_a410eb4b59ed5965: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_cancelBubble_1164dd588f0c3481: function(arg0) {
            const ret = arg0.cancelBubble;
            return ret;
        },
        __wbg_checked_3f95ede4ece3a2c2: function(arg0) {
            const ret = arg0.checked;
            return ret;
        },
        __wbg_clearTimeout_fd2c5dfcf4c67f84: function(arg0, arg1) {
            arg0.clearTimeout(arg1);
        },
        __wbg_click_c6eff13daacf9806: function(arg0) {
            arg0.click();
        },
        __wbg_clipboard_047ad3d952a13066: function(arg0) {
            const ret = arg0.clipboard;
            return ret;
        },
        __wbg_cloneNode_36b90700fd8dbbb0: function() { return handleError(function (arg0) {
            const ret = arg0.cloneNode();
            return ret;
        }, arguments); },
        __wbg_cloneNode_70249c8beed88ee3: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.cloneNode(arg1 !== 0);
            return ret;
        }, arguments); },
        __wbg_close_294d3f727cb7a844: function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments); },
        __wbg_close_bf94aa7e38b9a94b: function() { return handleError(function (arg0) {
            arg0.close();
        }, arguments); },
        __wbg_commands_f50b4d194c526964: function(arg0) {
            const ret = arg0.commands;
            return ret;
        },
        __wbg_composedPath_99300fc960717ac7: function(arg0) {
            const ret = arg0.composedPath();
            return ret;
        },
        __wbg_computePosition_b2a9d305418a77c3: function(arg0, arg1, arg2) {
            const ret = globalThis.FloatingUIDOM.computePosition(arg0, arg1, arg2);
            return ret;
        },
        __wbg_confirm_386a15f7129fded4: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.confirm(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_contains_743aa7c94499c269: function(arg0, arg1) {
            const ret = arg0.contains(arg1);
            return ret;
        },
        __wbg_content_6fed93ca1fd29f06: function(arg0) {
            const ret = arg0.content;
            return ret;
        },
        __wbg_createComment_bf8f324d5ce3d460: function(arg0, arg1, arg2) {
            const ret = arg0.createComment(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_createElementNS_2f7464180077e8e4: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.createElementNS(arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            return ret;
        }, arguments); },
        __wbg_createElement_de4a31ed2fb252be: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_createObjectURL_45693a1454d24dac: function() { return handleError(function (arg0, arg1) {
            const ret = URL.createObjectURL(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_createTextNode_2b48c3fc75dfcc67: function(arg0, arg1, arg2) {
            const ret = arg0.createTextNode(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_data_c7e5e8500f1bca2f: function(arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_debug_29e5044b8c3730bb: function(arg0) {
            console.debug(arg0);
        },
        __wbg_defineVimWrite_59764c393fff30d4: function() { return handleError(function () {
            defineVimWrite();
        }, arguments); },
        __wbg_deleteProperty_3ad7a085da623822: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.deleteProperty(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_delete_b3b71efdb29e73b9: function(arg0, arg1, arg2) {
            arg0.delete(getStringFromWasm0(arg1, arg2));
        },
        __wbg_destroy_8eb96db7df2aa98c: function(arg0) {
            arg0.destroy();
        },
        __wbg_destroy_ed75a87b220d41c0: function(arg0) {
            arg0.destroy();
        },
        __wbg_disconnect_cfb0d06aca1c1b50: function(arg0) {
            arg0.disconnect();
        },
        __wbg_documentElement_5e44e37dd0181a81: function(arg0) {
            const ret = arg0.documentElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_document_3b5c3071b43e97ee: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_done_57e08d0be6e30c02: function(arg0) {
            const ret = arg0.done;
            return ret;
        },
        __wbg_edit_dc999563610263d7: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = ace.edit(getStringFromWasm0(arg0, arg1), arg2);
            return ret;
        }, arguments); },
        __wbg_enqueue_c11626eae1c54509: function() { return handleError(function (arg0, arg1) {
            arg0.enqueue(arg1);
        }, arguments); },
        __wbg_entries_7bb1459bd9b5c2b9: function(arg0) {
            const ret = Object.entries(arg0);
            return ret;
        },
        __wbg_error_757e9472f8410341: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_error_d1fc0193610e0e39: function(arg0) {
            console.error(arg0);
        },
        __wbg_files_d7e65f35a3acc89f: function(arg0) {
            const ret = arg0.files;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_firstElementChild_a60f6eaf7d453487: function(arg0) {
            const ret = arg0.firstElementChild;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_flip_d578755dcae61a3a: function() {
            const ret = globalThis.FloatingUIDOM.flip();
            return ret;
        },
        __wbg_focus_d5e2fdbdb56416f0: function() { return handleError(function (arg0) {
            arg0.focus();
        }, arguments); },
        __wbg_getElementById_7ee213db584f37b0: function(arg0, arg1, arg2) {
            const ret = arg0.getElementById(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_getItem_49556a3e59cfb688: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.getItem(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_getSelectedText_7f7ccff831d01fbf: function(arg0, arg1) {
            const ret = arg1.getSelectedText();
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_getValue_4353e6da0b4acd58: function(arg0, arg1) {
            const ret = arg1.getValue();
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_get_1d33d8d1c6cefa41: function(arg0, arg1, arg2, arg3) {
            const ret = arg1.get(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_get_2080c49015e35254: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_635eea311fa1a291: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_955bde2b2fe2defa: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_d7176cfb76d71114: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_index_ff57aa2f75d9456b: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_unchecked_e6325ad374eff861: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_with_ref_key_fc601247060b7204: function(arg0, arg1) {
            const ret = arg0[arg1];
            return ret;
        },
        __wbg_history_7236bed7324e63c0: function() { return handleError(function (arg0) {
            const ret = arg0.history;
            return ret;
        }, arguments); },
        __wbg_host_64d73c152c626ec8: function(arg0) {
            const ret = arg0.host;
            return ret;
        },
        __wbg_href_436d1661275da591: function(arg0, arg1) {
            const ret = arg1.href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_href_a0128fb7c568d8e5: function() { return handleError(function (arg0, arg1) {
            const ret = arg1.href;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_info_ef9e889444913d2f: function(arg0) {
            console.info(arg0);
        },
        __wbg_insertBefore_10ab0c19b463c12b: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.insertBefore(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_instanceof_ArrayBuffer_12cdaa4161423e6b: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ArrayBuffer;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Element_05dbb22ded594343: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Element;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Error_74e3cca0c4a69161: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Error;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlAnchorElement_fee0a0613fd8e6aa: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLAnchorElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Node_01bbf5179851b46b: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Node;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_ShadowRoot_cd78159e03fed884: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ShadowRoot;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Uint8Array_d7aeb0dcb3108fea: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Uint8Array;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_5cf453f5639aa1a2: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isArray_bcf1376d0d732d08: function(arg0) {
            const ret = Array.isArray(arg0);
            return ret;
        },
        __wbg_isConnected_c529655b02d009e7: function(arg0) {
            const ret = arg0.isConnected;
            return ret;
        },
        __wbg_isSafeInteger_1a35ba4d31e71d38: function(arg0) {
            const ret = Number.isSafeInteger(arg0);
            return ret;
        },
        __wbg_isTrusted_ee28e0e4c56ca6e0: function(arg0) {
            const ret = arg0.isTrusted;
            return ret;
        },
        __wbg_iterator_e4c5e8f68ada7b58: function() {
            const ret = Symbol.iterator;
            return ret;
        },
        __wbg_key_76125a528069feff: function(arg0, arg1) {
            const ret = arg1.key;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_length_186163212ffe79c3: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_694310723930d77e: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_b03a0fabb109b1e9: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_localStorage_6827cfe6db14a9f6: function() { return handleError(function (arg0) {
            const ret = arg0.localStorage;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_location_7320782274fc691b: function(arg0) {
            const ret = arg0.location;
            return ret;
        },
        __wbg_log_ff86bbb34c295e21: function(arg0) {
            console.log(arg0);
        },
        __wbg_matchMedia_b53687d22ec43649: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.matchMedia(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_matches_a47c420646498c64: function(arg0) {
            const ret = arg0.matches;
            return ret;
        },
        __wbg_message_43d8c8f1d543a349: function(arg0) {
            const ret = arg0.message;
            return ret;
        },
        __wbg_message_4f4d0f24ef6c18e6: function(arg0, arg1) {
            const ret = arg1.message;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_name_f721590be23206c9: function(arg0, arg1) {
            const ret = arg1.name;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_navigator_c17f443a6fe5d38d: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_new_13de428334042d26: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_37eb45aaf5eeb820: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_787b9fb330d55d78: function(arg0, arg1) {
            const ret = new Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_96baed0986e0d99f: function() { return handleError(function (arg0, arg1) {
            const ret = new URL(getStringFromWasm0(arg0, arg1));
            return ret;
        }, arguments); },
        __wbg_new_bb74001155cb8ca8: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_bdf613c6b0b3d702: function() { return handleError(function (arg0) {
            const ret = new ResizeObserver(arg0);
            return ret;
        }, arguments); },
        __wbg_new_cd4348290afae586: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_typed_e09da9dae6ee5935: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___js_sys_acc8bf69ddac3ab___Function_fn_wasm_bindgen_2af58f5718f9baab___JsValue_____wasm_bindgen_2af58f5718f9baab___sys__Undefined___js_sys_acc8bf69ddac3ab___Function_fn_wasm_bindgen_2af58f5718f9baab___JsValue_____wasm_bindgen_2af58f5718f9baab___sys__Undefined_______true_(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_with_byte_offset_and_length_3cd32055a417f095: function(arg0, arg1, arg2) {
            const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_new_with_options_4e2ed14ce2ee51dd: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = new Worker(getStringFromWasm0(arg0, arg1), arg2);
            return ret;
        }, arguments); },
        __wbg_new_with_u8_array_sequence_and_options_ca80bf9b8bbc67cf: function() { return handleError(function (arg0, arg1) {
            const ret = new Blob(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_next_178c1651c11432b3: function(arg0) {
            const ret = arg0.next;
            return ret;
        },
        __wbg_next_af5ac6a08bb0a717: function() { return handleError(function (arg0) {
            const ret = arg0.next();
            return ret;
        }, arguments); },
        __wbg_now_463ac581d2fc7ea3: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_observe_0bd2cd721b4e32da: function(arg0, arg1) {
            arg0.observe(arg1);
        },
        __wbg_off_683988a497cece2e: function(arg0, arg1, arg2, arg3) {
            arg0.off(getStringFromWasm0(arg1, arg2), arg3);
        },
        __wbg_offset_629728e7b8e23c70: function(arg0) {
            const ret = globalThis.FloatingUIDOM.offset(arg0);
            return ret;
        },
        __wbg_on_6376a628f7583d99: function(arg0, arg1, arg2, arg3) {
            arg0.on(getStringFromWasm0(arg1, arg2), arg3);
        },
        __wbg_packZip_1540737e43c29de9: function() { return handleError(function (arg0) {
            const ret = SQLightArchive.packZip(arg0);
            return ret;
        }, arguments); },
        __wbg_parentNode_fe15bcc26df42856: function(arg0) {
            const ret = arg0.parentNode;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_postMessage_61772322081d1756: function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments); },
        __wbg_preventDefault_cfb06b02e7406899: function(arg0) {
            arg0.preventDefault();
        },
        __wbg_prototypesetcall_13162965d1fe7aa3: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_push_c9f4893fc209f768: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_queueMicrotask_1e8503a8a690f0a3: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_4a1a4d8dd332373b: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_random_df36b90cd78a7d54: function() {
            const ret = Math.random();
            return ret;
        },
        __wbg_removeAttribute_735f25a801a701e1: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.removeAttribute(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_removeChild_d2dc6fc1d6db896b: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.removeChild(arg1);
            return ret;
        }, arguments); },
        __wbg_removeEventListener_4e20bd5fac3de092: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.removeEventListener(getStringFromWasm0(arg1, arg2), arg3);
        }, arguments); },
        __wbg_removeProperty_24e2a492b137f2d7: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            const ret = arg1.removeProperty(getStringFromWasm0(arg2, arg3));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        }, arguments); },
        __wbg_remove_8228e91d7ddbc755: function(arg0) {
            arg0.remove();
        },
        __wbg_remove_b28d816f9e896ff9: function(arg0) {
            arg0.remove();
        },
        __wbg_replaceState_1dad4255f11a05e4: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.replaceState(arg1, getStringFromWasm0(arg2, arg3), arg4 === 0 ? undefined : getStringFromWasm0(arg4, arg5));
        }, arguments); },
        __wbg_resize_0aaf8be885b44d9c: function(arg0) {
            arg0.resize();
        },
        __wbg_resolve_dbf78ab4942478b0: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_respond_d97c023ee981d1f5: function() { return handleError(function (arg0, arg1) {
            arg0.respond(arg1 >>> 0);
        }, arguments); },
        __wbg_revokeObjectURL_52940372bc4245cf: function() { return handleError(function (arg0, arg1) {
            URL.revokeObjectURL(getStringFromWasm0(arg0, arg1));
        }, arguments); },
        __wbg_searchParams_1e9bb9820401be23: function(arg0) {
            const ret = arg0.searchParams;
            return ret;
        },
        __wbg_setAttribute_f75317b2b5cabfb4: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setItem_cfa3cb31f271776a: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setKeyboardHandler_b2976eac1ce4850b: function() { return handleError(function (arg0, arg1) {
            arg0.setKeyboardHandler(arg1);
        }, arguments); },
        __wbg_setProperty_3fe74e09d53acb0b: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setTheme_b5e00c3c3748170c: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.setTheme(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_setTimeout_9ef22b48643bd7de: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.setTimeout(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_setValue_b3180804ef8b815f: function(arg0, arg1, arg2, arg3) {
            arg0.setValue(getStringFromWasm0(arg1, arg2), arg3);
        },
        __wbg_setWorkspaceValue_dce4dc2a74817bee: function(arg0, arg1, arg2) {
            setWorkspaceValue(arg0, getStringFromWasm0(arg1, arg2));
        },
        __wbg_set_2793479888329cb6: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_a7340d5a40337463: function(arg0, arg1, arg2, arg3, arg4) {
            arg0.set(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        },
        __wbg_set_className_fcd90345b3a090af: function(arg0, arg1, arg2) {
            arg0.className = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_d2d33b52458e984a: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_set_download_8e0ceaaeb91cf778: function(arg0, arg1, arg2) {
            arg0.download = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_e4b9a833d288759a: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_set_feb3714f475485ba: function(arg0, arg1, arg2) {
            arg0.set(getArrayU8FromWasm0(arg1, arg2));
        },
        __wbg_set_href_0f986799cac24892: function(arg0, arg1, arg2) {
            arg0.href = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_innerHTML_a9d3a267967b5117: function(arg0, arg1, arg2) {
            arg0.innerHTML = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_mode_92c8cba20318a2e1: function(arg0, arg1) {
            arg0.mode = __wbindgen_enum_ShadowRootMode[arg1];
        },
        __wbg_set_nodeValue_7e9217115013d873: function(arg0, arg1, arg2) {
            arg0.nodeValue = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_onerror_8346fd6e80c876c8: function(arg0, arg1) {
            arg0.onerror = arg1;
        },
        __wbg_set_onmessage_fb42c85f5237566a: function(arg0, arg1) {
            arg0.onmessage = arg1;
        },
        __wbg_set_type_4308a9d5a5949ede: function(arg0, arg1) {
            arg0.type = __wbindgen_enum_WorkerType[arg1];
        },
        __wbg_set_type_609b0cea6429d7bb: function(arg0, arg1, arg2) {
            arg0.type = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_value_cb7e77c93a95e491: function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        },
        __wbg_shift_af58ac583e389691: function() {
            const ret = globalThis.FloatingUIDOM.shift();
            return ret;
        },
        __wbg_size_9d1084e29743663c: function(arg0) {
            const ret = arg0.size;
            return ret;
        },
        __wbg_stack_575c4e0918581b9c: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_state_05c2560b2c0c4f58: function() { return handleError(function (arg0) {
            const ret = arg0.state;
            return ret;
        }, arguments); },
        __wbg_static_accessor_GLOBAL_THIS_003ee833af9699fc: function() {
            const ret = typeof globalThis === 'undefined' ? undefined : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_e4293fb22cbe77da: function() {
            const ret = typeof global === 'undefined' ? undefined : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_53bb20457401453d: function() {
            const ret = typeof self === 'undefined' ? undefined : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_b5e0810bfecb7feb: function() {
            const ret = typeof window === 'undefined' ? undefined : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_style_f499e599ac70277f: function(arg0) {
            const ret = arg0.style;
            return ret;
        },
        __wbg_target_e5cd9c7aa87dd8df: function(arg0) {
            const ret = arg0.target;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_terminate_72951fee87f2d80a: function(arg0) {
            arg0.terminate();
        },
        __wbg_then_656f253a5dd7d8ea: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_e3f7f2d365708be8: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_unpackZip_16c6854119c393cf: function() { return handleError(function (arg0) {
            const ret = SQLightArchive.unpackZip(arg0);
            return ret;
        }, arguments); },
        __wbg_value_df0a0e29a4d182fe: function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 1, len1, true);
            getDataViewMemory0().setInt32((arg0 >>> 0) + 4 * 0, ptr1, true);
        },
        __wbg_value_e0f328965e184931: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_view_3b708d3bdaf3fc96: function(arg0) {
            const ret = arg0.view;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_warn_228ff2257384d295: function(arg0) {
            console.warn(arg0);
        },
        __wbg_writeText_6b6a85b68a7dd3eb: function(arg0, arg1, arg2) {
            const ret = arg0.writeText(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 83, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true_);
            return ret;
        },
        __wbindgen_generic_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 89, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue__core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_2af58f5718f9baab___JsError___true_);
            return ret;
        },
        __wbindgen_generic_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("ErrorEvent")], shim_idx: 83, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true__13);
            return ret;
        },
        __wbindgen_generic_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("Event")], shim_idx: 83, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true__14);
            return ret;
        },
        __wbindgen_generic_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("MessageEvent")], shim_idx: 83, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true__15);
            return ret;
        },
        __wbindgen_generic_0000000000000006: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 134, ret: Unit, inner_ret: Some(Unit) }, mutable: false }) -> Externref`.
            const ret = makeClosure(arg0, arg1, wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke_______true_);
            return ret;
        },
        __wbindgen_generic_0000000000000007: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [], shim_idx: 170, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke_______true__1_);
            return ret;
        },
        __wbindgen_generic_0000000000000008: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_generic_0000000000000009: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_generic_000000000000000a: function(arg0) {
            // Cast intrinsic for `U64 -> Externref`.
            const ret = BigInt.asUintN(64, arg0);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            table.set(0, undefined);
            table.set(1024, undefined);
            table.set(1025, null);
            table.set(1026, true);
            table.set(1027, false);
        },
    };
    return {
        __proto__: null,
        "./app_bg.js": import0,
        "./snippets/floating-ui-fe1d6f75754df512/inline0.js": import1,
        "./snippets/aceditor-5e0aadc1546f54d8/inline0.js": import2,
    };
}

function wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke_______true_(arg0, arg1) {
    wasm.wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke_______true_(arg0, arg1);
}

function wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke_______true__1_(arg0, arg1) {
    wasm.wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke_______true__1_(arg0, arg1);
}

function wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true_(arg0, arg1, arg2) {
    wasm.wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true_(arg0, arg1, arg2);
}

function wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true__13(arg0, arg1, arg2) {
    wasm.wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true__13(arg0, arg1, arg2);
}

function wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true__14(arg0, arg1, arg2) {
    wasm.wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true__14(arg0, arg1, arg2);
}

function wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true__15(arg0, arg1, arg2) {
    wasm.wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue______true__15(arg0, arg1, arg2);
}

function wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue__core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_2af58f5718f9baab___JsError___true_(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___wasm_bindgen_2af58f5718f9baab___JsValue__core_37ee0ce0c4cd57d5___result__Result_____wasm_bindgen_2af58f5718f9baab___JsError___true_(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___js_sys_acc8bf69ddac3ab___Function_fn_wasm_bindgen_2af58f5718f9baab___JsValue_____wasm_bindgen_2af58f5718f9baab___sys__Undefined___js_sys_acc8bf69ddac3ab___Function_fn_wasm_bindgen_2af58f5718f9baab___JsValue_____wasm_bindgen_2af58f5718f9baab___sys__Undefined_______true_(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen_2af58f5718f9baab___convert__closures_____invoke___js_sys_acc8bf69ddac3ab___Function_fn_wasm_bindgen_2af58f5718f9baab___JsValue_____wasm_bindgen_2af58f5718f9baab___sys__Undefined___js_sys_acc8bf69ddac3ab___Function_fn_wasm_bindgen_2af58f5718f9baab___JsValue_____wasm_bindgen_2af58f5718f9baab___sys__Undefined_______true_(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_ReadableStreamType = ["bytes"];


const __wbindgen_enum_ShadowRootMode = ["open", "closed"];


const __wbindgen_enum_WorkerType = ["classic", "module"];
const IntoUnderlyingByteSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingbytesource_free(ptr, 1));
const IntoUnderlyingSinkFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsink_free(ptr, 1));
const IntoUnderlyingSourceFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_intounderlyingsource_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    switch (typeof val) {
        // primitive types
        case 'number':
        case 'boolean':
        case 'undefined':
        return `${val}`;
        case 'string':
        return JSON.stringify(val);
        case 'bigint':
        return `${val}n`;
        case 'symbol':
        return val.description !== undefined
            ? `Symbol(${val.description})`
            : 'Symbol';
        case 'function':
        return typeof val.name === 'string' && val.name !== ''
            ? `Function(${val.name})`
            : 'Function';
        case 'object': {
            if (val === null) {
                return 'null';
            }
            // objects
            if (Array.isArray(val)) {
                return '[' + Array.from(val, debugString).join(', ') + ']';
            }
            // errors
            if (val instanceof Error) {
                return `${val.name}: ${val.message}\n${val.stack}`;
            }

            const toStringOutput = toString.call(val);
            // Test for built-in
            const className = /\[object ([^\]]+)\]/.exec(toStringOutput)?.[1];

            // Failed to match the standard '[object ClassName]'
            if (className === undefined) {
                return toStringOutput;
            }
            if (className === 'Object') {
                // we're a user defined class or Object
                // JSON.stringify avoids problems with cycles, and is generally much
                // easier than looping through ownProperties of `val`.
                try {
                    return 'Object(' + JSON.stringify(val) + ')';
                } catch (_) {
                    return 'Object';
                }
            }
            // TODO we could test for more things here, like `Set`s and `Map`s.
            return className;
        }
        // Per MDN: 'This list of values is exhaustive.'
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/typeof#description
    }
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        try {
            return f(state.a, state.b, ...args);
        } finally {
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(bytes) {
    numBytesDecoded += bytes.length;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true });
        cachedTextDecoder.decode();
        numBytesDecoded = bytes.length;
    }
    return cachedTextDecoder.decode(bytes);
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

let __wbg_init_promise;
async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;
    if (__wbg_init_promise === undefined) {
        __wbg_init_promise = __wbg_init_once(module_or_path);
        __wbg_init_promise.catch(() => { __wbg_init_promise = undefined; });
    }
    return __wbg_init_promise;
}

async function __wbg_init_once(module_or_path) {

    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('app_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
