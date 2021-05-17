#[cfg(not(feature = "as-library"))]
#[macro_use]
extern crate redis_module;

use redis_module::native_types::RedisType;
use redis_module::raw::RedisModuleTypeMethods;
#[cfg(not(feature = "as-library"))]
use redis_module::{Context, NextArg, NotifyEvent, RedisError, RedisResult, RedisValue};
#[cfg(not(feature = "as-library"))]
use serde_json::{Value};
#[cfg(not(feature = "as-library"))]
use std::{i64, usize};

mod array_index;
mod backward;
pub mod commands;
pub mod error;
mod formatter;
pub mod manager;
mod nodevisitor;
pub mod redisjson;

#[cfg(not(feature = "as-library"))]
use crate::array_index::ArrayIndex;
#[cfg(not(feature = "as-library"))]
use crate::error::Error;
use crate::redisjson::Format;
#[cfg(not(feature = "as-library"))]
use crate::redisjson::Path;
#[cfg(not(feature = "as-library"))]
use crate::redisjson::RedisJSON;

#[cfg(not(feature = "as-library"))]
const JSON_ROOT_PATH: &'static str = "$";
pub const REDIS_JSON_TYPE_VERSION: i32 = 3;

pub static REDIS_JSON_TYPE: RedisType = RedisType::new(
    "ReJSON-RL",
    REDIS_JSON_TYPE_VERSION,
    RedisModuleTypeMethods {
        version: redis_module::TYPE_METHOD_VERSION,

        rdb_load: Some(redisjson::type_methods::rdb_load),
        rdb_save: Some(redisjson::type_methods::rdb_save),
        aof_rewrite: None, // TODO add support
        free: Some(redisjson::type_methods::free),

        // Currently unused by Redis
        mem_usage: None,
        digest: None,

        // Auxiliary data (v2)
        aux_load: Some(redisjson::type_methods::aux_load),
        aux_save: None,
        aux_save_triggers: 0,

        free_effort: None,
        unlink: None,
        copy: None,
        defrag: None,
    },
);

///
/// Backwards compatibility convertor for RedisJSON 1.x clients
///
pub fn backwards_compat_path(mut path: String) -> String {
    if !path.starts_with('$') {
        if path == "." {
            path.replace_range(..1, "$");
        } else if path.starts_with('.') {
            path.insert(0, '$');
        } else {
            path.insert_str(0, "$.");
        }
    }
    path
}

///
/// JSON.DEL <key> [path]
///
#[cfg(not(feature = "as-library"))]
fn json_del(ctx: &Context, args: Vec<String>) -> RedisResult {
    commands::command_json_del(manager::RedisJsonKeyManager, ctx, args)
}

///
/// JSON.GET <key>
///         [INDENT indentation-string]
///         [NEWLINE line-break-string]
///         [SPACE space-string]
///         [path ...]
///
/// TODO add support for multi path
#[cfg(not(feature = "as-library"))]
fn json_get(ctx: &Context, args: Vec<String>) -> RedisResult {
    commands::command_json_get(manager::RedisJsonKeyManager, ctx, args)
}

///
/// JSON.SET <key> <path> <json> [NX | XX | FORMAT <format>]
///
#[cfg(not(feature = "as-library"))]
fn json_set(ctx: &Context, args: Vec<String>) -> RedisResult {
    commands::command_json_set(manager::RedisJsonKeyManager, ctx, args)
}

///
/// JSON.MGET <key> [key ...] <path>
///
#[cfg(not(feature = "as-library"))]
fn json_mget(ctx: &Context, args: Vec<String>) -> RedisResult {
    commands::command_json_mget(manager::RedisJsonKeyManager, ctx, args)
}

///
/// JSON.STRLEN <key> [path]
///
#[cfg(not(feature = "as-library"))]
fn json_str_len(ctx: &Context, args: Vec<String>) -> RedisResult {
    json_len(ctx, args, |doc, path| doc.str_len(path))
}

///
/// JSON.TYPE <key> [path]
///
#[cfg(not(feature = "as-library"))]
fn json_type(ctx: &Context, args: Vec<String>) -> RedisResult {
    commands::command_json_type(manager::RedisJsonKeyManager, ctx, args)
}

///
/// JSON.NUMINCRBY <key> <path> <number>
///
#[cfg(not(feature = "as-library"))]
fn json_num_incrby(ctx: &Context, args: Vec<String>) -> RedisResult {
    commands::command_json_num_incrby(manager::RedisJsonKeyManager, ctx, args)
}

