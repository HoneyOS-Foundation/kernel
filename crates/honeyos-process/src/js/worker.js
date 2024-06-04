import init, {create_thread_instance} from "BINDGEN_SHIM_URL";

self.onmessage = event => {
    const [pid, kernel, kernel_memory, module, memory, f_ptr] = event.data;

    init(kernel, kernel_memory).catch(err => {
        console.error("Failed to initialize module: " + err);
        throw err;
    }).then(async () => {
        let instance = await create_thread_instance(pid, module, memory);

        // Find a table in exports
        let table = undefined;
        for (const key in instance.exports) {
            const value = instance.exports[key];
            if (value.toString() === '[object WebAssembly.Table]') {
                table = value;
            }
        }

        // Fail if no table exported
        if (table === undefined) {
            console.error("Wasm binary must export table for multithreading support.");
            close();
            return;
        }

        // Make sure the function pointer is valid
        if (table.length < f_ptr) {
            console.error("The function pointer `" + f_ptr + "` is invalid");
            close();
            return;
        }
        const function_ptr = table.get(f_ptr);
        function_ptr();
        close();
    })
}