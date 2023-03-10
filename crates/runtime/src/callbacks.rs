use wasmtime::{
    AsContext, Caller, Func, FuncType, Memory, StoreContext, Val, ValType,
};
use crate::environment_state::EnvironmentState;

pub(crate) fn guest_request_func(
    store: &mut wasmtime::Store<EnvironmentState>
) -> Func {
    let callback_type = FuncType::new(vec![ValType::I32, ValType::I32], vec![]);
    Func::new_async(store, callback_type, |mut caller, params, _results| {
        Box::new(async move {
            let op_ptr = params[0].i32();
            let ptr = params[1].i32();

            //  let host = caller.data();
            let host = caller.data();
            let invocation = host.get_guest_request();
            let memory = get_caller_memory(&mut caller);
            if let Some(inv) = invocation {
                write_bytes_to_memory(caller.as_context(), memory, ptr.unwrap(), &inv.msg);
                write_bytes_to_memory(
                    caller.as_context(),
                    memory,
                    op_ptr.unwrap(),
                    inv.operation.as_bytes(),
                );
            }
            Ok(())
        })
    })
}

pub(crate) fn console_log_func(
    store: &mut wasmtime::Store<EnvironmentState>
) -> Func {
    let callback_type = FuncType::new(vec![ValType::I32, ValType::I32], vec![]);

    Func::new_async(
        store,
        callback_type,
        |mut caller, params: &[Val], _results: &mut [Val]| {
            Box::new(async move {
                let ptr = params[0].i32();
                let len = params[1].i32();
                let memory = get_caller_memory(&mut caller);
                let vec =
                    get_vec_from_memory(caller.as_context(), memory, ptr.unwrap(), len.unwrap());

                let msg = std::str::from_utf8(&vec).unwrap();
                let host = caller.data();
                host.do_console_log(msg);
                Ok(())
            })
        },
    )
}

pub(crate) fn host_call_func(
    store: &mut wasmtime::Store<EnvironmentState>
) -> Func {
    let callback_type = FuncType::new(
        vec![
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
        ],
        vec![ValType::I32],
    );
    // let host:Arc<Mutex<EnvironmentState>>  = Arc::clone(&host);
    Func::new_async(
        store,
        callback_type,
        |mut caller, params: &[Val], results: &mut [Val]| {
            // let host:Arc<Mutex<EnvironmentState>> = host.clone();
            Box::new(async move {
                // let host:Arc<Mutex<EnvironmentState>>  = Arc::clone(&host);
                // let host:Arc<Mutex<EnvironmentState>> = host.clone();
                /*let id = {
                    let mut state = state.borrow_mut();
                    state.host_response = None;
                    state.host_error = None;
                    state.id
                }; */
                let memory = get_caller_memory(&mut caller);

                let bd_ptr = params[0].i32();
                let bd_len = params[1].i32();
                let ns_ptr = params[2].i32();
                let ns_len = params[3].i32();
                let op_ptr = params[4].i32();
                let op_len = params[5].i32();
                let ptr = params[6].i32();
                let len = params[7].i32();

                let vec =
                    get_vec_from_memory(caller.as_context(), memory, ptr.unwrap(), len.unwrap());
                let bd_vec = get_vec_from_memory(
                    caller.as_context(),
                    memory,
                    bd_ptr.unwrap(),
                    bd_len.unwrap(),
                );
                let bd = std::str::from_utf8(&bd_vec).unwrap();
                let ns_vec = get_vec_from_memory(
                    caller.as_context(),
                    memory,
                    ns_ptr.unwrap(),
                    ns_len.unwrap(),
                );
                let ns = std::str::from_utf8(&ns_vec).unwrap();
                let op_vec = get_vec_from_memory(
                    caller.as_context(),
                    memory,
                    op_ptr.unwrap(),
                    op_len.unwrap(),
                );
                let op = std::str::from_utf8(&op_vec).unwrap();
                //trace!("Guest {} invoking host operation", id, op);
                let data = caller.data();
                let id = data.id;
                let resource_table = data.resource_table.clone();
                let result = caller.data().do_host_call(id, bd, ns, op, &vec, resource_table).await;
                // let host = host.lock().unwrap();
                let result: Result<i32, Box<dyn std::error::Error>> = Ok(match result {
                    Ok(v) => {
                        data.set_host_response(v);
                        1
                    }
                    Err(e) => {
                        data.set_host_error(format!("{}", e));
                        0
                    }
                });
                // *host.host_response.write() = Some(v);
                // let result = host.do_host_call(bd, ns, op, &vec).await;
                if let Ok(r) = result {
                    results[0] = Val::I32(r);
                }
                Ok(())
            })
        },
    )
}

