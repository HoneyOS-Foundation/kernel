/// Extracts current script file path from artificially generated stack trace
/// Adapted from: https://github.com/chemicstry/wasm_thread/blob/v0.3.0/src/wasm32/js/script_path.js
function script_path() {
    try {
        throw new Error();
    } catch (e) {
        let parts = e.stack.match(/(?:\(|@)(\S+):\d+:\d+/);
        return parts[1];
    }
}

script_path()