///
/// JSON.NUMMULTBY <key> <path> <number>
///
#[cfg(not(feature = "as-library"))]
fn json_num_multby(ctx: &Context, args: Vec<String>) -> RedisResult {
    commands::command_json_num_multby(manager::RedisJsonKeyManager, ctx, args)
}

///
/// JSON.NUMPOWBY <key> <path> <number>
///
#[cfg(not(feature = "as-library"))]
fn json_num_powby(ctx: &Context, args: Vec<String>) -> RedisResult {
    commands::command_json_num_powby(manager::RedisJsonKeyManager, ctx, args)
}
//
/// JSON.TOGGLE <key> <path>
#[cfg(not(feature = "as-library"))]
fn json_bool_toggle(ctx: &Context, args: Vec<String>) -> RedisResult {
    commands::command_json_bool_toggle(manager::RedisJsonKeyManager, ctx, args)
}

#[cfg(not(feature = "as-library"))]
fn err_json(value: &Value, expected_value: &'static str) -> Error {
    Error::from(format!(
        "ERR wrong type of path value - expected {} but found {}",
        expected_value,
        RedisJSON::value_name(value)
    ))
}

///
/// JSON.STRAPPEND <key> [path] <json-string>
///
#[cfg(not(feature = "as-library"))]
fn json_str_append(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);

    let key = args.next_string()?;
    let path_or_json = args.next_string()?;

    let path;
    let json;

    // path is optional
    if let Ok(val) = args.next_string() {
        path = backwards_compat_path(path_or_json);
        json = val;
    } else {
        path = JSON_ROOT_PATH.to_string();
        json = path_or_json;
    }

    let redis_key = ctx.open_key_writable(&key);

    redis_key
        .get_value::<RedisJSON>(&REDIS_JSON_TYPE)?
        .ok_or_else(RedisError::nonexistent_key)
        .and_then(|doc| {
            doc.value_op(
                &path,
                |value| do_json_str_append(&json, value),
                |result| {
                    ctx.notify_keyspace_event(NotifyEvent::MODULE, "json_strappend", key.as_str());
                    ctx.replicate_verbatim();
                    Ok(result
                        .as_str()
                        .map_or(usize::MAX, |result| result.len())
                        .into())
                },
            )
            .map_err(|e| e.into())
        })
}

#[cfg(not(feature = "as-library"))]
fn do_json_str_append(json: &str, value: &Value) -> Result<Value, Error> {
    value
        .as_str()
        .ok_or_else(|| err_json(value, "string"))
        .and_then(|curr| {
            let v = serde_json::from_str(json)?;
            if let Value::String(s) = v {
                let new_value = [curr, s.as_str()].concat();
                Ok(Value::String(new_value))
            } else {
                Err(format!("ERR wrong type of value - expected string but found {}", v).into())
            }
        })
}

///
/// JSON.ARRAPPEND <key> <path> <json> [json ...]
///
#[cfg(not(feature = "as-library"))]
fn json_arr_append(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1).peekable();

    let key = args.next_string()?;
    let path = backwards_compat_path(args.next_string()?);

    // We require at least one JSON item to append
    args.peek().ok_or(RedisError::WrongArity)?;

    let redis_key = ctx.open_key_writable(&key);

    redis_key
        .get_value::<RedisJSON>(&REDIS_JSON_TYPE)?
        .ok_or_else(RedisError::nonexistent_key)
        .and_then(|doc| {
            doc.value_op(
                &path,
                |value| do_json_arr_append(args.clone(), value),
                |result| {
                    ctx.notify_keyspace_event(NotifyEvent::MODULE, "json_arrappend", key.as_str());
                    ctx.replicate_verbatim();
                    Ok(result
                        .as_array()
                        .map_or(usize::MAX, |result| result.len())
                        .into())
                },
            )
            .map_err(|e| e.into())
        })
}

#[cfg(not(feature = "as-library"))]
fn do_json_arr_append<I>(args: I, value: &mut Value) -> Result<Value, Error>
where
    I: Iterator<Item = String>,
{
    if !value.is_array() {
        return Err(err_json(value, "array"));
    }

    let mut items: Vec<Value> = args
        .map(|json| serde_json::from_str(&json))
        .collect::<Result<_, _>>()?;

    let mut new_value = value.take();
    new_value.as_array_mut().unwrap().append(&mut items);
    Ok(new_value)
}

