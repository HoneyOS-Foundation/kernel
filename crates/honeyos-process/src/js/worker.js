import init, {create_thread_instance} from "BINDGEN_SHIM_URL";

self.onmessage = event => {
    const [pid, kernel, kernel_memory, module, memory, f_ptr] = event.data;

    init(kernel, kernel_memory).catch(err => {
        console.error("Failed to initialize module: " + err);
        throw err;
    }).then(async () => {
        try {
            let instance = await create_thread_instance(pid, module, memory);
            instance.exports._thread_entrypoint(f_ptr);
            close();
        }
        catch (e) {
            console.error(e);
            close();
        }
    })
}