pub(crate) fn host_response_func(
    store: &mut wasmtime::Store<EnvironmentState>
) -> Func {
    let callback_type = FuncType::new(vec![ValType::I32], vec![]);
    Func::new_async(
        store,
        callback_type,
        |mut caller, params: &[Val], _results: &mut [Val]| {
            Box::new(async move {
                let host = caller.data();
                if let Some(ref e) = host.get_host_response() {
                    let memory = get_caller_memory(&mut caller);
                    let ptr = params[0].i32();
                    write_bytes_to_memory(caller.as_context(), memory, ptr.unwrap(), e);
                }
                Ok(())
            })
        },
    )
}

pub(crate) fn host_response_len_func(
    store: &mut wasmtime::Store<EnvironmentState>
) -> Func {
    let callback_type = FuncType::new(vec![], vec![ValType::I32]);

    Func::new_async(
        store,
        callback_type,
        |caller, _params: &[Val], results: &mut [Val]| {
            Box::new(async move {
                let host = caller.data();
                results[0] = Val::I32(match host.get_host_response() {
                    Some(ref r) => r.len() as _,
                    None => 0,
                });
                Ok(())
            })
        },
    )
}

pub(crate) fn guest_response_func(
    store: &mut wasmtime::Store<EnvironmentState>
) -> Func {
    let callback_type = FuncType::new(vec![ValType::I32, ValType::I32], vec![]);
    Func::new_async(
        store,
        callback_type,
        |mut caller, params: &[Val], _results: &mut [Val]| {
            Box::new(async move {
                let ptr = params[0].i32();
                let len = params[1].i32();
                let memory = get_caller_memory(&mut caller);
                let vec =
                    get_vec_from_memory(caller.as_context(), memory, ptr.unwrap(), len.unwrap());
                let host = caller.data();
                host.set_guest_response(vec);
                Ok(())
            })
        },
    )
}

pub(crate) fn guest_error_func(
    store: &mut wasmtime::Store<EnvironmentState>
) -> Func {
    let callback_type = FuncType::new(vec![ValType::I32, ValType::I32], vec![]);
    Func::new_async(
        store,
        callback_type,
        |mut caller, params: &[Val], _results: &mut [Val]| {
            Box::new(async move {
                let memory = get_caller_memory(&mut caller);
                let ptr = params[0].i32();
                let len = params[1].i32();

                let vec =
                    get_vec_from_memory(caller.as_context(), memory, ptr.unwrap(), len.unwrap());
                let host = caller.data();
                host.set_guest_error(String::from_utf8(vec).unwrap());
                Ok(())
            })
        },
    )
}

pub(crate) fn host_error_func(
    store: &mut wasmtime::Store<EnvironmentState>
) -> Func {
    let callback_type = FuncType::new(vec![ValType::I32], vec![]);
    Func::new_async(
        store,
        callback_type,
        |mut caller, params: &[Val], _results: &mut [Val]| {
            Box::new(async move {
                let host = caller.data();
                if let Some(ref e) = host.get_host_error() {
                    let ptr = params[0].i32();
                    let memory = get_caller_memory(&mut caller);
                    write_bytes_to_memory(caller.as_context(), memory, ptr.unwrap(), e.as_bytes());
                }
                Ok(())
            })
        },
    )
}

pub(crate) fn host_error_len_func(
    store: &mut wasmtime::Store<EnvironmentState>
) -> Func {
    let callback_type = FuncType::new(vec![], vec![ValType::I32]);
    Func::new_async(
        store,
        callback_type,
        |caller, _params: &[Val], results: &mut [Val]| {
            Box::new(async move {
                let host = caller.data();
                results[0] = Val::I32(match host.get_host_error() {
                    Some(ref e) => e.len() as _,
                    None => 0,
                });
                Ok(())
            })
        },
    )
}

fn get_caller_memory<T>(caller: &mut Caller<T>) -> Memory {
    let memory = caller
        .get_export("memory")
        .map(|e| e.into_memory().unwrap());
    memory.unwrap()
}

fn get_vec_from_memory<'a, T: 'a>(
    store: impl Into<StoreContext<'a, T>>,
    mem: Memory,
    ptr: i32,
    len: i32,
) -> Vec<u8> {
    let data = mem.data(store);
    data[ptr as usize..(ptr + len) as usize].to_vec()
}

fn write_bytes_to_memory(store: impl AsContext, memory: Memory, ptr: i32, slice: &[u8]) {
    #[allow(unsafe_code)]
    unsafe {
        let raw = memory.data_ptr(store).offset(ptr as isize);
        raw.copy_from(slice.as_ptr(), slice.len());
    }
}
