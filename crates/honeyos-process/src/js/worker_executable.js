import init, {create_instance} from "BINDGEN_SHIM_URL";

self.onmessage = event => {
    const [pid, kernel, kernel_memory, module] = event.data;

    init(kernel, kernel_memory).catch(err => {
        console.error("Failed to initialize module: " + err);
        throw err;
    }).then(async () => {
        let instance = await create_instance(pid, module);
        instance.exports._start();
        close();
    })
}