///
/// JSON.ARRINDEX <key> <path> <json-scalar> [start [stop]]
///
/// scalar - number, string, Boolean (true or false), or null
///
#[cfg(not(feature = "as-library"))]
fn json_arr_index(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);

    let key = args.next_string()?;
    let path = backwards_compat_path(args.next_string()?);
    let json_scalar = args.next_string()?;
    let start: i64 = args.next().map(|v| v.parse()).unwrap_or(Ok(0))?;
    let end: i64 = args.next().map(|v| v.parse()).unwrap_or(Ok(0))?;

    args.done()?; // TODO: Add to other functions as well to terminate args list

    let key = ctx.open_key(&key);

    let index = key
        .get_value::<RedisJSON>(&REDIS_JSON_TYPE)?
        .map_or(Ok(-1), |doc| doc.arr_index(&path, &json_scalar, start, end))?;

    Ok(index.into())
}

///
/// JSON.ARRINSERT <key> <path> <index> <json> [json ...]
///
#[cfg(not(feature = "as-library"))]
fn json_arr_insert(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1).peekable();

    let key = args.next_string()?;
    let path = backwards_compat_path(args.next_string()?);
    let index = args.next_i64()?;

    // We require at least one JSON item to append
    args.peek().ok_or(RedisError::WrongArity)?;

    let redis_key = ctx.open_key_writable(&key);

    redis_key
        .get_value::<RedisJSON>(&REDIS_JSON_TYPE)?
        .ok_or_else(RedisError::nonexistent_key)
        .and_then(|doc| {
            doc.value_op(
                &path,
                |value| do_json_arr_insert(args.clone(), index, value),
                |result| {
                    ctx.notify_keyspace_event(NotifyEvent::MODULE, "json_arrinsert", key.as_str());
                    ctx.replicate_verbatim();
                    Ok(result
                        .as_array()
                        .map_or(usize::MAX, |result| result.len())
                        .into())
                },
            )
            .map_err(|e| e.into())
        })
}

#[cfg(not(feature = "as-library"))]
fn do_json_arr_insert<I>(args: I, index: i64, value: &mut Value) -> Result<Value, Error>
where
    I: Iterator<Item = String>,
{
    if let Some(array) = value.as_array() {
        // Verify legal index in bounds
        let len = array.len() as i64;
        let index = if index < 0 { len + index } else { index };
        if !(0..=len).contains(&index) {
            return Err("ERR index out of bounds".into());
        }
        let index = index as usize;

        let items: Vec<Value> = args
            .map(|json| serde_json::from_str(&json))
            .collect::<Result<_, _>>()?;
        let mut new_value = value.take();
        let curr = new_value.as_array_mut().unwrap();
        curr.splice(index..index, items.into_iter());
        Ok(new_value)
    } else {
        Err(err_json(value, "array"))
    }
}

///
/// JSON.ARRLEN <key> [path]
///
#[cfg(not(feature = "as-library"))]
fn json_arr_len(ctx: &Context, args: Vec<String>) -> RedisResult {
    json_len(ctx, args, |doc, path| doc.arr_len(path))
}

///
/// JSON.ARRPOP <key> [path [index]]
///
#[cfg(not(feature = "as-library"))]
fn json_arr_pop(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);

    let key = args.next_string()?;

    let (path, index) = args
        .next()
        .map(|p| {
            let path = backwards_compat_path(p);
            let index = args.next_i64().unwrap_or(-1);
            (path, index)
        })
        .unwrap_or((JSON_ROOT_PATH.to_string(), i64::MAX));

    let redis_key = ctx.open_key_writable(&key);
    let mut res = None;

    redis_key
        .get_value::<RedisJSON>(&REDIS_JSON_TYPE)?
        .ok_or_else(RedisError::nonexistent_key)
        .and_then(|doc| {
            doc.value_op(
                &path,
                |value| do_json_arr_pop(index, &mut res, value),
                |_result| {
                    ctx.notify_keyspace_event(NotifyEvent::MODULE, "json_arrpop", key.as_str());
                    ctx.replicate_verbatim();
                    Ok(()) // fake result doesn't use it uses `res` instead
                },
            )
            .map_err(|e| e.into())
        })?;

    let result = match res {
        None => ().into(),
        Some(r) => RedisJSON::serialize(&r, Format::JSON)?.into(),
    };
    Ok(result)
}

#[cfg(not(feature = "as-library"))]
fn do_json_arr_pop(index: i64, res: &mut Option<Value>, value: &mut Value) -> Result<Value, Error> {
    if let Some(array) = value.as_array() {
        if array.is_empty() {
            *res = None;
            return Ok(value.clone());
        }

        // Verify legel index in bounds
        let len = array.len() as i64;
        let index = if index < 0 {
            0.max(len + index)
        } else {
            index.min(len - 1)
        } as usize;

        let mut new_value = value.take();
        let curr = new_value.as_array_mut().unwrap();
        *res = Some(curr.remove(index as usize));
        Ok(new_value)
    } else {
        Err(err_json(value, "array"))
    }
}

