let wasm_bindgen;
(function() {
    const __exports = {};
    let script_src;
    if (typeof document !== 'undefined' && document.currentScript !== null) {
        script_src = new URL(document.currentScript.src, location.href).toString();
    }
    let wasm = undefined;

    const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

    if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

    let cachedUint8ArrayMemory0 = null;

    function getUint8ArrayMemory0() {
        if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
            cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
        }
        return cachedUint8ArrayMemory0;
    }

    function getStringFromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
    }

    let cachedFloat64ArrayMemory0 = null;

    function getFloat64ArrayMemory0() {
        if (cachedFloat64ArrayMemory0 === null || cachedFloat64ArrayMemory0.byteLength === 0) {
            cachedFloat64ArrayMemory0 = new Float64Array(wasm.memory.buffer);
        }
        return cachedFloat64ArrayMemory0;
    }

    function getArrayF64FromWasm0(ptr, len) {
        ptr = ptr >>> 0;
        return getFloat64ArrayMemory0().subarray(ptr / 8, ptr / 8 + len);
    }

    let cachedUint32ArrayMemory0 = null;

    function getUint32ArrayMemory0() {
        if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
            cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
        }
        return cachedUint32ArrayMemory0;
    }

    let WASM_VECTOR_LEN = 0;

    function passArray32ToWasm0(arg, malloc) {
        const ptr = malloc(arg.length * 4, 4) >>> 0;
        getUint32ArrayMemory0().set(arg, ptr / 4);
        WASM_VECTOR_LEN = arg.length;
        return ptr;
    }

    const LifeUniverseFinalization = (typeof FinalizationRegistry === 'undefined')
        ? { register: () => {}, unregister: () => {} }
        : new FinalizationRegistry(ptr => wasm.__wbg_lifeuniverse_free(ptr >>> 0, 1));

    class LifeUniverse {

        __destroy_into_raw() {
            const ptr = this.__wbg_ptr;
            this.__wbg_ptr = 0;
            LifeUniverseFinalization.unregister(this);
            return ptr;
        }

        free() {
            const ptr = this.__destroy_into_raw();
            wasm.__wbg_lifeuniverse_free(ptr, 0);
        }
        clear_pattern() {
            wasm.lifeuniverse_clear_pattern(this.__wbg_ptr);
        }
        constructor() {
            const ret = wasm.lifeuniverse_new();
            this.__wbg_ptr = ret >>> 0;
            LifeUniverseFinalization.register(this, this.__wbg_ptr, this);
            return this;
        }
        save_rewind_state() {
            wasm.lifeuniverse_save_rewind_state(this.__wbg_ptr);
        }
        restore_rewind_state() {
            wasm.lifeuniverse_restore_rewind_state(this.__wbg_ptr);
        }
        /**
         * @returns {boolean}
         */
        has_rewind_state() {
            const ret = wasm.lifeuniverse_has_rewind_state(this.__wbg_ptr);
            return ret !== 0;
        }
        /**
         * @param {number} x
         * @param {number} y
         * @param {boolean} living
         */
        set_bit(x, y, living) {
            wasm.lifeuniverse_set_bit(this.__wbg_ptr, x, y, living);
        }
        /**
         * @param {number} x
         * @param {number} y
         * @returns {boolean}
         */
        get_bit(x, y) {
            const ret = wasm.lifeuniverse_get_bit(this.__wbg_ptr, x, y);
            return ret !== 0;
        }
        /**
         * @returns {Float64Array}
         */
        get_root_bounds() {
            const ret = wasm.lifeuniverse_get_root_bounds(this.__wbg_ptr);
            var v1 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
            return v1;
        }
        /**
         * @param {boolean} is_single
         */
        next_generation(is_single) {
            wasm.lifeuniverse_next_generation(this.__wbg_ptr, is_single);
        }
        /**
         * @param {Int32Array} field_x
         * @param {Int32Array} field_y
         */
        setup_field(field_x, field_y) {
            const ptr0 = passArray32ToWasm0(field_x, wasm.__wbindgen_malloc);
            const len0 = WASM_VECTOR_LEN;
            const ptr1 = passArray32ToWasm0(field_y, wasm.__wbindgen_malloc);
            const len1 = WASM_VECTOR_LEN;
            wasm.lifeuniverse_setup_field(this.__wbg_ptr, ptr0, len0, ptr1, len1);
        }
        /**
         * @returns {number}
         */
        get_step() {
            const ret = wasm.lifeuniverse_get_step(this.__wbg_ptr);
            return ret >>> 0;
        }
        /**
         * @param {number} step
         */
        set_step(step) {
            wasm.lifeuniverse_set_step(this.__wbg_ptr, step);
        }
        /**
         * @param {number} s
         * @param {number} b
         */
        set_rules(s, b) {
            wasm.lifeuniverse_set_rules(this.__wbg_ptr, s, b);
        }
        /**
         * @param {number} x
         * @param {number} y
         * @param {number} size
         * @param {number} height
         * @param {number} width
         * @param {number} offset_x
         * @param {number} offset_y
         * @returns {Float64Array}
         */
        draw(x, y, size, height, width, offset_x, offset_y) {
            const ret = wasm.lifeuniverse_draw(this.__wbg_ptr, x, y, size, height, width, offset_x, offset_y);
            var v1 = getArrayF64FromWasm0(ret[0], ret[1]).slice();
            wasm.__wbindgen_free(ret[0], ret[1] * 8, 8);
            return v1;
        }
        /**
         * @returns {number}
         */
        get_generation() {
            const ret = wasm.lifeuniverse_get_generation(this.__wbg_ptr);
            return ret;
        }
        /**
         * @returns {number}
         */
        get_population() {
            const ret = wasm.lifeuniverse_get_population(this.__wbg_ptr);
            return ret >>> 0;
        }
        /**
         * @returns {number}
         */
        get_level() {
            const ret = wasm.lifeuniverse_get_level(this.__wbg_ptr);
            return ret >>> 0;
        }
    }
    __exports.LifeUniverse = LifeUniverse;

    async function __wbg_load(module, imports) {
        if (typeof Response === 'function' && module instanceof Response) {
            if (typeof WebAssembly.instantiateStreaming === 'function') {
                try {
                    return await WebAssembly.instantiateStreaming(module, imports);

                } catch (e) {
                    if (module.headers.get('Content-Type') != 'application/wasm') {
                        console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                    } else {
                        throw e;
                    }
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
    }

    function __wbg_get_imports() {
        const imports = {};
        imports.wbg = {};
        imports.wbg.__wbindgen_init_externref_table = function() {
            const table = wasm.__wbindgen_export_0;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
            ;
        };
        imports.wbg.__wbindgen_throw = function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        };

        return imports;
    }

    function __wbg_init_memory(imports, memory) {

    }

    function __wbg_finalize_init(instance, module) {
        wasm = instance.exports;
        __wbg_init.__wbindgen_wasm_module = module;
        cachedFloat64ArrayMemory0 = null;
        cachedUint32ArrayMemory0 = null;
        cachedUint8ArrayMemory0 = null;


        wasm.__wbindgen_start();
        return wasm;
    }

    function initSync(module) {
        if (wasm !== undefined) return wasm;


        if (typeof module !== 'undefined') {
            if (Object.getPrototypeOf(module) === Object.prototype) {
                ({module} = module)
            } else {
                console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
            }
        }

        const imports = __wbg_get_imports();

        __wbg_init_memory(imports);

        if (!(module instanceof WebAssembly.Module)) {
            module = new WebAssembly.Module(module);
        }

        const instance = new WebAssembly.Instance(module, imports);

        return __wbg_finalize_init(instance, module);
    }

    async function __wbg_init(module_or_path) {
        if (wasm !== undefined) return wasm;


        if (typeof module_or_path !== 'undefined') {
            if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
                ({module_or_path} = module_or_path)
            } else {
                console.warn('using deprecated parameters for the initialization function; pass a single object instead')
            }
        }

        if (typeof module_or_path === 'undefined' && typeof script_src !== 'undefined') {
            module_or_path = script_src.replace(/\.js$/, '_bg.wasm');
        }
        const imports = __wbg_get_imports();

        if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
            module_or_path = fetch(module_or_path);
        }

        __wbg_init_memory(imports);

        const { instance, module } = await __wbg_load(await module_or_path, imports);

        return __wbg_finalize_init(instance, module);
    }

    wasm_bindgen = Object.assign(__wbg_init, { initSync }, __exports);

})();
