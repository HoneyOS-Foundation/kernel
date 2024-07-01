use std::{ffi::CString, str::FromStr, sync::Arc};

use honeyos_atomics::rwlock::SpinRwLock;
use honeyos_networking::{
    request::{RequestMethod, RequestMode, RequestStatus},
    NetworkingManager,
};
use honeyos_process::context::{ApiModuleBuilder, ProcessCtx};
use uuid::Uuid;
use wasm_bindgen::closure::Closure;

/// Register the network api
pub fn register_network_api(ctx: Arc<ProcessCtx>, builder: &mut ApiModuleBuilder) {
    // hapi_network_request
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_network_request",
        Closure::<dyn Fn(*const u8, u32, *const u8) -> *const u8>::new(
            move |url, method, headers| {
                // Read params
                let mut memory = ctx_f.memory();
                let url = memory.read_str(url as u32);
                let Some(url) = url else {
                    return std::ptr::null();
                };
                let Ok(method) = RequestMethod::try_from(method) else {
                    return std::ptr::null();
                };
                let headers = memory.read_str(headers as u32);
                let Some(headers) = headers else {
                    return std::ptr::null();
                };

                // Setup request
                let networking_manager_lock = NetworkingManager::get();
                let mut networking_manager = networking_manager_lock.spin_write().unwrap();
                let id = networking_manager.request(url, method, RequestMode::Cors, headers);

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
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_network_request_local",
        Closure::<dyn Fn(*const u8, u32, *const u8) -> *const u8>::new(
            move |url, method, headers| {
                // Read params
                let mut memory = ctx_f.memory();
                let url = memory.read_str(url as u32);
                let Some(url) = url else {
                    return std::ptr::null();
                };
                let Ok(method) = RequestMethod::try_from(method) else {
                    return std::ptr::null();
                };
                let headers = memory.read_str(headers as u32);
                let Some(headers) = headers else {
                    return std::ptr::null();
                };

                // Setup request
                let networking_manager_lock = NetworkingManager::get();
                let mut networking_manager = networking_manager_lock.spin_write().unwrap();
                let id = networking_manager.request(url, method, RequestMode::SameOrigin, headers);

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
    let ctx_f = ctx.clone();
    builder.register(
        "hapi_network_request_status",
        Closure::<dyn Fn(*const u8) -> i32>::new(move |id| {
            let memory = ctx_f.memory();
            let id = memory.read_str(id as u32);
            let Some(id) = id else {
                return -1;
            };
            let Ok(id) = Uuid::from_str(&id) else {
                return -1;
            };

            let networking_manager_lock = NetworkingManager::get();
            let networking_manager = networking_manager_lock.spin_read().unwrap();
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
            let Some(id) = id else {
                return -1;
            };
            let Ok(id) = Uuid::from_str(&id) else {
                return -1;
            };

            let networking_manager_lock = NetworkingManager::get();
            let networking_manager = networking_manager_lock.spin_read().unwrap();
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
            let Some(id) = id else {
                return std::ptr::null();
            };
            let Ok(id) = Uuid::from_str(&id) else {
                return std::ptr::null();
            };

            let networking_manager_lock = NetworkingManager::get();
            let networking_manager = networking_manager_lock.spin_read().unwrap();
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
            let Some(id) = id else {
                return;
            };
            let Ok(id) = Uuid::from_str(&id) else {
                return;
            };

            let networking_manager_lock = NetworkingManager::get();
            let mut networking_manager = networking_manager_lock.spin_write().unwrap();
            networking_manager.remove(id);
        })
        .into_js_value(),
    );
}