///
/// JSON.ARRTRIM <key> <path> <start> <stop>
///
#[cfg(not(feature = "as-library"))]
fn json_arr_trim(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);

    let key = args.next_string()?;
    let path = backwards_compat_path(args.next_string()?);
    let start = args.next_i64()?;
    let stop = args.next_i64()?;

    let redis_key = ctx.open_key_writable(&key);

    redis_key
        .get_value::<RedisJSON>(&REDIS_JSON_TYPE)?
        .ok_or_else(RedisError::nonexistent_key)
        .and_then(|doc| {
            doc.value_op(
                &path,
                |value| do_json_arr_trim(start, stop, value),
                |result| {
                    ctx.notify_keyspace_event(NotifyEvent::MODULE, "json_arrtrim", key.as_str());
                    ctx.replicate_verbatim();
                    Ok(result
                        .as_array()
                        .map_or(usize::MAX, |result| result.len())
                        .into())
                },
            )
            .map_err(|e| e.into())
        })
}

#[cfg(not(feature = "as-library"))]
fn do_json_arr_trim(start: i64, stop: i64, value: &mut Value) -> Result<Value, Error> {
    if let Some(array) = value.as_array() {
        let len = array.len() as i64;
        let stop = stop.normalize(len);

        let range = if start > len || start > stop as i64 {
            0..0 // Return an empty array
        } else {
            start.normalize(len)..(stop + 1)
        };

        let mut new_value = value.take();
        let curr = new_value.as_array_mut().unwrap();
        curr.rotate_left(range.start);
        curr.resize(range.end - range.start, Value::Null);

        Ok(new_value)
    } else {
        Err(err_json(value, "array"))
    }
}

///
/// JSON.OBJKEYS <key> [path]
///
#[cfg(not(feature = "as-library"))]
fn json_obj_keys(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    let path = backwards_compat_path(args.next_string()?);

    let key = ctx.open_key(&key);

    let value = match key.get_value::<RedisJSON>(&REDIS_JSON_TYPE)? {
        Some(doc) => doc.obj_keys(&path)?.into(),
        None => RedisValue::Null,
    };

    Ok(value)
}

///
/// JSON.OBJLEN <key> [path]
///
#[cfg(not(feature = "as-library"))]
fn json_obj_len(ctx: &Context, args: Vec<String>) -> RedisResult {
    json_len(ctx, args, |doc, path| doc.obj_len(path))
}

///
/// JSON.CLEAR <key> [path ...]
///
#[cfg(not(feature = "as-library"))]
fn json_clear(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    let paths = args.map(Path::new).collect::<Vec<_>>();

    let paths = if paths.is_empty() {
        vec![Path::new(JSON_ROOT_PATH.to_string())]
    } else {
        paths
    };

    // FIXME: handle multi paths
    let key = ctx.open_key_writable(&key);
    let deleted = match key.get_value::<RedisJSON>(&REDIS_JSON_TYPE)? {
        Some(doc) => {
            let res = doc.clear(paths.first().unwrap().fixed.as_str())?;
            ctx.replicate_verbatim();
            res
        }
        None => 0,
    };
    Ok(deleted.into())
}
///
/// JSON.DEBUG <subcommand & arguments>
///
/// subcommands:
/// MEMORY <key> [path]
/// HELP
///
#[cfg(not(feature = "as-library"))]
fn json_debug(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    match args.next_string()?.to_uppercase().as_str() {
        "MEMORY" => {
            let key = args.next_string()?;
            let path = backwards_compat_path(args.next_string()?);

            let key = ctx.open_key(&key);
            let value = match key.get_value::<RedisJSON>(&REDIS_JSON_TYPE)? {
                Some(doc) => doc.get_memory(&path)?,
                None => 0,
            };
            Ok(value.into())
        }
        "HELP" => {
            let results = vec![
                "MEMORY <key> [path] - reports memory usage",
                "HELP                - this message",
            ];
            Ok(results.into())
        }
        _ => Err(RedisError::Str(
            "ERR unknown subcommand - try `JSON.DEBUG HELP`",
        )),
    }
}

