use std::{ffi::CString, str::FromStr, sync::Arc};

use honeyos_networking::{
    request::{RequestMethod, RequestMode, RequestStatus},
    NetworkingManager,
};
use honeyos_process::api::{ApiModuleBuilder, ApiModuleCtx};
use uuid::Uuid;
use wasm_bindgen::closure::Closure;

/// Register the network api
pub fn register_network_api(ctx: Arc<ApiModuleCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_network_request
    // Create a network request and return it's id.
    // ### Returns:
    // - The id of the request on sucess
    // - NULL if the request method was invalid, or when failed to parse headers as json.
    // ### Methods:
    // - Get = 0
    // - Head = 1
    // - Post = 2
    // - Put = 3
    // - Delete = 4
    // - Connect = 5
    // - Options = 6
    // - Trace = 7
    // - Patch = 8
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_network_request",
        Closure::<dyn Fn(*const u8, u32, *const u8) -> *const u8>::new(
            move |url, method, headers| {
                // Read params
                let mut memory = ctx_f.memory();
                let url = memory.read_str(url as u32);
                let Ok(method) = RequestMethod::try_from(method) else {
                    return std::ptr::null();
                };
                let headers = memory.read_str(headers as u32);

                // Setup request
                let mut network_manager = NetworkingManager::blocking_get();
                let id = network_manager.request(url, method, RequestMode::Cors, headers);

                // Write id to memory
                let id = id.to_string();
                let Some(id_ptr) = memory.alloc(id.len() as u32) else {
                    return std::ptr::null();
                };
                let cstring = CString::new(id).unwrap();
                memory.write(id_ptr, cstring.as_bytes());

                id_ptr as *const u8
            },
        )
        .into_js_value(),
    );

    // hapi_network_request_local
    // Create a network request to the local server and return it's id.
    // ### Returns:
    // - The id of the request on sucess
    // - NULL if the request method was invalid, or when failed to parse headers as json.
    // ### Methods:
    // - Get = 0
    // - Head = 1
    // - Post = 2
    // - Put = 3
    // - Delete = 4
    // - Connect = 5
    // - Options = 6
    // - Trace = 7
    // - Patch = 8
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_network_request_local",
        Closure::<dyn Fn(*const u8, u32, *const u8) -> *const u8>::new(
            move |url, method, headers| {
                // Read params
                let mut memory = ctx_f.memory();
                let url = memory.read_str(url as u32);
                let Ok(method) = RequestMethod::try_from(method) else {
                    return std::ptr::null();
                };
                let headers = memory.read_str(headers as u32);

                // Setup request
                let mut network_manager = NetworkingManager::blocking_get();
                let id = network_manager.request(url, method, RequestMode::SameOrigin, headers);

                // Write id to memory
                let id = id.to_string();
                let Some(id_ptr) = memory.alloc(id.len() as u32) else {
                    return std::ptr::null();
                };
                let cstring = CString::new(id).unwrap();
                memory.write(id_ptr, cstring.as_bytes());

                id_ptr as *const u8
            },
        )
        .into_js_value(),
    );

    // hapi_network_request_status
    // Check the status of the request
    // ### Returns
    // - `-1` if the request does not exists.
    // - `0`if the request is pending.
    // - `1`if the request succeeded.
    // - `2`if the request failed.
    // - `3`if the request is still pending
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_network_request_status",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |id| {
            let memory = ctx_f.memory();
            let id = memory.read_str(id as u32);
            let Ok(id) = Uuid::from_str(&id) else {
                return -1;
            };

            let networking_manager = NetworkingManager::blocking_get();
            let Some(status) = networking_manager.status(id) else {
                return -1;
            };
            match status {
                RequestStatus::Processing => 0,
                RequestStatus::Success => 1,
                RequestStatus::Fail => 2,
                RequestStatus::Pending => 3,
            }
        })
        .into_js_value(),
    );

    // hapi_network_request_data_length
    // Check the lenght of the data in bytes.
    // ### Returns
    // - The data length on success
    // - -1 if the request does not exist.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_network_request_data_length",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |id| {
            let memory = ctx_f.memory();
            let id = memory.read_str(id as u32);
            let Ok(id) = Uuid::from_str(&id) else {
                return -1;
            };

            let networking_manager = NetworkingManager::blocking_get();
            let Some(length) = networking_manager.data_length(id) else {
                return -1;
            };

            length as i32
        })
        .into_js_value(),
    );

    // hapi_network_request_data
    // Check the data in a request
    // ### Returns
    // - The data on success
    // - NULL if the request does not exist,
    // - NULL if the request has failed or is still pending,
    // - NULL if the memory allocation failed.
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_network_request_data",
        Closure::<dyn Fn(*const u8) -> *const u8>::new(move |id| {
            let mut memory = ctx_f.memory();
            let id = memory.read_str(id as u32);
            let Ok(id) = Uuid::from_str(&id) else {
                return std::ptr::null();
            };

            let networking_manager = NetworkingManager::blocking_get();
            let Some(data) = networking_manager.data(id) else {
                return std::ptr::null();
            };

            let Some(ptr) = memory.alloc(data.len() as u32) else {
                return std::ptr::null();
            };
            memory.write(ptr, &data);

            ptr as *const u8
        })
        .into_js_value(),
    );

    // hapi_network_request_drop
    // Drop the request from memory.
    // Does nothing if the request does not exist
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_network_request_drop",
        Closure::<dyn Fn(*const u8)>::new(move |id| {
            let memory = ctx_f.memory();
            let id = memory.read_str(id as u32);
            let Ok(id) = Uuid::from_str(&id) else {
                return;
            };

            let mut networking_manager = NetworkingManager::blocking_get();
            networking_manager.remove(id);
        })
        .into_js_value(),
    );
}
