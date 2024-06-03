import init, {create_thread_instance} from "BINDGEN_SHIM_URL";

self.onmessage = event => {
    const [pid, kernel, kernel_memory, module, memory, f_ptr] = event.data;

    init(kernel, kernel_memory).catch(err => {
        console.error("Failed to initialize module: " + err);
        throw err;
    }).then(async () => {
        let instance = await create_thread_instance(pid, module, memory);

        // Make sure the module exports the function table
        if (!Object.hasOwn(instance.exports, '__indirect_function_table')) {
            console.error("Wasm module must have it's table exported as `__indirect_function_table` in order to support multithreading.");
            close();
            return;
        }

        // Make sure the function pointer is valid
        if (instance.exports.__indirect_function_table.length < f_ptr) {
            console.error("The function pointer `" + f_ptr + "` is invalid");
            close();
            return;
        }
        instance.exports.__indirect_function_table.get(f_ptr)();
        close();
    })
}