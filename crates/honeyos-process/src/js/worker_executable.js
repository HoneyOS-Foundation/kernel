import init, {create_instance} from "BINDGEN_SHIM_URL";

self.onmessage = event => {
    self.onmessage = null;
    const [pid, kernel, kernel_memory, memory] = event.data;

    init(kernel, kernel_memory).catch(err => {
        console.error("Failed to initialize module: " + err);
        throw err;
    }).then(async () => {
        const table = new WebAssembly.Table({
            initial: 4,
            element: "anyfunc"
        });
        let instance = await create_instance(pid, memory, table);
        instance.exports._start();
        postMessage({}); // Tell the kernel the process is dead
        close();
    })
}