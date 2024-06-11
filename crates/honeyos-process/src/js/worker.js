import init, {create_instance} from "BINDGEN_SHIM_URL";

self.onmessage = event => {
    self.onmessage = null; // Prevent eval from reading onmessage
    const [pid, kernel, kernel_memory, memory, f_ptr] = event.data;

    init(kernel, kernel_memory).catch(err => {
        console.error("Failed to initialize module: " + err);
        throw err;
    }).then(async () => {
        try {
            const table = new WebAssembly.Table({
                initial: 4,
                element: "anyfunc"
            });
            let instance = await create_instance(pid, memory, table);
            instance.exports._thread_entrypoint(f_ptr);
            postMessage({}); // Tell the kernel the process is dead
            close();
        }
        catch (e) {
            console.error(e);
            close();
        }
    })
}