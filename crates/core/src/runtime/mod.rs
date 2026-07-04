use crate::prelude::*;
use boa_engine::{Context, JsValue, Source, value::TryFromJs};

pub struct Runtime {
    context: Context,
}

impl Runtime {
    /// Creates a new JavaScript runtime.
    pub fn new() -> Self {
        Self {
            context: Context::default(),
        }
    }

    /// Evaluates JavaScript and returns the result as a string.
    pub fn eval(&mut self, code: &str) -> Result<String> {
        let value = self
            .context
            .eval(Source::from_bytes(code))
            .map_err(|e| e.to_string())?;

        Ok(js_to_string(&value, &mut self.context).map_err(|e| e.to_string())?)
    }

    /// Evaluates JavaScript and converts the result to a Rust type.
    pub fn eval_json<T>(&mut self, code: &str) -> Result<T>
    where
        T: TryFromJs,
    {
        let value = self
            .context
            .eval(Source::from_bytes(code))
            .map_err(|e| e.to_string())?;

        Ok(T::try_from_js(&value, &mut self.context).map_err(|e| e.to_string())?)
    }

    /// Clears the runtime by creating a fresh Context.
    pub fn reset(&mut self) {
        self.context = Context::default();
    }
}

fn js_to_string(value: &JsValue, ctx: &mut Context) -> StdResult<String, boa_engine::JsError> {
    if value.is_null() {
        return Ok("null".into());
    }

    if value.is_undefined() {
        return Ok("undefined".into());
    }

    if let Some(s) = value.as_string() {
        return Ok(s.to_std_string_escaped());
    }

    if value.is_object() {
        ctx.register_global_property(
            boa_engine::js_string!("__ovsy_value__"),
            value.clone(),
            boa_engine::property::Attribute::all(),
        )?;

        let json = ctx.eval(Source::from_bytes("JSON.stringify(__ovsy_value__)"))?;

        return Ok(json.to_string(ctx)?.to_std_string_escaped());
    }

    Ok(value.to_string(ctx)?.to_std_string_escaped())
}