///
/// JSON.RESP <key> [path]
///
#[cfg(not(feature = "as-library"))]
fn json_resp(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);

    let key = args.next_string()?;
    let path = args
        .next_string()
        .map_or_else(|_| JSON_ROOT_PATH.to_string(), |v| backwards_compat_path(v));

    let key = ctx.open_key(&key);
    match key.get_value::<RedisJSON>(&REDIS_JSON_TYPE)? {
        Some(doc) => Ok(resp_serialize(doc.get_first(&path)?)),
        None => Ok(RedisValue::Null),
    }
}

#[cfg(not(feature = "as-library"))]
fn resp_serialize(doc: &Value) -> RedisValue {
    match doc {
        Value::Null => RedisValue::Null,

        Value::Bool(b) => RedisValue::SimpleString(b.to_string()),

        Value::Number(n) => n
            .as_i64()
            .map(RedisValue::Integer)
            .unwrap_or_else(|| RedisValue::Float(n.as_f64().unwrap())),

        Value::String(s) => RedisValue::BulkString(s.clone()),

        Value::Array(arr) => {
            let mut res: Vec<RedisValue> = Vec::with_capacity(arr.len() + 1);
            res.push(RedisValue::SimpleStringStatic("["));
            arr.iter().for_each(|v| res.push(resp_serialize(v)));
            RedisValue::Array(res)
        }

        Value::Object(obj) => {
            let mut res: Vec<RedisValue> = Vec::with_capacity(obj.len() + 1);
            res.push(RedisValue::SimpleStringStatic("{"));
            for (key, value) in obj.iter() {
                res.push(RedisValue::BulkString(key.to_string()));
                res.push(resp_serialize(value));
            }
            RedisValue::Array(res)
        }
    }
}

#[cfg(not(feature = "as-library"))]
fn json_len<F: Fn(&RedisJSON, &String) -> Result<usize, Error>>(
    ctx: &Context,
    args: Vec<String>,
    fun: F,
) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    let path = backwards_compat_path(args.next_string()?);

    let key = ctx.open_key(&key);
    let length = match key.get_value::<RedisJSON>(&REDIS_JSON_TYPE)? {
        Some(doc) => fun(&doc, &path)?.into(),
        None => RedisValue::Null,
    };

    Ok(length)
}

#[cfg(not(feature = "as-library"))]
fn json_cache_info(_ctx: &Context, _args: Vec<String>) -> RedisResult {
    Err(RedisError::Str("Command was not implemented"))
}

#[cfg(not(feature = "as-library"))]
fn json_cache_init(_ctx: &Context, _args: Vec<String>) -> RedisResult {
    Err(RedisError::Str("Command was not implemented"))
}
//////////////////////////////////////////////////////

#[cfg(not(feature = "as-library"))]
redis_module! {
    name: "ReJSON",
    version: 99_99_99,
    data_types: [
        REDIS_JSON_TYPE,
    ],
    commands: [
        ["json.del", json_del, "write", 1,1,1],
        ["json.get", json_get, "readonly", 1,1,1],
        ["json.mget", json_mget, "readonly", 1,1,1],
        ["json.set", json_set, "write deny-oom", 1,1,1],
        ["json.type", json_type, "readonly", 1,1,1],
        ["json.numincrby", json_num_incrby, "write", 1,1,1],
        ["json.toggle", json_bool_toggle, "write deny-oom", 1,1,1],
        ["json.nummultby", json_num_multby, "write", 1,1,1],
        ["json.numpowby", json_num_powby, "write", 1,1,1],
        ["json.strappend", json_str_append, "write deny-oom", 1,1,1],
        ["json.strlen", json_str_len, "readonly", 1,1,1],
        ["json.arrappend", json_arr_append, "write deny-oom", 1,1,1],
        ["json.arrindex", json_arr_index, "readonly", 1,1,1],
        ["json.arrinsert", json_arr_insert, "write deny-oom", 1,1,1],
        ["json.arrlen", json_arr_len, "readonly", 1,1,1],
        ["json.arrpop", json_arr_pop, "write", 1,1,1],
        ["json.arrtrim", json_arr_trim, "write", 1,1,1],
        ["json.objkeys", json_obj_keys, "readonly", 1,1,1],
        ["json.objlen", json_obj_len, "readonly", 1,1,1],
        ["json.clear", json_clear, "write", 1,1,1],
        ["json.debug", json_debug, "readonly", 1,1,1],
        ["json.forget", json_del, "write", 1,1,1],
        ["json.resp", json_resp, "readonly", 1,1,1],
        ["json._cacheinfo", json_cache_info, "readonly", 1,1,1],
        ["json._cacheinit", json_cache_init, "write", 1,1,1],
    ],
}
